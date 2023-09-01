use crate::{
    fieldmetadata::{self, get_field_metadata, FieldMetadata, FieldSpecialType},
    fnv1a,
    hashers::{I32HashBuilder, U64HashBuiler},
    protos, varint,
};
use hashbrown::HashMap;
use prost::Message;
use std::rc::Rc;

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

#[derive(Clone)]
pub struct FlattenedSerializerField {
    pub var_type: Box<str>,
    pub var_type_hash: u64,

    pub var_name: Box<str>,
    pub var_name_hash: u64,

    // TODO: figure out which fields should, and which should not be optional
    pub bit_count: Option<i32>,
    pub low_value: Option<f32>,
    pub high_value: Option<f32>,
    pub encode_flags: Option<i32>,

    pub field_serializer_name: Option<Box<str>>,
    pub field_serializer_name_hash: Option<u64>,
    pub field_serializer: Option<Rc<FlattenedSerializer>>,

    pub field_serializer_version: Option<i32>,
    pub send_node: Option<Box<str>>,

    pub var_encoder: Option<Box<str>>,
    pub var_encoder_hash: Option<u64>,

    pub metadata: Option<FieldMetadata>,
}

impl FlattenedSerializerField {
    fn new(
        svcmsg: &protos::CsvcMsgFlattenedSerializer,
        field: &protos::ProtoFlattenedSerializerFieldT,
    ) -> Self {
        let resolve_sym = |v: i32| Some(svcmsg.symbols[v as usize].clone().into_boxed_str());

        let var_type = field.var_type_sym.and_then(resolve_sym).expect("var type");
        let var_type_hash = fnv1a::hash(var_type.as_bytes());

        let var_name = field.var_name_sym.and_then(resolve_sym).expect("var name");
        let var_name_hash = fnv1a::hash(var_name.as_bytes());

        let field_serializer_name = field.field_serializer_name_sym.and_then(resolve_sym);
        let field_serializer_name_hash = field_serializer_name
            .as_ref()
            .map(|field_serializer_name| fnv1a::hash(field_serializer_name.as_bytes()));

        let var_encoder = field.var_encoder_sym.and_then(resolve_sym);
        let var_encoder_hash = var_encoder
            .as_ref()
            .map(|var_encoder| fnv1a::hash(var_encoder.as_bytes()));

        Self {
            var_type,
            var_type_hash,

            var_name,
            var_name_hash,

            bit_count: field.bit_count,
            low_value: field.low_value,
            high_value: field.high_value,
            encode_flags: field.encode_flags,

            field_serializer_name,
            field_serializer_name_hash,
            field_serializer: None,

            field_serializer_version: field.field_serializer_version,
            send_node: field.send_node_sym.and_then(resolve_sym),

            var_encoder,
            var_encoder_hash,

            metadata: None,
        }
    }

    #[inline(always)]
    pub fn get_child(&self, index: usize) -> &Self {
        self.field_serializer
            .as_ref()
            .unwrap_or_else(|| {
                panic!(
                    "expected field serialized to be present in field of type {}",
                    self.var_type
                )
            })
            .get_child(index)
    }

    pub fn is_var_encoder_hash_eq(&self, var_encoder_hash: u64) -> bool {
        self.var_encoder_hash
            .map(|veh| veh == var_encoder_hash)
            .unwrap_or(false)
    }

    #[inline]
    fn default() -> Self {
        Self {
            var_type: Box::default(),
            var_type_hash: 0,

            var_name: Box::default(),
            var_name_hash: 0,

            // TODO: figure out which fields should, and which should not be optional
            bit_count: None,
            low_value: None,
            high_value: None,
            encode_flags: None,

            field_serializer_name: None,
            field_serializer_name_hash: None,
            field_serializer: None,

            field_serializer_version: None,
            send_node: None,

            var_encoder: None,
            var_encoder_hash: None,

            metadata: None,
        }
    }
}

// NOTE: Clone derive is needed here because Entity in entities.rs needs to be
// clonable which means that all members of it also should be clonable.
#[derive(Clone)]
pub struct FlattenedSerializer {
    pub serializer_name: Box<str>,
    pub serializer_version: Option<i32>,
    pub fields: Vec<Rc<FlattenedSerializerField>>,

    pub serializer_name_hash: u64,
}

impl FlattenedSerializer {
    fn new(
        svcmsg: &protos::CsvcMsgFlattenedSerializer,
        fs: &protos::ProtoFlattenedSerializerT,
    ) -> Result<Self> {
        let resolve_sym = |v: i32| Some(svcmsg.symbols[v as usize].clone().into_boxed_str());

        let serializer_name = fs
            .serializer_name_sym
            .and_then(resolve_sym)
            .expect("serializer name");
        let serializer_name_hash = fnv1a::hash(serializer_name.as_bytes());

        Ok(Self {
            serializer_name,
            serializer_version: fs.serializer_version,
            fields: Vec::with_capacity(fs.fields_index.len()),
            serializer_name_hash,
        })
    }

