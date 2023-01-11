use crate::{
    protos, read_varu32, BitBuf, EntityClasses, Error, FlattenedSerializers, StringTables,
};
use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use derivative::Derivative;
use prost::Message;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

const DEMO_HEADER_ID: [u8; 8] = *b"PBDEMS2\0";

pub trait Visitor {
    fn on_cdemo_stop(&self) {}
    fn on_cdemo_file_header(&self, _v: protos::CDemoFileHeader) {}
}

impl Visitor for () {}

pub struct DemFile<Vi: Visitor> {
    file: File,
    file_info_offset: i32,
    message_data: Vec<u8>,
    message_snappy_data: Vec<u8>,
    packet_data: Vec<u8>,
    visitor: Vi,
    flattened_serializers: Option<FlattenedSerializers>,
    entity_classes: Option<EntityClasses>,
    string_tables: StringTables,
}

impl<Vi> DemFile<Vi>
where
    Vi: Visitor,
{
    pub fn open<P: AsRef<Path>>(path: P, visitor: Vi) -> Result<Self> {
        let file = File::open(path)?;

        let header = DemHeader::read(&file)?;
        if header.demo_file_stamp != DEMO_HEADER_ID {
            return Err(anyhow!(Error::InvalidHeaderId {
                want: DEMO_HEADER_ID,
                got: header.demo_file_stamp,
            }));
        }

        Ok(Self {
            file,
            file_info_offset: header.file_info_offset,
            message_data: vec![0; 256000],
            message_snappy_data: vec![0; 512000],
            packet_data: vec![0; 512000],
            visitor,
            flattened_serializers: None,
            entity_classes: None,
            string_tables: StringTables::new(),
        })
    }

    pub fn get_file_info(&mut self) -> Result<protos::CDemoFileInfo> {
        if self.file_info_offset <= 0 {
            return Err(anyhow!(Error::InvalidFileInfoOffset));
        }

        let offset = self.file.stream_position()?;
        self.file
            .seek(SeekFrom::Start(self.file_info_offset as u64))?;

        let message = DemMessage::read(
            &self.file,
            &mut self.message_data,
            &mut self.message_snappy_data,
        )?;
        let file_info = protos::CDemoFileInfo::decode(message.data)?;

        self.file.seek(SeekFrom::Start(offset))?;

        Ok(file_info)
    }

    pub fn parse(&mut self) -> Result<()> {
        loop {
            let message = DemMessage::read(
                &self.file,
                &mut self.message_data,
                &mut self.message_snappy_data,
            )?;

            use protos::EDemoCommands::*;
            match message.command {
                DemStop => {
                    self.visitor.on_cdemo_stop();
                    break;
                }
                DemFileHeader => {
                    let file_header = protos::CDemoFileHeader::decode(message.data)?;
                    self.visitor.on_cdemo_file_header(file_header);
                }
                DemSyncTick => {
                    // ignore
                }
                DemSendTables => {
                    self.flattened_serializers = Some(FlattenedSerializers::new(message.data)?);
                }
                DemClassInfo => {
                    self.entity_classes = Some(EntityClasses::new(message.data)?);
                }
                DemStringTables => {
                    // let string_tables = protos::CDemoStringTables::decode(message.data)?;
                    // quote from dotabuff/manta:
                    // > These appear to be periodic state dumps and appear every 1800 outer ticks.
                    // they ignore this.
                }
                DemPacket | DemSignonPacket => {
                    let packet = protos::CDemoPacket::decode(message.data)?;
                    if let Some(data) = packet.data {
                        self.handle_packet(&data)?;
                    }
                }
                DemFullPacket => {
                    let full_packet = protos::CDemoFullPacket::decode(message.data)?;
                    // NOTE: we're not interested in string tables, see comment in DemStringTables arm
                    if let Some(packet) = full_packet.packet {
                        if let Some(data) = packet.data {
                            self.handle_packet(&data)?;
                        }
                    }
                }
                _ => {
                    dbg!(&message);
                    unimplemented!()
                }
            }
        }
        Ok(())
    }

    fn handle_packet(&mut self, data: &[u8]) -> Result<()> {
        let mut bitbuf = BitBuf::new(data);

        while !bitbuf.is_empty() {
            let packet = DemPacket::read(&mut bitbuf, &mut self.packet_data)?;

            use protos::SvcMessages::*;
            match packet.command {
                DemPacketCommand::Svc(v) if v == SvcCreateStringTable => {
                    let proto = protos::CsvcMsgCreateStringTable::decode(packet.data)?;
                    self.string_tables.insert(proto)?;
                }
                DemPacketCommand::Svc(v) if v == SvcUpdateStringTable => {
                    unimplemented!()
                }
                DemPacketCommand::Svc(v) if v == SvcPacketEntities => {
                    let packet_entities = protos::CsvcMsgPacketEntities::decode(packet.data)?;

                    let entity_data = &packet_entities.entity_data.expect("some entity data");
                    let mut bitbuf = BitBuf::new(entity_data);

                    let mut updated_entries = packet_entities
                        .updated_entries
                        .expect("some updated entries");
                    let mut index: i32 = -1;

                    while updated_entries > 0 {
                        updated_entries -= 1;
                        index += bitbuf.read_ubitvar()? as i32 + 1;

                        let entity_command = EntityCommand::from(bitbuf.read(2)?);
                        match entity_command {
                            EntityCommand::Create => {
                                let class_id = bitbuf.read(
                                    self.entity_classes
                                        .as_ref()
                                        .expect("some entity classes")
                                        .bits(),
                                )?;
                                bitbuf.read(17)?; // serial
                                bitbuf.read_varu32()?; // unknown
                            }
                            _ => {}
                        }
                    }
                }
                _ => {
                    // dbg!(packet.command);
                }
            }
        }

        Ok(())
    }
}

