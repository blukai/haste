use crate::{
    bitreader::BitReader,
    client::{self, UpdateType},
    dem::{self, Msg as DemMsg},
    entity_classes::EntityClasses,
    error::{required, Error, Result},
    field_path::{build_field_ops_tree, FieldPath, Tree, FIELD_OPS},
    flattened_serializers::FlattenedSerializers,
    packet::{self, Msg as PacketMsg},
    protos::{self, EDemoCommands},
    string_tables::StringTables,
};
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use std::{
    alloc::{Allocator, Global},
    fmt::Debug,
    io::{self, Read, Seek, SeekFrom},
};

// TODO: don't read byte-by-byte, use buffered read because it's faster!

// some cool docs on dota 2 replays:
// https://github.com/skadistats/smoke/wiki/Anatomy-of-a-Dota-2-Replay-File

// ~/.local/share/Steam/steamapps/common/dota 2 beta/game/core/tools/demoinfo2/demoinfo2.txt
// has some interesting mappings :thinking:

const DEMO_FILE_STAMP: [u8; 8] = *b"PBDEMS2\0";
const MSG_BUF_SIZE: usize = 1024 * 1024;

#[derive(thiserror::Error, Debug)]
pub enum ParserError {
    #[error("unexpected header id (want {want:?}, got {got:?})")]
    InvalidHeader { want: [u8; 8], got: [u8; 8] },
}

pub struct Parser<R: Read + Seek, A: Allocator + Clone + Debug = Global> {
    rdr: R,
    file_info_offset: i32,
    buf: Vec<u8, A>,
    entity_classes: Option<EntityClasses<A>>,
    string_tables: StringTables<A>,
    instancebaseline: Option<HashMap<i32, Vec<u8, A>, DefaultHashBuilder, A>>,
    flattened_serializers: Option<FlattenedSerializers<A>>,
    // packet_entities: HashMap<i32, PacketEntity, DefaultHashBuilder, A>,
    server_info: Option<protos::CsvcMsgServerInfo>,
    alloc: A,
}

impl<R: Read + Seek> Parser<R> {
    pub fn new(rdr: R) -> Result<Self> {
        Self::new_in(rdr, Global)
    }
}

impl<R: Read + Seek, A: Allocator + Clone + Debug> Parser<R, A> {
    pub fn new_in(mut rdr: R, alloc: A) -> Result<Self> {
        let header = dem::Header::from_reader(&mut rdr)?;
        if header.demo_file_stamp != DEMO_FILE_STAMP {
            return Err(ParserError::InvalidHeader {
                want: DEMO_FILE_STAMP,
                got: header.demo_file_stamp,
            }
            .into());
        }

        let mut msg_buf = Vec::with_capacity_in(MSG_BUF_SIZE, alloc.clone());
        msg_buf.resize_with(MSG_BUF_SIZE, Default::default);

        Ok(Self {
            rdr,
            file_info_offset: header.file_info_offset,
            buf: msg_buf,
            entity_classes: None,
            string_tables: StringTables::new_in(alloc.clone()),
            instancebaseline: None,
            flattened_serializers: None,
            // NOTE: 20480 is is value of BUTTERFLY_MAX_ENTS in butterfly
            // packet_entities: HashMap::with_capacity_in(20480, alloc.clone()),
            server_info: None,
            alloc,
        })
    }

    pub fn read_file_info(&mut self) -> Result<protos::CDemoFileInfo> {
        let initial_pos = self.rdr.stream_position()?;
        self.rdr
            .seek(SeekFrom::Start(self.file_info_offset as u64))?;

        // NOTE: rust does not have defer, after going to the offset we want to
        // restore position even in case of error.
        #[inline(always)]
        fn inner<R: Read + Seek>(rdr: &mut R, buf: &mut [u8]) -> Result<protos::CDemoFileInfo> {
            let msg_header = dem::MsgHeader::from_reader(rdr)?;
            protos::CDemoFileInfo::from_reader(rdr, &msg_header, buf)
        }
        let result = inner(&mut self.rdr, &mut self.buf);

        self.rdr.seek(SeekFrom::Start(initial_pos))?;

        result
    }

