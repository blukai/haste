use crate::{
    dota2_protos::{
        prost::{self, Message},
        CDemoSendTables, CsvcMsgFlattenedSerializer, ProtoFlattenedSerializerFieldT,
        ProtoFlattenedSerializerT,
    },
    fieldmetadata::{self, get_field_metadata, FieldMetadata, FieldSpecialDescriptor},
    fxhash, varint,
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
}

pub type Result<T> = std::result::Result<T, Error>;

// some info about string tables
// https://developer.valvesoftware.com/wiki/Networking_Events_%26_Messages
// https://developer.valvesoftware.com/wiki/Networking_Entities

#[derive(Debug, Clone, Default)]
pub struct FlattenedSerializerField {
    #[cfg(debug_assertions)]
    pub var_type: Box<str>,

    #[cfg(debug_assertions)]
    pub var_name: Box<str>,
    pub var_name_hash: u64,

    pub bit_count: Option<i32>,
    pub low_value: Option<f32>,
    pub high_value: Option<f32>,
    pub encode_flags: Option<i32>,

    #[cfg(debug_assertions)]
    pub field_serializer_name: Option<Box<str>>,
    pub field_serializer_name_hash: Option<u64>,
    pub field_serializer: Option<Rc<FlattenedSerializer>>,

    // NOTE: field_serializer_version and send_node are not being used anywhere
    // (obviously duh).
    //
    // pub field_serializer_version: Option<i32>, pub
    // send_node: Option<Box<str>>,
    //
    #[cfg(debug_assertions)]
    pub var_encoder: Option<Box<str>>,
    pub var_encoder_hash: Option<u64>,

    pub metadata: FieldMetadata,
}

impl FlattenedSerializerField {
    fn new(
        msg: &CsvcMsgFlattenedSerializer,
        field: &ProtoFlattenedSerializerFieldT,
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
        let var_type_decl = haste_dota2_deflat::var_type::parse(var_type.as_str());

        let var_name = unsafe {
            field
                .var_name_sym
                .map(resolve_sym_unchecked)
                .unwrap_unchecked()
        };
        let var_name_hash = fxhash::hash_u8(var_name.as_bytes());

        let field_serializer_name = field.field_serializer_name_sym.map(resolve_sym);
        let field_serializer_name_hash = field_serializer_name
            .as_ref()
            .map(|field_serializer_name| fxhash::hash_u8(field_serializer_name.as_bytes()));

        let var_encoder = field.var_encoder_sym.map(resolve_sym);
        let var_encoder_hash = var_encoder
            .as_ref()
            .map(|var_encoder| fxhash::hash_u8(var_encoder.as_bytes()));

        let mut ret = Self {
            #[cfg(debug_assertions)]
            var_type: var_type.clone().into_boxed_str(),

            #[cfg(debug_assertions)]
            var_name: var_name.clone().into_boxed_str(),
            var_name_hash,

            bit_count: field.bit_count,
            low_value: field.low_value,
            high_value: field.high_value,
            encode_flags: field.encode_flags,

            #[cfg(debug_assertions)]
            field_serializer_name: field_serializer_name.map(|s| s.clone().into_boxed_str()),
            field_serializer_name_hash,
            field_serializer: None,

            #[cfg(debug_assertions)]
            var_encoder: var_encoder.map(|s| s.clone().into_boxed_str()),
            var_encoder_hash,

            metadata: Default::default(),
        };
        ret.metadata = get_field_metadata(var_type_decl, &ret)?;
        Ok(ret)
    }

    #[inline(always)]
    pub unsafe fn get_child_unchecked(&self, index: usize) -> &Self {
        let fs = self.field_serializer.as_ref();

        #[cfg(debug_assertions)]
        debug_assert!(
            fs.is_some(),
            "expected field serializer to be present in field of type {:?}",
            self.var_type
        );

        fs.unwrap_unchecked().get_child_unchecked(index)
    }

    #[inline(always)]
    pub fn var_encoder_hash_eq(&self, var_encoder_hash: u64) -> bool {
        self.var_encoder_hash
            .map(|veh| veh == var_encoder_hash)
            .unwrap_or(false)
    }
}

