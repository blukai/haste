use std::hash::BuildHasherDefault;
use std::rc::Rc;

use dungers::varint;
use hashbrown::hash_map::Values;
use hashbrown::HashMap;
use nohash::NoHashHasher;
use prost::{self, Message};
use valveprotos::common::{
    CDemoSendTables, CsvcMsgFlattenedSerializer, ProtoFlattenedSerializerFieldT,
    ProtoFlattenedSerializerT,
};

use crate::fieldmetadata::{
    get_field_metadata, FieldMetadata, FieldMetadataError, FieldSpecialDescriptor,
};
use crate::fxhash;

#[derive(thiserror::Error, Debug)]
pub enum FlattenedSerializersError {
    #[error(transparent)]
    DecodeError(#[from] prost::DecodeError),
    #[error(transparent)]
    ReadVarintError(#[from] varint::ReadVarintError),
    #[error(transparent)]
    FieldMetadataError(#[from] FieldMetadataError),
}

// TODO: symbol table / string cache (but do not use servo's string cache
// because it's super slow; it relies on rust-phf that uses sip13 cryptograpgic
// hasher, and you can't replace it with something else (without forking it
// really)).
//
// valve's implementation: public/tier1/utlsymbol.h;
// blukai's 3head implementation:
#[derive(Debug, Clone, Default)]
pub struct Symbol {
    pub hash: u64,
    // TODO: consider renaming str into boxed_str
    #[cfg(feature = "preserve-metadata")]
    pub str: Box<str>,
}

impl From<&String> for Symbol {
    #[inline(always)]
    fn from(value: &String) -> Self {
        Self {
            hash: fxhash::hash_bytes(value.as_bytes()),
            #[cfg(feature = "preserve-metadata")]
            str: value.clone().into_boxed_str(),
        }
    }
}

// some info about string tables
// https://developer.valvesoftware.com/wiki/Networking_Events_%26_Messages
// https://developer.valvesoftware.com/wiki/Networking_Entities

// TODO: merge string + hash into a single struct or something
//
// TODO: do not clone strings, but reference them instead -> introduce lifetimes
// or build a symbol table from symbols (string cache?)

/// note about missing `field_serializer_version` field (from
/// [`valveprotos::common::ProtoFlattenedSerializerFieldT`]): i did not find any evidence of it
/// being used nor any breakage or data corruptions. it is possible that i missed something. but
/// unless proven otherwise i don't see a reason for incorporating it. field serializers always
/// reference "highest" version of serializer.
///
/// it might be reasonable to actually incorporate it if there will be a need to run this parser in
/// an environment that processes high volumes of replays. theoretically flattened serializers can
/// be parsed once and then reused for future parse passes.
#[derive(Debug, Clone, Default)]
pub struct FlattenedSerializerField {
    pub var_type: Symbol,
    pub var_name: Symbol,
    pub bit_count: Option<i32>,
    pub low_value: Option<f32>,
    pub high_value: Option<f32>,
    pub encode_flags: Option<i32>,
    pub field_serializer_name: Option<Symbol>,
    pub var_encoder: Option<Symbol>,

    pub field_serializer: Option<Rc<FlattenedSerializer>>,
    pub(crate) metadata: FieldMetadata,
}

// TODO: try to split flattened serializer field initialization into 3 clearly separate stages
// (protobuf mapping; metadata; field serializer construction).
impl FlattenedSerializerField {
    fn new(
        msg: &CsvcMsgFlattenedSerializer,
        field: &ProtoFlattenedSerializerFieldT,
    ) -> Result<Self, FieldMetadataError> {
        // SAFETY: some symbols are cricual, if they don't exist - fail early
        // and loudly.
        //
        // TODO: do not call get_unchecked here! that's stupid.
        let resolve_sym_unchecked = |i: i32| unsafe { msg.symbols.get_unchecked(i as usize) };
        let resolve_sym = |v: i32| &msg.symbols[v as usize];

        let var_type = unsafe {
            field
                .var_type_sym
                .map(resolve_sym_unchecked)
                .unwrap_unchecked()
        };
        let var_name = unsafe {
            field
                .var_name_sym
                .map(resolve_sym_unchecked)
                .unwrap_unchecked()
        };

        let mut ret = Self {
            var_type: Symbol::from(var_type),
            var_name: Symbol::from(var_name),
            bit_count: field.bit_count,
            low_value: field.low_value,
            high_value: field.high_value,
            encode_flags: field.encode_flags,
            field_serializer_name: field
                .field_serializer_name_sym
                .map(resolve_sym)
                .map(Symbol::from),
            var_encoder: field.var_encoder_sym.map(resolve_sym).map(Symbol::from),

            field_serializer: None,
            metadata: Default::default(),
        };
        ret.metadata = get_field_metadata(&ret, var_type)?;
        Ok(ret)
    }

    #[inline(always)]
    pub(crate) unsafe fn get_child_unchecked(&self, index: usize) -> &Self {
        let fs = self.field_serializer.as_ref();

        debug_assert!(fs.is_some(), "field serializer is missing");

        fs.unwrap_unchecked().get_child_unchecked(index)
    }

    // NOTE: using this method can hurt performance when used in critical code
    // paths. use the unsafe [`Self::get_child_unchecked`] instead.
    pub fn get_child(&self, index: usize) -> Option<&Self> {
        self.field_serializer
            .as_ref()
            .and_then(|fs| fs.get_child(index))
    }

    #[inline(always)]
    pub(crate) fn var_encoder_heq(&self, rhs: u64) -> bool {
        self.var_encoder.as_ref().is_some_and(|lhs| lhs.hash == rhs)
    }

    #[inline(always)]
    pub fn is_dynamic_array(&self) -> bool {
        self.metadata
            .special_descriptor
            .as_ref()
            .is_some_and(|sd| sd.is_dynamic_array())
    }
}

/// note about missing `serializer_version` field (from
/// [`valveprotos::common::ProtoFlattenedSerializerT`]): entities resolve their serializers by
/// looking up their class info within the [crate::entityclasses::EntityClasses] struct (which i
/// parse out of [`valveprotos::common::CDemoClassInfo`] proto).
/// [`valveprotos::common::CDemoClassInfo`] carries absolutely no info about serializer version
/// thus i don't see any need to preserve it.
//
// NOTE: Clone is derived because Entity in entities.rs needs to be clonable which means that all
// members of it also should be clonable.
//
// TODO: why does FlattnedeSerializer need to derive Default?
#[derive(Debug, Clone, Default)]
pub struct FlattenedSerializer {
    pub serializer_name: Symbol,
    pub fields: Vec<Rc<FlattenedSerializerField>>,
}

impl FlattenedSerializer {
    fn new(msg: &CsvcMsgFlattenedSerializer, fs: &ProtoFlattenedSerializerT) -> Self {
        // SAFETY: some symbols are cricual, if they don't exist - fail early
        // and loudly.
        let resolve_sym_unchecked = |i: i32| unsafe { msg.symbols.get_unchecked(i as usize) };

        let serializer_name = unsafe {
            fs.serializer_name_sym
                .map(resolve_sym_unchecked)
                .unwrap_unchecked()
        };

        Self {
            serializer_name: Symbol::from(serializer_name),
            fields: Vec::with_capacity(fs.fields_index.len()),
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn get_child_unchecked(&self, index: usize) -> &FlattenedSerializerField {
        debug_assert!(
            self.fields.get(index).is_some(),
            "field at index {} is missing",
            index,
        );

        self.fields.get_unchecked(index)
    }

    // NOTE: using this method can hurt performance when used in critical code
    // paths. use the unsafe [`Self::get_child_unchecked`] instead.
    pub fn get_child(&self, index: usize) -> Option<&FlattenedSerializerField> {
        self.fields.get(index).map(|field| field.as_ref())
    }
}

type FieldMap = HashMap<i32, Rc<FlattenedSerializerField>, BuildHasherDefault<NoHashHasher<i32>>>;
type SerializerMap = HashMap<u64, Rc<FlattenedSerializer>, BuildHasherDefault<NoHashHasher<u64>>>;

pub struct FlattenedSerializerContainer {
    serializer_map: SerializerMap,
}

impl FlattenedSerializerContainer {
    pub fn parse(cmd: CDemoSendTables) -> Result<Self, FlattenedSerializersError> {
        let msg = {
            // TODO: make prost work with ByteString and turn data into Bytes
            //
            // NOTE: calling unwrap_or_default is for some reason faster then
            // relying on prost's default unwrapping by calling .data().
            let mut data = &cmd.data.unwrap_or_default()[..];
            // NOTE: count is useless because read_uvarint32 will "consume"
            // bytes that it'll read from data; size is useless because data
            // supposedly contains only one message.
            //
            // NOTE: there are 2 levels of indirection here, but rust's compiler
            // may optimize them away. but if this will be affecting performance
            // -> createa a function that will be capable of reading varint from
            // &[u8] without multiple levels of indirection.
            let (_size, _count) = varint::read_uvarint64(&mut data)?;
            CsvcMsgFlattenedSerializer::decode(data)?
        };

        let mut field_map: FieldMap =
            FieldMap::with_capacity_and_hasher(msg.fields.len(), BuildHasherDefault::default());
        let mut serializer_map: SerializerMap = SerializerMap::with_capacity_and_hasher(
            msg.serializers.len(),
            BuildHasherDefault::default(),
        );

        for serializer in msg.serializers.iter() {
            let mut flattened_serializer = FlattenedSerializer::new(&msg, serializer);

            for field_index in serializer.fields_index.iter() {
                if let Some(field) = field_map.get(field_index) {
                    flattened_serializer.fields.push(field.clone());
                    continue;
                }

                let mut field =
                    FlattenedSerializerField::new(&msg, &msg.fields[*field_index as usize])?;

                field.field_serializer = match field.metadata.special_descriptor {
                    Some(FieldSpecialDescriptor::FixedArray { length }) => {
                        let mut field = field.clone();
                        field.field_serializer = field
                            .field_serializer_name
                            .as_ref()
                            .and_then(|symbol| serializer_map.get(&symbol.hash).cloned());
                        Some(Rc::new(FlattenedSerializer {
                            fields: {
                                let mut fields = Vec::with_capacity(length);
                                fields.resize(length, Rc::new(field));
                                fields
                            },
                            ..Default::default()
                        }))
                    }
                    Some(FieldSpecialDescriptor::DynamicArray { ref decoder }) => {
                        let field = FlattenedSerializerField {
                            metadata: FieldMetadata {
                                decoder: decoder.clone(),
                                ..Default::default()
                            },
                            ..Default::default()
                        };
                        Some(Rc::new(FlattenedSerializer {
                            fields: vec![Rc::new(field)],
                            ..Default::default()
                        }))
                    }
                    Some(FieldSpecialDescriptor::DynamicSerializerArray) => {
                        let field = FlattenedSerializerField {
                            field_serializer: field
                                .field_serializer_name
                                .as_ref()
                                .and_then(|symbol| serializer_map.get(&symbol.hash).cloned()),
                            ..Default::default()
                        };
                        Some(Rc::new(FlattenedSerializer {
                            fields: vec![Rc::new(field)],
                            ..Default::default()
                        }))
                    }
                    _ => field
                        .field_serializer_name
                        .as_ref()
                        .and_then(|symbol| serializer_map.get(&symbol.hash).cloned()),
                };

                let field = Rc::new(field);
                field_map.insert(*field_index, field.clone());
                flattened_serializer.fields.push(field);
            }

            serializer_map.insert(
                flattened_serializer.serializer_name.hash,
                Rc::new(flattened_serializer),
            );
        }

        Ok(Self { serializer_map })
    }

    // TODO: think about exposing the whole serializer map

    #[inline(always)]
    pub fn by_name_hash(&self, serializer_name_hash: u64) -> Option<Rc<FlattenedSerializer>> {
        self.serializer_map.get(&serializer_name_hash).cloned()
    }

    #[inline(always)]
    pub unsafe fn by_name_hash_unckecked(
        &self,
        serializer_name_hash: u64,
    ) -> Rc<FlattenedSerializer> {
        self.serializer_map
            .get(&serializer_name_hash)
            .unwrap_unchecked()
            // NOTE: do not chain .cloned() after calling .get(), because .cloned() uses match
            // under the hood which adds a branch; that is redunant.
            .clone()
    }

    #[inline]
    pub fn values(&self) -> Values<'_, u64, Rc<FlattenedSerializer>> {
        self.serializer_map.values()
    }
}