    pub fn process_next_msg(&mut self) -> Result<bool> {
        match dem::MsgHeader::from_reader(&mut self.rdr) {
            // TODO: can this error matching be less obnoxious?
            Err(Error::Io(e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                // we're done.
                Ok(true)
            }
            Err(e) => Err(e),
            Ok(msg_header) => {
                use EDemoCommands::*;
                match msg_header.command {
                    DemSignonPacket | DemPacket => {
                        let proto = protos::CDemoPacket::from_reader(
                            &mut self.rdr,
                            &msg_header,
                            &mut self.buf,
                        )?;
                        self.handle_packet(&proto.data.ok_or(required!())?)?;
                    }
                    DemSendTables => {
                        let proto = protos::CDemoSendTables::from_reader(
                            &mut self.rdr,
                            &msg_header,
                            &mut self.buf,
                        )?;
                        self.flattened_serializers =
                            Some(FlattenedSerializers::new_in(proto, self.alloc.clone())?);
                    }
                    DemClassInfo => {
                        let proto = protos::CDemoClassInfo::from_reader(
                            &mut self.rdr,
                            &msg_header,
                            &mut self.buf,
                        )?;
                        let entity_classes = EntityClasses::new_in(proto, self.alloc.clone())?;
                        self.entity_classes = Some(entity_classes);
                    }

                    // CDemoFullPacket contains the entire state of the world at
                    // that tick. Full packets are only taken once every 1800
                    // ticks (1 minute).
                    // TODO: do we want to handle this now?
                    // DemFullPacket => {
                    //     let proto = protos::CDemoFullPacket::from_reader(
                    //         &mut self.rdr,
                    //         &msg_header,
                    //         &mut self.buf,
                    //     )?;
                    //     self.handle_packet(
                    //         &proto.packet.ok_or(required!())?.data.ok_or(required!())?,
                    //     )?;
                    // }
                    _ => {
                        self.rdr.seek(SeekFrom::Current(msg_header.size as i64))?;
                    }
                }
                Ok(false)
            }
        }
    }

    fn handle_packet(&mut self, data: &[u8]) -> Result<()> {
        let mut br = BitReader::new(data);
        while !br.is_empty() {
            let packet_header = packet::Header::from_bitreader(&mut br)?;
            match packet_header.command {
                c if c == protos::SvcMessages::SvcServerInfo as u32 => {
                    self.server_info = Some(protos::CsvcMsgServerInfo::from_bitreader(
                        &mut br,
                        &packet_header,
                        &mut self.buf,
                    )?);
                }
                c if c == protos::SvcMessages::SvcCreateStringTable as u32 => {
                    let proto = protos::CsvcMsgCreateStringTable::from_bitreader(
                        &mut br,
                        &packet_header,
                        &mut self.buf,
                    )?;
                    if proto.name.as_ref().ok_or(required!())? == "instancebaseline" {
                        let string_table = self.string_tables.create(proto)?;
                        // NOTE: table can only be created once
                        let mut instancebaseline =
                            HashMap::with_capacity_in(string_table.len(), self.alloc.clone());
                        for (_, v) in string_table.iter() {
                            let class_id = v.key.as_ref().ok_or(required!())?.parse::<i32>()?;
                            instancebaseline
                                .insert(class_id, v.value.as_ref().ok_or(required!())?.clone());
                        }
                        self.instancebaseline = Some(instancebaseline);
                    }
                }
                c if c == protos::SvcMessages::SvcUpdateStringTable as u32 => {
                    let proto = protos::CsvcMsgUpdateStringTable::from_bitreader(
                        &mut br,
                        &packet_header,
                        &mut self.buf,
                    )?;
                    // TODO: update instancebaseline (/class_baselines) table
                    unimplemented!()
                }
                c if c == protos::SvcMessages::SvcPacketEntities as u32 => {
                    let proto = protos::CsvcMsgPacketEntities::from_bitreader(
                        &mut br,
                        &packet_header,
                        &mut self.buf,
                    )?;
                    self.handle_packet_entities(proto)?;
                }
                _ => {
                    br.skip_bytes(packet_header.size as u64)?;
                }
            }
        }
        Ok(())
    }

    fn handle_packet_entities(&mut self, proto: protos::CsvcMsgPacketEntities) -> Result<()> {
        let entity_classes = self.entity_classes.as_ref().ok_or(required!())?;
        let instancebaseline = self.instancebaseline.as_ref().ok_or(required!())?;
        let flattened_serializers = self.flattened_serializers.as_ref().ok_or(required!())?;

        let entity_data = proto.entity_data.ok_or(required!())?;
        let mut br = BitReader::new(&entity_data);

        let mut idx: i32 = -1;

        for i in 0..proto.updated_entries.ok_or(required!())? {
            idx += br.read_ubitvar()? as i32 + 1;

            // TODO: what if this not delta packet (proto.is_delta) ?
            let update_flags = client::parse_delta_header(&mut br)?;
            let update_type = client::determine_update_type(update_flags);

            match update_type {
                UpdateType::EnterPVS => {
                    let class_id = br.read(entity_classes.bits())? as i32;
                    let _serial = br.read(17)?;
                    let _unknown = br.read_varu32()?;

                    let class = entity_classes.get(&class_id).ok_or(required!())?;
                    let baseline_value = instancebaseline.get(&class_id).ok_or(required!())?;

                    {
                        let read_fields = |br: &mut BitReader| -> Result<()> {
                            let mut fp = FieldPath::new();
                            let mut fps: Vec<FieldPath, A> = Vec::new_in(self.alloc.clone());

                            let root = build_field_ops_tree();
                            let mut node: &Tree<usize> = &root;
                            while !fp.finished {
                                let next = if br.read_bool()? {
                                    node.right().expect("right branch")
                                } else {
                                    node.left().expect("left branch")
                                };
                                match next {
                                    Tree::Leaf { value, .. } => {
                                        node = &root;
                                        dbg!(&FIELD_OPS[*value].name);
                                        (&FIELD_OPS[*value].fp)(&mut fp, br)?;
                                        dbg!(format!("{:?}", &fp.data));
                                        if !fp.finished {
                                            fps.push(fp.clone())
                                        }
                                    }
                                    Tree::Node { .. } => node = next,
                                }
                            }

                            let serializer = flattened_serializers
                                .get_by_class_id(&class.network_name.as_ref().ok_or(required!())?)
                                .ok_or(required!())?;

                            for fp in fps {
                                let f = match fp.position {
                                    0 => serializer.fields.get(fp.data[0] as usize),
                                    1 => serializer.fields.get(fp.data[0] as usize).and_then(|f| {
                                        // dbg!(&f.var_type, &f.var_name);
                                        if f.size != 0 {
                                            Some(f)
                                        } else if f.is_dynamic {
                                            Some(f)
                                        } else {
                                            f.field_serializer.as_ref().and_then(|serializer| {
                                                serializer.fields.get(fp.data[1] as usize)
                                            })
                                        }
                                    }),
                                    _ => unimplemented!(),
                                };

                                print!(
                                    "ser={:<8} \tfp={:>2?}",
                                    &serializer.serializer_name,
                                    &fp.data[..=fp.position]
                                );

                                if let Some(f) = f {
                                    print!(" \tt={:<55} \tn={:<43}", &f.var_type, &f.var_name);

                                    print!(
                                        "\tbc={:?} \tlv={:?} \thv={:?} \tef={:?} \te={:<24?}",
                                        &f.bit_count,
                                        &f.low_value,
                                        &f.high_value,
                                        &f.encode_flags,
                                        &f.var_encoder,
                                    );

                                    if let Some(value) =
                                        f.decoder.as_ref().map(|decode| (decode)(br, &f))
                                    {
                                        print!(" \tvalue={:?}", value?);
                                    }

                                    if f.decoder.is_none() {
                                        unimplemented!();
                                    }
                                }
                                println!();
                            }

                            Ok(())
                        };

                        read_fields(&mut BitReader::new(&baseline_value))?;
                        read_fields(&mut br)?;
                    }

                    // TODO!
                    // unimplemented!()
                }
                update_type => {
                    dbg!(update_type);
                    unimplemented!()
                }
            }
        }

        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use std::fs::File;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn parse() -> Result<()> {
        let file = File::open("./fixtures/7116662198_1379602574.dem")?;
        let mut parser = Parser::new(file)?;
        while !parser.process_next_msg()? {}
        Ok(())
    }

    #[test]
    fn parse_file_info() -> Result<()> {
        let file = File::open("./fixtures/7116662198_1379602574.dem")?;
        let mut parser = Parser::new(file)?;
        let _file_info = parser.read_file_info()?;
        // dbg!(file_info);
        Ok(())
    }
}