#[derive(Default, Debug)]
struct DemHeader {
    demo_file_stamp: [u8; 8], // PROTODEMO_HEADER_ID
    file_info_offset: i32,
    unknown: i32,
}

impl DemHeader {
    fn read(mut r: impl Read) -> Result<Self> {
        let mut header = DemHeader::default();
        r.read_exact(&mut header.demo_file_stamp)?;
        header.file_info_offset = r.read_i32::<LittleEndian>()?;
        header.unknown = r.read_i32::<LittleEndian>()?;
        Ok(header)
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct DemMessage<'dst> {
    command: protos::EDemoCommands,
    tick: u32,
    size: u32,
    #[derivative(Debug = "ignore")]
    data: &'dst [u8],
}

impl<'dst> DemMessage<'dst> {
    fn read(
        mut r: impl Read,
        dst_data: &'dst mut [u8],
        dst_snappy_data: &'dst mut [u8],
    ) -> Result<Self> {
        let mut command = read_varu32(&mut r)?;
        let tick = read_varu32(&mut r)?;
        let mut size = read_varu32(&mut r)?;

        let mut data = &mut dst_data[0..size as usize];
        r.read_exact(data)?;

        // TODO: don't decompress data here; decompress it only if it's needed to be
        // decompressed (if visitor wants it (/subscribed for it)).
        let dem_is_compressed = protos::EDemoCommands::DemIsCompressed as u32;
        if command & dem_is_compressed == dem_is_compressed {
            let decompressed_size = snap::raw::decompress_len(data)?;
            let decompressed_data = &mut dst_snappy_data[0..decompressed_size];
            snap::raw::Decoder::new().decompress(data, decompressed_data)?;

            command &= !dem_is_compressed;
            size = decompressed_size as u32;
            data = decompressed_data;
        }

        // NOTE: we can't convert command to enum earlier because we need to
        // extract IsCompressed flag first (with it bundled in command is
        // invalid).
        let command = protos::EDemoCommands::from_i32(command as i32)
            .ok_or(Error::UnknownDemoCommand(command))?;

        Ok(DemMessage {
            command,
            tick,
            size,
            data,
        })
    }
}

#[derive(Debug, PartialEq)]
enum DemPacketCommand {
    Net(protos::NetMessages),
    Svc(protos::SvcMessages),
    EBaseUser(protos::EBaseUserMessages),
    EDotaUserMessages(protos::EDotaUserMessages),
    EBaseGameEvents(protos::EBaseGameEvents),
    EteProtobufIds(protos::EteProtobufIds),
    Unknown(i32),
}

impl From<u32> for DemPacketCommand {
    fn from(u: u32) -> Self {
        let i = u as i32;
        if let Some(v) = protos::NetMessages::from_i32(i) {
            Self::Net(v)
        } else if let Some(v) = protos::SvcMessages::from_i32(i) {
            Self::Svc(v)
        } else if let Some(v) = protos::EBaseUserMessages::from_i32(i) {
            Self::EBaseUser(v)
        } else if let Some(v) = protos::EDotaUserMessages::from_i32(i) {
            Self::EDotaUserMessages(v)
        } else if let Some(v) = protos::EBaseGameEvents::from_i32(i) {
            Self::EBaseGameEvents(v)
        } else if let Some(v) = protos::EteProtobufIds::from_i32(i) {
            Self::EteProtobufIds(v)
        } else {
            // TODO: return an error
            Self::Unknown(i)
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct DemPacket<'dst> {
    command: DemPacketCommand,
    size: u32,
    #[derivative(Debug = "ignore")]
    data: &'dst [u8],
}

impl<'dst> DemPacket<'dst> {
    fn read<'src>(bitbuf: &mut BitBuf, dst_data: &'dst mut [u8]) -> Result<Self> {
        let command = DemPacketCommand::from(bitbuf.read_ubitvar()?);
        let size = bitbuf.read_varu32()?;

        let data = &mut dst_data[0..size as usize];
        bitbuf.read_bytes(data)?;

        Ok(Self {
            command,
            size,
            data,
        })
    }
}

#[derive(Debug)]
enum EntityCommand {
    Update = 0,
    Leave,
    Create,
    Delete,
}

impl From<u32> for EntityCommand {
    fn from(value: u32) -> Self {
        use EntityCommand::*;
        if value & 0x01 == 0 {
            if value & 0x02 != 0 {
                return Create;
            } else {
                return Update;
            }
        } else {
            if value & 0x02 != 0 {
                return Delete;
            } else {
                return Leave;
            }
        }
    }
}

// struct SerializedEntity {}

// impl SerializedEntity {
//     fn parse(bitbuf: &mut BitBuf) -> Result<()> {
//         Ok(())
//     }
// }
