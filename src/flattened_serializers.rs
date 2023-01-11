use crate::{protos, read_varu32};
use anyhow::Result;
use prost::Message;
use std::{collections::HashMap, io::Cursor};

// TODO: can we not clone strings? can we do something more efficient?

#[derive(Debug)]
struct FlattenedSerializer {
    serializer_name: String,
    serializer_version: i32,
    fields: Vec<FlattenedSerializerField>,
}

impl FlattenedSerializer {
    fn new(
        msg: &protos::CsvcMsgFlattenedSerializer,
        fs: &protos::ProtoFlattenedSerializerT,
    ) -> Self {
        Self {
            serializer_name: msg.symbols
                [fs.serializer_name_sym.expect("some serializer name") as usize]
                .to_owned(),
            serializer_version: fs.serializer_version.expect("some serializer version"),
            fields: Vec::with_capacity(fs.fields_index.len()),
        }
    }
}

#[derive(Debug)]
struct FlattenedSerializerField {
    var_type: Option<String>,
    var_name: Option<String>,
    bit_count: Option<i32>,
    low_value: Option<f32>,
    high_value: Option<f32>,
    encode_flags: Option<i32>,
    field_serializer_name: Option<String>,
    field_serializer_version: Option<i32>,
    send_node: Option<String>,
    var_encoder: Option<String>,
}

impl FlattenedSerializerField {
    fn new(
        msg: &protos::CsvcMsgFlattenedSerializer,
        fsf: &protos::ProtoFlattenedSerializerFieldT,
    ) -> Self {
        let resolve = |v: i32| Some(msg.symbols[v as usize].to_owned());
        Self {
            var_type: fsf.var_type_sym.and_then(resolve),
            var_name: fsf.var_name_sym.and_then(resolve),
            bit_count: fsf.bit_count,
            low_value: fsf.low_value,
            high_value: fsf.high_value,
            encode_flags: fsf.encode_flags,
            field_serializer_name: fsf.field_serializer_name_sym.and_then(resolve),
            field_serializer_version: fsf.field_serializer_version,
            send_node: fsf.send_node_sym.and_then(resolve),
            var_encoder: fsf.var_encoder_sym.and_then(resolve),
        }
    }
}

pub struct FlattenedSerializers {
    serializers: HashMap<String, FlattenedSerializer>,
}

impl FlattenedSerializers {
    pub fn new(data: &[u8]) -> Result<Self> {
        let st = protos::CDemoSendTables::decode(data)?;
        let st_data = st.data.expect("some data");
        let mut st_data_cursor = Cursor::new(&st_data);

        let size = read_varu32(&mut st_data_cursor)? as usize;
        let offset = st_data_cursor.position() as usize;
        let msg = protos::CsvcMsgFlattenedSerializer::decode(&st_data[offset..offset + size])?;

        let mut serializers: HashMap<String, FlattenedSerializer> = HashMap::new();

        msg.serializers.iter().for_each(|fs| {
            let mut serializer = FlattenedSerializer::new(&msg, fs);
            fs.fields_index.iter().for_each(|field_index| {
                let fsf = FlattenedSerializerField::new(&msg, &msg.fields[*field_index as usize]);
                serializer.fields.push(fsf);
            });
            serializers.insert(serializer.serializer_name.to_owned(), serializer);
        });

        Ok(Self { serializers })
    }
}