    #[inline(always)]
    pub fn get_child(&self, index: usize) -> &FlattenedSerializerField {
        self.fields.get(index).unwrap_or_else(|| {
            panic!(
                "expected field to be at index {} in {}",
                index, self.serializer_name
            )
        })
    }
}

type FieldMap = HashMap<i32, Rc<FlattenedSerializerField>, I32HashBuilder>;
type SerializerMap = HashMap<u64, Rc<FlattenedSerializer>, U64HashBuiler>;

#[derive(Default)]
pub struct FlattenedSerializers {
    serializers: Option<SerializerMap>,
}

impl FlattenedSerializers {
    pub fn parse(&mut self, cmd: protos::CDemoSendTables) -> Result<()> {
        debug_assert!(
            self.serializers.is_none(),
            "serializer map is expected to not be created yet"
        );

        let svcmsg = {
            let data = cmd.data.expect("send tables data");
            let (size, offset) = varint::uvarint32(&data)
                .map(|(size, bytes_read)| (size as usize, bytes_read + 1))?;
            protos::CsvcMsgFlattenedSerializer::decode(&data[offset..offset + size])?
        };

        let mut fields: FieldMap = FieldMap::with_hasher(I32HashBuilder::default());
        let mut serializers: SerializerMap = SerializerMap::with_capacity_and_hasher(
            svcmsg.serializers.len(),
            U64HashBuiler::default(),
        );

        for serializer in svcmsg.serializers.iter() {
            let mut flattened_serializer = FlattenedSerializer::new(&svcmsg, serializer)?;

            for field_index in serializer.fields_index.iter() {
                if !fields.contains_key(field_index) {
                    let mut field = FlattenedSerializerField::new(
                        &svcmsg,
                        &svcmsg.fields[*field_index as usize],
                    );

                    if let Some(field_serializer_name_hash) =
                        field.field_serializer_name_hash.as_ref()
                    {
                        if let Some(field_serializer) = serializers.get(field_serializer_name_hash)
                        {
                            field.field_serializer = Some(field_serializer.clone());
                        }
                    }

                    field.metadata = get_field_metadata(&field)?;
                    // TODO: maybe extract arms into separate functions
                    match field.metadata.as_ref() {
                        Some(field_metadata) => match field_metadata.special_type {
                            Some(FieldSpecialType::Array { length }) => {
                                let mut fields = Vec::with_capacity(length);
                                fields.resize(length, Rc::new(field.clone()));

                                field.field_serializer = Some(Rc::new(FlattenedSerializer {
                                    serializer_name: Box::default(),
                                    serializer_version: None,
                                    fields,
                                    serializer_name_hash: 0,
                                }));
                            }
                            Some(FieldSpecialType::VariableLengthArray) => {
                                const LENGTH: usize = 256;
                                let mut fields = Vec::with_capacity(LENGTH);
                                fields.resize(LENGTH, Rc::new(field.clone()));

                                field.field_serializer = Some(Rc::new(FlattenedSerializer {
                                    serializer_name: Box::default(),
                                    serializer_version: None,
                                    fields,
                                    serializer_name_hash: 0,
                                }));
                            }
                            Some(FieldSpecialType::VariableLengthSerializerArray {
                                element_serializer_name_hash,
                            }) => {
                                let mut sub_field = FlattenedSerializerField::default();
                                sub_field.field_serializer =
                                    serializers.get(&element_serializer_name_hash).cloned();

                                const SIZE: usize = 256;
                                let mut sub_fields = Vec::with_capacity(SIZE);
                                sub_fields.resize(SIZE, Rc::new(sub_field));

                                field.field_serializer = Some(Rc::new(FlattenedSerializer {
                                    serializer_name: Box::default(),
                                    serializer_version: None,
                                    fields: sub_fields,
                                    serializer_name_hash: 0,
                                }));
                            }
                            _ => {}
                        },
                        None => {
                            // TODO: don't panic?
                            panic!(
                                "unhandled flattened serializer var type: {}",
                                &field.var_type
                            )
                        }
                    }

                    fields.insert(*field_index, Rc::new(field));
                };

                let field = fields.get(field_index).unwrap();
                flattened_serializer.fields.push(field.clone());
            }

            serializers.insert(
                flattened_serializer.serializer_name_hash,
                Rc::new(flattened_serializer),
            );
        }

        self.serializers = Some(serializers);

        Ok(())
    }

    #[inline(always)]
    fn serializers(&self) -> &SerializerMap {
        self.serializers.as_ref().expect("serializers to be parsed")
    }

    pub fn get_by_serializer_name_hash(
        &self,
        serializer_name_hash: u64,
    ) -> Option<&FlattenedSerializer> {
        self.serializers()
            .get(&serializer_name_hash)
            .map(|v| v.as_ref())
    }
}