// NOTE: Clone derive is needed here because Entity in entities.rs needs to be
// clonable which means that all members of it also should be clonable.
#[derive(Debug, Clone, Default)]
pub struct FlattenedSerializer {
    #[cfg(debug_assertions)]
    pub serializer_name: Box<str>,
    pub serializer_name_hash: u64,

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
        let serializer_name_hash = fxhash::hash_u8(serializer_name.as_bytes());

        Ok(Self {
            #[cfg(debug_assertions)]
            serializer_name: serializer_name.clone().into_boxed_str(),
            serializer_name_hash,

            serializer_version: fs.serializer_version,
            fields: Vec::with_capacity(fs.fields_index.len()),
        })
    }

    #[inline(always)]
    pub unsafe fn get_child_unchecked(&self, index: usize) -> &FlattenedSerializerField {
        #[cfg(debug_assertions)]
        debug_assert!(
            self.fields.get(index).is_some(),
            "expected field to be at index {} in {}",
            index,
            self.serializer_name
        );

        self.fields.get_unchecked(index)
    }
}

type FieldMap = HashMap<i32, Rc<FlattenedSerializerField>, BuildHasherDefault<NoHashHasher<i32>>>;
type SerializerMap = HashMap<u64, Rc<FlattenedSerializer>, BuildHasherDefault<NoHashHasher<u64>>>;

pub struct FlattenedSerializers {
    serializer_map: SerializerMap,
}

impl FlattenedSerializers {
    pub fn parse(cmd: CDemoSendTables) -> Result<Self> {
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

        for serializer in msg.serializers.iter() {
            let mut flattened_serializer = FlattenedSerializer::new(&msg, serializer)?;

            for field_index in serializer.fields_index.iter() {
                let field = if fields.contains_key(field_index) {
                    // SAFETY: we already know that hashmap has the key!
                    let field = unsafe { fields.get(field_index).unwrap_unchecked() };
                    // NOTE: it is more efficient to clone outself instead of
                    // using .clonned() because we're doing unsafe unwrap which
                    // removes the branch, but .clonned() used match under the
                    // hood which adds a branch!
                    field.clone()
                } else {
                    let mut field =
                        FlattenedSerializerField::new(&msg, &msg.fields[*field_index as usize])?;

                    if let Some(field_serializer_name_hash) =
                        field.field_serializer_name_hash.as_ref()
                    {
                        field.field_serializer =
                            serializer_map.get(field_serializer_name_hash).cloned();
                    }

                    // TODO: maybe extract arms into separate functions
                    match field.metadata.special_descriptor {
                        Some(FieldSpecialDescriptor::Array { length }) => {
                            field.field_serializer = Some(Rc::new(FlattenedSerializer {
                                fields: {
                                    let mut fields = Vec::with_capacity(length);
                                    fields.resize(length, Rc::new(field.clone()));
                                    fields
                                },
                                ..Default::default()
                            }));
                        }
                        Some(FieldSpecialDescriptor::VariableLengthArray) => {
                            field.field_serializer = Some(Rc::new(FlattenedSerializer {
                                fields: {
                                    const SIZE: usize = 128;
                                    let mut fields = Vec::with_capacity(SIZE);
                                    fields.resize(SIZE, Rc::new(field.clone()));
                                    fields
                                },
                                ..Default::default()
                            }));
                        }
                        Some(FieldSpecialDescriptor::VariableLengthSerializerArray) => {
                            field.field_serializer = Some(Rc::new(FlattenedSerializer {
                                fields: {
                                    let sub_field = FlattenedSerializerField {
                                        field_serializer: field
                                            .field_serializer_name_hash
                                            .and_then(|field_serializer_name_hash| {
                                                serializer_map.get(&field_serializer_name_hash)
                                            })
                                            .cloned(),
                                        ..Default::default()
                                    };

                                    const SIZE: usize = 128;
                                    let mut sub_fields = Vec::with_capacity(SIZE);
                                    sub_fields.resize(SIZE, Rc::new(sub_field));
                                    sub_fields
                                },
                                ..Default::default()
                            }));
                        }
                        _ => {}
                    }

                    let field = Rc::new(field);
                    // NOTE: not field is being clonned, but rc!
                    let ret = field.clone();
                    fields.insert(*field_index, field);
                    ret
                };
                flattened_serializer.fields.push(field);
            }

            serializer_map.insert(
                flattened_serializer.serializer_name_hash,
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
