use crate::{
    error::{required, Result},
    protos,
    varint::VarIntRead,
};
use compact_str::CompactString;
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use prost::Message;
use std::{alloc::Allocator, io::Cursor};

#[derive(Debug)]
struct FlattenedSerializerField {
    // TODO: figure out which fields should, and which should not be optional
    var_type: Option<CompactString>,
    var_name: Option<CompactString>,
    bit_count: Option<i32>,
    low_value: Option<f32>,
    high_value: Option<f32>,
    encode_flags: Option<i32>,
    field_serializer_name: Option<CompactString>,
    field_serializer_version: Option<i32>,
    send_node: Option<CompactString>,
    var_encoder: Option<CompactString>,
}

impl FlattenedSerializerField {
    fn new(
        fser: &protos::CsvcMsgFlattenedSerializer,
        fser_field: &protos::ProtoFlattenedSerializerFieldT,
    ) -> Self {
        let resolve_string = |v: i32| Some(CompactString::from(&fser.symbols[v as usize]));
        Self {
            var_type: fser_field.var_type_sym.and_then(resolve_string),
            var_name: fser_field.var_name_sym.and_then(resolve_string),
            bit_count: fser_field.bit_count,
            low_value: fser_field.low_value,
            high_value: fser_field.high_value,
            encode_flags: fser_field.encode_flags,
            field_serializer_name: fser_field
                .field_serializer_name_sym
                .and_then(resolve_string),
            field_serializer_version: fser_field.field_serializer_version,
            send_node: fser_field.send_node_sym.and_then(resolve_string),
            var_encoder: fser_field.var_encoder_sym.and_then(resolve_string),
        }
    }
}

#[derive(Debug)]
struct FlattenedSerializer<A: Allocator + Clone> {
    serializer_name: CompactString,
    serializer_version: i32,
    fields: Vec<FlattenedSerializerField, A>,
}

impl<A: Allocator + Clone> FlattenedSerializer<A> {
    fn new_in(
        msg: &protos::CsvcMsgFlattenedSerializer,
        fs: &protos::ProtoFlattenedSerializerT,
        alloc: A,
    ) -> Result<Self> {
        Ok(Self {
            serializer_name: CompactString::from(
                &msg.symbols[fs.serializer_name_sym.ok_or(required!())? as usize],
            ),
            serializer_version: fs.serializer_version.ok_or(required!())?,
            fields: Vec::with_capacity_in(fs.fields_index.len(), alloc.clone()),
        })
    }
}

type Container<A> = HashMap<CompactString, FlattenedSerializer<A>, DefaultHashBuilder, A>;

pub struct FlattenedSerializers<A: Allocator + Clone> {
    container: Container<A>,
}

impl<A: Allocator + Clone> FlattenedSerializers<A> {
    pub fn new_in(proto: protos::CDemoSendTables, alloc: A) -> Result<Self> {
        let data = proto.data.ok_or(required!())?;
        let mut data_cursor = Cursor::new(&data);
        let size = data_cursor.read_varu32()? as usize;
        let offset = data_cursor.position() as usize;
        let msg = protos::CsvcMsgFlattenedSerializer::decode(&data[offset..offset + size])?;

        let mut container: Container<A> = Container::new_in(alloc.clone());

        for fs in msg.serializers.iter() {
            let mut serializer = FlattenedSerializer::new_in(&msg, fs, alloc.clone())?;
            for field_index in fs.fields_index.iter() {
                let fsf = FlattenedSerializerField::new(&msg, &msg.fields[*field_index as usize]);
                serializer.fields.push(fsf);
            }
            container.insert(serializer.serializer_name.clone(), serializer);
        }

        Ok(Self { container })
    }
}
