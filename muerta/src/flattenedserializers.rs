use crate::{
    fielddecoder::{self, FieldDecoder},
    fieldkind::FieldKind,
    fieldpath::FieldPath,
    fnv1a, protos, varint,
};
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use prost::Message;
use std::alloc::{Allocator, Global};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // 3rd party crates
    #[error(transparent)]
    Prost(#[from] prost::DecodeError),
    // crate
    #[error(transparent)]
    Varint(#[from] varint::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

// some info about string tables
// https://developer.valvesoftware.com/wiki/Networking_Events_%26_Messages
// https://developer.valvesoftware.com/wiki/Networking_Entities

#[derive(Clone)]
pub struct FlattenedSerializerField<A: Allocator + Clone> {
    pub var_type: Vec<u8, A>,
    pub var_type_hash: u64,

    pub var_name: Vec<u8, A>,
    pub var_name_hash: u64,

    // TODO: figure out which fields should, and which should not be optional
    pub bit_count: Option<i32>,
    pub low_value: Option<f32>,
    pub high_value: Option<f32>,
    pub encode_flags: Option<i32>,

    pub field_serializer_name: Option<Vec<u8, A>>,
    pub field_serializer_name_hash: Option<u64>,
    pub field_serializer: Option<FlattenedSerializer<A>>,

    pub field_serializer_version: Option<i32>,
    pub send_node: Option<Vec<u8, A>>,

    pub var_encoder: Option<Vec<u8, A>>,
    pub var_encoder_hash: Option<u64>,

    pub kind: Option<FieldKind>,
    pub decoder: Option<FieldDecoder<A>>,
}

impl<A: Allocator + Clone> FlattenedSerializerField<A> {
    fn new_in(
        svcmsg: &protos::CsvcMsgFlattenedSerializer,
        field: &protos::ProtoFlattenedSerializerFieldT,
        alloc: A,
    ) -> Self {
        let resolve_string = |v: i32| {
            Some(
                svcmsg.symbols[v as usize]
                    .as_bytes()
                    .to_vec_in(alloc.clone()),
            )
        };

        let var_type = field
            .var_type_sym
            .and_then(resolve_string)
            .expect("var type");
        let var_type_hash = fnv1a::hash(&var_type);

        let var_name = field
            .var_name_sym
            .and_then(resolve_string)
            .expect("var name");
        let var_name_hash = fnv1a::hash(&var_name);

        let field_serializer_name = field.field_serializer_name_sym.and_then(resolve_string);
        let field_serializer_name_hash = field_serializer_name
            .as_ref()
            .and_then(|field_serializer_name| Some(fnv1a::hash(field_serializer_name)));

        let var_encoder = field.var_encoder_sym.and_then(resolve_string);
        let var_encoder_hash = var_encoder
            .as_ref()
            .and_then(|var_encoder| Some(fnv1a::hash(var_encoder)));

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
            send_node: field.send_node_sym.and_then(resolve_string),

            var_encoder,
            var_encoder_hash,

            kind: None,
            decoder: None,
        }
    }

    pub fn get_field_for_field_path(
        &self,
        fp: FieldPath,
        pos: usize,
    ) -> &FlattenedSerializerField<A> {
        match self.kind {
            None => {
                if fp.position != pos - 1 {
                    self.field_serializer
                        .as_ref()
                        .expect("field serializer")
                        .get_field_for_field_path(fp, pos)
                } else {
                    self
                }
            }
            Some(FieldKind::FixedArray { .. }) | Some(FieldKind::DynamicArray) => self,
            Some(FieldKind::FixedTable { .. }) | Some(FieldKind::DynamicTable) => {
                if fp.position >= pos + 1 {
                    self.field_serializer
                        .as_ref()
                        .expect("field serializer")
                        .get_field_for_field_path(fp, pos + 1)
                } else {
                    self
                }
            }
        }
    }

    pub fn is_var_encoder_hash_eq(&self, var_encoder_hash: u64) -> bool {
        self.var_encoder_hash
            .map(|veh| veh == var_encoder_hash)
            .unwrap_or(false)
    }
}

// NOTE: Clone derive is needed here because Entity in entities.rs needs to be
// clonable which means that all members of it also should be clonable.
#[derive(Clone)]
pub struct FlattenedSerializer<A: Allocator + Clone> {
    pub serializer_name: Vec<u8, A>,
    pub serializer_version: Option<i32>,
    pub fields: Vec<FlattenedSerializerField<A>, A>,

    pub serializer_name_hash: u64,
}

impl<A: Allocator + Clone> FlattenedSerializer<A> {
    fn new_in(
        svcmsg: &protos::CsvcMsgFlattenedSerializer,
        fs: &protos::ProtoFlattenedSerializerT,
        alloc: A,
    ) -> Result<Self> {
        let resolve_string = |v: i32| {
            Some(
                svcmsg.symbols[v as usize]
                    .as_bytes()
                    .to_vec_in(alloc.clone()),
            )
        };
        let serializer_name = fs
            .serializer_name_sym
            .and_then(resolve_string)
            .expect("serializer name");
        let serializer_name_hash = fnv1a::hash(&serializer_name);

        Ok(Self {
            serializer_name,
            serializer_version: fs.serializer_version,
            fields: Vec::with_capacity_in(fs.fields_index.len(), alloc.clone()),
            serializer_name_hash,
        })
    }

    pub fn get_field_for_field_path(
        &self,
        fp: FieldPath,
        pos: usize,
    ) -> &FlattenedSerializerField<A> {
        self.fields
            .get(fp.data[pos] as usize)
            .expect("field")
            .get_field_for_field_path(fp, pos + 1)
    }
}

type FieldMap<A> = HashMap<i32, FlattenedSerializerField<A>, DefaultHashBuilder, A>;
type SerializerMap<A> = HashMap<u64, FlattenedSerializer<A>, DefaultHashBuilder, A>;

pub struct FlattenedSerializers<A: Allocator + Clone = Global> {
    serializers: Option<SerializerMap<A>>,
    alloc: A,
}

impl FlattenedSerializers<Global> {
    pub fn new() -> Self {
        Self::new_in(Global)
    }
}

impl<A: Allocator + Clone> FlattenedSerializers<A> {
    pub fn new_in(alloc: A) -> Self {
        Self {
            serializers: None,
            alloc,
        }
    }

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

        let mut fields: FieldMap<A> = FieldMap::new_in(self.alloc.clone());
        let mut serializers: SerializerMap<A> =
            SerializerMap::with_capacity_in(svcmsg.serializers.len(), self.alloc.clone());

        for serializer in svcmsg.serializers.iter() {
            let mut flattened_serializer =
                FlattenedSerializer::new_in(&svcmsg, serializer, self.alloc.clone())?;

            for field_index in serializer.fields_index.iter() {
                if !fields.contains_key(field_index) {
                    let mut field = FlattenedSerializerField::new_in(
                        &svcmsg,
                        &svcmsg.fields[*field_index as usize],
                        self.alloc.clone(),
                    );
                    fielddecoder::supplement(&mut field);

                    if let Some(field_serializer_name_hash) =
                        field.field_serializer_name_hash.as_ref()
                    {
                        if let Some(field_serializer) = serializers.get(field_serializer_name_hash)
                        {
                            field.field_serializer = Some(field_serializer.clone());
                        }
                    }

                    // fixed arrays
                    // if let Some(size) = field.size {
                    //     let mut field_serializer = FlattenedSerializer {
                    //         // serializer_name will never be set
                    //         serializer_name: Vec::with_capacity_in(0, self.alloc.clone()),
                    //         serializer_version: None,
                    //         fields: Vec::with_capacity_in(size, self.alloc.clone()),
                    //         serializer_name_hash: 0,
                    //     };

                    //     for i in 0..size {
                    //         let mut field = field.clone();
                    //         field.var_name = usize_to_byte_vec_in(i, self.alloc.clone());
                    //         field_serializer.fields.push(field);
                    //     }
                    //     field.field_serializer = Some(field_serializer);
                    // }

                    // TODO: handle dynamic vectors

                    fields.insert(*field_index, field);
                };

                let field = fields.get(field_index).unwrap();
                flattened_serializer.fields.push(field.clone());
            }

            serializers.insert(
                flattened_serializer.serializer_name_hash,
                flattened_serializer,
            );
        }

        self.serializers = Some(serializers);

        Ok(())
    }

    #[inline(always)]
    fn serializers(&self) -> &SerializerMap<A> {
        self.serializers.as_ref().expect("serializers to be parsed")
    }

    pub fn get_by_serializer_name_hash(
        &self,
        serializer_name_hash: u64,
    ) -> Option<&FlattenedSerializer<A>> {
        self.serializers().get(&serializer_name_hash)
    }
}
