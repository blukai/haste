use crate::{
    fieldmetadata::{self, get_field_metadata, FieldMetadata, FieldSpecialDescriptor},
    fxhash,
    protos::{
        prost::{self, Message},
        CDemoSendTables, CsvcMsgFlattenedSerializer, ProtoFlattenedSerializerFieldT,
        ProtoFlattenedSerializerT,
    },
    varint, vartype,
};
use hashbrown::{hash_map::Values, HashMap};
use nohash::NoHashHasher;
use std::{hash::BuildHasherDefault, rc::Rc};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // 3rd party crates
    #[error(transparent)]
    Prost(#[from] prost::DecodeError),
    // crate
    #[error(transparent)]
    Varint(#[from] varint::Error),
    #[error(transparent)]
    FieldMetadata(#[from] fieldmetadata::Error),
    #[error(transparent)]
    Vartype(#[from] vartype::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

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

#[derive(Debug)]
pub struct FlattenedSerializerContext {
    pub tick_interval: f32,
}

// some info about string tables
// https://developer.valvesoftware.com/wiki/Networking_Events_%26_Messages
// https://developer.valvesoftware.com/wiki/Networking_Entities

// TODO: merge string + hash into a single struct or something
//
// TODO: do not clone strings, but reference them instead -> introduce lifetimes
// or build a symbol table from symbols (string cache?)

#[derive(Debug, Clone, Default)]
pub struct FlattenedSerializerField {
    pub var_type: Symbol,
    pub var_name: Symbol,

    pub bit_count: Option<i32>,
    pub low_value: Option<f32>,
    pub high_value: Option<f32>,
    pub encode_flags: Option<i32>,

    pub field_serializer_name: Option<Symbol>,
    pub field_serializer: Option<Rc<FlattenedSerializer>>,

    // NOTE: field_serializer_version and send_node are not being used anywhere
    // (obviously duh).
    //
    // pub field_serializer_version: Option<i32>, pub
    // send_node: Option<Box<str>>,
    //
    pub var_encoder: Option<Symbol>,

    pub metadata: FieldMetadata,
}

impl FlattenedSerializerField {
    fn new(
        msg: &CsvcMsgFlattenedSerializer,
        field: &ProtoFlattenedSerializerFieldT,
        ctx: &FlattenedSerializerContext,
    ) -> Result<Self> {
        // SAFETY: some symbols are cricual, if they don't exist - fail early
        // and loudly.
        let resolve_sym_unchecked = |i: i32| unsafe { msg.symbols.get_unchecked(i as usize) };
        let resolve_sym = |v: i32| &msg.symbols[v as usize];

        let var_type = unsafe {
            field
                .var_type_sym
                .map(resolve_sym_unchecked)
                .unwrap_unchecked()
        };
        let var_type_expr = vartype::parse(var_type.as_str())?;

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
            field_serializer: None,

            var_encoder: field.var_encoder_sym.map(resolve_sym).map(Symbol::from),

            metadata: Default::default(),
        };
        ret.metadata = get_field_metadata(var_type_expr, &ret, ctx)?;
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
    pub(crate) fn is_var_encoder_hash_eq(&self, rhs: u64) -> bool {
        self.var_encoder.as_ref().is_some_and(|lhs| lhs.hash == rhs)
    }

    // TODO(blukai): rename is_vector to is_dynamic_array or something
    #[inline(always)]
    pub fn is_vector(&self) -> bool {
        self.metadata
            .special_descriptor
            .as_ref()
            .is_some_and(|sd| sd.is_vector())
    }
}

// NOTE: Clone derive is needed here because Entity in entities.rs needs to be
// clonable which means that all members of it also should be clonable.
#[derive(Debug, Clone, Default)]
pub struct FlattenedSerializer {
    pub serializer_name: Symbol,

    // TODO: figure out serializer version, is it needed?
    pub serializer_version: Option<i32>,
    pub fields: Vec<Rc<FlattenedSerializerField>>,
}

impl FlattenedSerializer {
    fn new(msg: &CsvcMsgFlattenedSerializer, fs: &ProtoFlattenedSerializerT) -> Result<Self> {
        // SAFETY: some symbols are cricual, if they don't exist - fail early
        // and loudly.
        let resolve_sym_unchecked = |i: i32| unsafe { msg.symbols.get_unchecked(i as usize) };

        let serializer_name = unsafe {
            fs.serializer_name_sym
                .map(resolve_sym_unchecked)
                .unwrap_unchecked()
        };

        Ok(Self {
            serializer_name: Symbol::from(serializer_name),

            serializer_version: fs.serializer_version,
            fields: Vec::with_capacity(fs.fields_index.len()),
        })
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
    pub fn parse(cmd: CDemoSendTables, ctx: FlattenedSerializerContext) -> Result<Self> {
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
            let (_size, _count) = varint::read_uvarint32(&mut data)?;
            CsvcMsgFlattenedSerializer::decode(data)?
        };

        let mut fields: FieldMap =
            FieldMap::with_capacity_and_hasher(msg.fields.len(), BuildHasherDefault::default());
        let mut serializer_map: SerializerMap = SerializerMap::with_capacity_and_hasher(
            msg.serializers.len(),
            BuildHasherDefault::default(),
        );

        // TODO: can fields be stored flatly?

        for serializer in msg.serializers.iter() {
            let mut flattened_serializer = FlattenedSerializer::new(&msg, serializer)?;

            for field_index in serializer.fields_index.iter() {
                let field = if fields.contains_key(field_index) {
                    // SAFETY: we already know that hashmap has the key!
                    let field = unsafe { fields.get(field_index).unwrap_unchecked() };
                    // NOTE: it is more efficient to clone outside instead of
                    // using .clonned() because we're doing unsafe unwrap which
                    // removes the branch, but .clonned() uses match under the
                    // hood which adds a branch!
                    Rc::clone(field)
                } else {
                    let mut field = FlattenedSerializerField::new(
                        &msg,
                        &msg.fields[*field_index as usize],
                        &ctx,
                    )?;

                    if let Some(field_serializer_name) = field.field_serializer_name.as_ref() {
                        field.field_serializer =
                            serializer_map.get(&field_serializer_name.hash).cloned();
                    }

                    // TODO: maybe extract arms into separate functions
                    match field.metadata.special_descriptor {
                        Some(FieldSpecialDescriptor::FixedArray { length }) => {
                            field.field_serializer = Some(Rc::new(FlattenedSerializer {
                                fields: {
                                    let mut fields = Vec::with_capacity(length);
                                    fields.resize(length, Rc::new(field.clone()));
                                    fields
                                },
                                ..Default::default()
                            }));
                        }
                        Some(FieldSpecialDescriptor::DynamicArray { ref decoder }) => {
                            field.field_serializer = Some(Rc::new(FlattenedSerializer {
                                fields: vec![Rc::new(FlattenedSerializerField {
                                    metadata: FieldMetadata {
                                        decoder: decoder.clone(),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })],
                                ..Default::default()
                            }));
                        }
                        Some(FieldSpecialDescriptor::DynamicSerializerVector) => {
                            field.field_serializer = Some(Rc::new(FlattenedSerializer {
                                fields: vec![Rc::new(FlattenedSerializerField {
                                    field_serializer: field
                                        .field_serializer_name
                                        .as_ref()
                                        .and_then(|field_serializer_name| {
                                            serializer_map.get(&field_serializer_name.hash)
                                        })
                                        .cloned(),
                                    ..Default::default()
                                })],
                                ..Default::default()
                            }));
                        }
                        _ => {}
                    }

                    let field = Rc::new(field);
                    let ret = Rc::clone(&field);
                    fields.insert(*field_index, field);
                    ret
                };
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
            .cloned()
            .unwrap_unchecked()
    }

    #[inline]
    pub fn values(&self) -> Values<'_, u64, Rc<FlattenedSerializer>> {
        self.serializer_map.values()
    }
}
