use crate::{
    protos, varint::VarInt, BitRead, EntityClasses, Error, FlattenedSerializers, StringTables,
};
use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use derivative::Derivative;
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use prost::Message;
use std::{
    alloc::{Allocator, Global},
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

// NOTE: A has a Clone trait bound, that's because we expect A to be a reference
// (https://github.com/rust-lang/rust/pull/98178)
pub struct DemFile<Vi: Visitor, A: Allocator + Clone = Global> {
    file: File,
    file_info_offset: i32,
    message_data: Vec<u8>,
    message_snappy_data: Vec<u8>,
    packet_data: Vec<u8>,
    visitor: Vi,
    flattened_serializers: Option<FlattenedSerializers>,

    entity_classes: Option<EntityClasses<A>>,
    string_tables: StringTables<A>,
    class_baselines: Option<HashMap<i32, Vec<u8, A>, DefaultHashBuilder, A>>,
    alloc: A,
}

impl<Vi: Visitor> DemFile<Vi, Global> {
    pub fn open<P: AsRef<Path>>(path: P, visitor: Vi) -> Result<Self> {
        Self::open_in(path, visitor, Global)
    }
}

impl<Vi: Visitor, A: Allocator + Clone> DemFile<Vi, A> {
    pub fn open_in<P: AsRef<Path>>(path: P, visitor: Vi, alloc: A) -> Result<Self> {
        let file = File::open(path)?;

        let header = DemHeader::read(&file)?;
        if header.demo_file_stamp != DEMO_HEADER_ID {
            return Err(anyhow!(Error::InvalidDemHeader {
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
            string_tables: StringTables::new_in(alloc.clone()),
            class_baselines: None,
            alloc,
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
                    self.entity_classes =
                        Some(EntityClasses::new_in(message.data, self.alloc.clone())?);
                }
                DemStringTables => {
                    // quote from dotabuff/manta (tldr they ignore this):
                    // > These appear to be periodic state dumps and appear every 1800 outer ticks.
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
        let mut br = BitRead::new(data);

        while !br.is_empty() {
            let packet = DemPacket::read(&mut br, &mut self.packet_data)?;

            use protos::SvcMessages::*;
            match packet.command {
                DemPacketCommand::Svc(v) if v == SvcCreateStringTable => {
                    let proto = protos::CsvcMsgCreateStringTable::decode(packet.data)?;
                    // NOTE: we are only interested in `instancebaseline` table
                    if proto.name.as_ref().expect("some name") == "instancebaseline" {
                        let string_table = self.string_tables.create(proto)?;
                        let mut class_baselines: HashMap<i32, Vec<u8, A>, DefaultHashBuilder, A> =
                            HashMap::with_capacity_in(string_table.len(), self.alloc.clone());
                        string_table.iter().for_each(|(_, v)| {
                            let class_id = v
                                .key
                                .as_ref()
                                .expect("instancebaseline key")
                                .parse::<i32>()
                                .expect("valid class id");
                            class_baselines.insert(
                                class_id,
                                v.value.as_ref().expect("instancebaseline value").clone(),
                            );
                        });
                        self.class_baselines = Some(class_baselines);
                    }
                }
                DemPacketCommand::Svc(v) if v == SvcUpdateStringTable => {
                    unimplemented!()
                    // TODO: decompose StringTables.insert into create and parse
                    // fns, create update fn.
                }
                DemPacketCommand::Svc(v) if v == SvcPacketEntities => {
                    let packet_entities = protos::CsvcMsgPacketEntities::decode(packet.data)?;
                    let entity_data = &packet_entities.entity_data.expect("some entity data");
                    let mut br = BitRead::new(entity_data);

                    let updated_entries = packet_entities
                        .updated_entries
                        .expect("some updated entries");
                    let mut index: i32 = -1;

                    let baselines = self
                        .string_tables
                        .get("instancebaseline")
                        .expect("some instancebaseline table");

                    for _ in 0..updated_entries as usize {
                        index += br.read_ubitvar()? as i32 + 1;

                        let entity_command = EntityCommand::from(br.read(2)?);
                        match entity_command {
                            EntityCommand::Create => {
                                let entity_classes =
                                    self.entity_classes.as_ref().expect("some entity classes");
                                // let mut keys =
                                //     entity_classes.keys().into_iter().collect::<Vec<&i32>>();
                                // keys.sort();
                                // dbg!(keys);

                                let class_id = br.read(entity_classes.bits())? as i32;
                                let serial = br.read(17)?; // serial
                                br.read_varu32()?; // unknown
                                dbg!(class_id, serial);

                                // let mut keys = baselines.keys().into_iter().collect::<Vec<&i32>>();
                                // keys.sort();
                                // dbg!(keys);

                                // TODO: we're not finding a baseline that we're looking for
                                let baseline = self
                                    .class_baselines
                                    .as_ref()
                                    .expect("class baselines table")
                                    .get(&class_id)
                                    .expect("class baseline value");

                                let class = entity_classes.get(&class_id).expect("some class");

                                let mut baseline_br = BitRead::new(baseline);
                                let mut opid = 0;
                                for _ in 0..17 {
                                    opid = (opid << 1) | baseline_br.read(1)?;
                                    dbg!(opid);
                                }
                                unimplemented!();

                                // fn manta_read_field_paths(br: BitRead) {}
                                // manta_read_field_paths(BitRead::new(
                                //     baseline.value.as_ref().expect("some value"),
                                // ));

                                // if let Some(ste) = baselines.get(&class_id) {
                                //     if let Some(value) = &ste.value {
                                //         let br = BitRead::new(value);
                                //         // TODO: parse
                                //     }
                                // }

                                // TODO: parse
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
        let mut command = r.read_varu32()?;
        let tick = r.read_varu32()?;
        let mut size = r.read_varu32()?;

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
    fn read<'src>(br: &mut BitRead, dst_data: &'dst mut [u8]) -> Result<Self> {
        let command = DemPacketCommand::from(br.read_ubitvar()?);
        let size = br.read_varu32()?;

        let data = &mut dst_data[0..size as usize];
        br.read_bytes(data)?;

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
//     fn parse(br: &mut BitRead) -> Result<()> {
//         Ok(())
//     }
// }
