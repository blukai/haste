use crate::{
    bitreader::BitReader,
    dem::{self, Msg as DemMsg},
    entity_classes::EntityClasses,
    error::{required, Error, Result},
    flattened_serializers::FlattenedSerializers,
    packet::{self, Msg as PacketMsg},
    packet_entitiy::{EntityOp, PacketEntity},
    protos::{self, EDemoCommands},
    string_tables::StringTables,
};
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use std::{
    alloc::{Allocator, Global},
    io::{self, Read, Seek, SeekFrom},
};

const DEMO_FILE_STAMP: [u8; 8] = *b"PBDEMS2\0";
const MSG_BUF_SIZE: usize = 1024 * 1024;

#[derive(thiserror::Error, Debug)]
pub enum ParserError {
    #[error("unexpected header id (want {want:?}, got {got:?})")]
    InvalidHeader { want: [u8; 8], got: [u8; 8] },
}

pub struct Parser<R: Read + Seek, A: Allocator + Clone = Global> {
    rdr: R,
    file_info_offset: i32,
    buf: Vec<u8, A>,
    entity_classes: Option<EntityClasses<A>>,
    string_tables: StringTables<A>,
    instancebaseline: Option<HashMap<i32, Vec<u8, A>, DefaultHashBuilder, A>>,
    packet_entities: HashMap<i32, PacketEntity, DefaultHashBuilder, A>,
    alloc: A,
}

impl<R: Read + Seek> Parser<R> {
    pub fn new(rdr: R) -> Result<Self> {
        Self::new_in(rdr, Global)
    }
}

impl<R: Read + Seek, A: Allocator + Clone> Parser<R, A> {
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
            // NOTE: 20480 is is value of BUTTERFLY_MAX_ENTS in butterfly
            packet_entities: HashMap::with_capacity_in(20480, alloc.clone()),
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
                        let flattened_serializers =
                            FlattenedSerializers::new_in(proto, self.alloc.clone())?;
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
                    DemFullPacket => {
                        let proto = protos::CDemoFullPacket::from_reader(
                            &mut self.rdr,
                            &msg_header,
                            &mut self.buf,
                        )?;
                        self.handle_packet(
                            &proto.packet.ok_or(required!())?.data.ok_or(required!())?,
                        )?;
                    }
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

        let entity_data = proto.entity_data.ok_or(required!())?;
        let mut br = BitReader::new(&entity_data);

        let mut idx: i32 = -1;

        for i in 0..proto.updated_entries.ok_or(required!())? {
            idx += br.read_ubitvar()? as i32 + 1;
            match EntityOp::from(br.read(2)?) {
                EntityOp::Create => {
                    let class_id = br.read(entity_classes.bits())? as i32;
                    let _serial = br.read(17)?;
                    let _unknown = br.read_varu32()?;

                    let class = entity_classes.get(&class_id);
                    let baseline_value = instancebaseline.get(&class_id).ok_or(required!())?;

                    dbg!(class);
                    dbg!(baseline_value);

                    // TODO!
                    unimplemented!()
                }
                op => {
                    dbg!(op);
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
        let file = File::open("./fixtures/6911306644_1806469309.dem")?;
        let mut parser = Parser::new(file)?;
        while !parser.process_next_msg()? {}
        Ok(())
    }

    #[test]
    fn parse_file_info() -> Result<()> {
        let file = File::open("./fixtures/6911306644_1806469309.dem")?;
        let mut parser = Parser::new(file)?;
        let _file_info = parser.read_file_info()?;
        Ok(())
    }
}
