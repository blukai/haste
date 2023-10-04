use crate::{
    bitbuf::BitReader,
    demofile::{CmdHeader, DemoFile},
    entities::{self, Entities},
    entityclasses::EntityClasses,
    flattenedserializers::FlattenedSerializers,
    instancebaseline::{InstanceBaseline, INSTANCE_BASELINE_TABLE_NAME},
    stringtables::StringTables,
};
use prost::Message;
use std::io::{Read, Seek, SeekFrom};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub trait Visitor {
    fn visit_entity(
        &self,
        update_flags: usize,
        update_type: entities::UpdateType,
        entity: &entities::Entity,
    );
}

pub struct Parser<R: Read + Seek, V: Visitor> {
    demo_file: DemoFile<R>,
    buf: Vec<u8>,
    string_tables: StringTables,
    instance_baseline: InstanceBaseline,
    flattened_serializers: Option<FlattenedSerializers>,
    entity_classes: Option<EntityClasses>,
    entities: Entities,
    visitor: V,
}

impl<R: Read + Seek, V: Visitor> Parser<R, V> {
    pub fn from_reader(rdr: R, visitor: V) -> Result<Self> {
        let mut demo_file = DemoFile::from_reader(rdr);
        // TODO: validate demo header
        let _demo_header = demo_file.read_demo_header()?;

        Ok(Self {
            demo_file,
            // 2mb
            buf: vec![0; 1024 * 1024 * 2],
            string_tables: StringTables::default(),
            instance_baseline: InstanceBaseline::default(),
            flattened_serializers: None,
            entity_classes: None,
            entities: Entities::default(),
            visitor,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let cmd_header = self.demo_file.read_cmd_header()?;
            if cmd_header.command == dota2protos::EDemoCommands::DemStop {
                break;
            }
            self.handle_cmd(cmd_header)?;
        }
        Ok(())
    }

    fn handle_cmd(&mut self, cmd_header: CmdHeader) -> Result<()> {
        use dota2protos::EDemoCommands;
        match cmd_header.command {
            EDemoCommands::DemPacket | EDemoCommands::DemSignonPacket => {
                let cmd = self
                    .demo_file
                    .read_cmd::<dota2protos::CDemoPacket>(&cmd_header, &mut self.buf)?;
                self.handle_cmd_packet(cmd)?;
            }

            EDemoCommands::DemSendTables => {
                let cmd = self
                    .demo_file
                    .read_cmd::<dota2protos::CDemoSendTables>(&cmd_header, &mut self.buf)?;
                self.flattened_serializers = Some(FlattenedSerializers::parse(cmd)?);
            }

            EDemoCommands::DemClassInfo => {
                let cmd = self
                    .demo_file
                    .read_cmd::<dota2protos::CDemoClassInfo>(&cmd_header, &mut self.buf)?;
                self.entity_classes = Some(EntityClasses::parse(cmd));

                // NOTE: DemClassInfo message becomes available after
                // SvcCreateStringTable(which has instancebaselines). to know
                // how long vec that will contain instancebaseline values needs
                // to be (to allocate precicely how much we need) we need to
                // wait for DemClassInfos.
                if let Some(string_table) =
                    self.string_tables.find_table(INSTANCE_BASELINE_TABLE_NAME)
                {
                    // SAFETY: entity_classes value was assigned right above ^.
                    let entity_classes = unsafe { self.entity_classes.as_ref().unwrap_unchecked() };
                    self.instance_baseline
                        .update(string_table, entity_classes.classes)?;
                }
            }

            _ => {
                self.demo_file
                    .seek(SeekFrom::Current(cmd_header.size as i64))?;
            }
        }

        Ok(())
    }

    fn handle_cmd_packet(&mut self, proto: dota2protos::CDemoPacket) -> Result<()> {
        let data = proto.data.expect("demo packet data");
        let mut br = BitReader::new(&data);

        while br.get_num_bits_left() > 8 {
            let command = br.read_ubitvar()?;
            let size = br.read_uvarint32()? as usize;

            let buf = &mut self.buf[..size];
            br.read_bytes(buf)?;
            let data: &_ = buf;

            use dota2protos::SvcMessages;
            match command {
                c if c == SvcMessages::SvcCreateStringTable as u32 => {
                    let svcmsg = dota2protos::CsvcMsgCreateStringTable::decode(data)?;
                    self.handle_svcmsg_create_string_table(svcmsg)?;
                }

                c if c == SvcMessages::SvcUpdateStringTable as u32 => {
                    let svcmsg = dota2protos::CsvcMsgUpdateStringTable::decode(data)?;
                    self.handle_svcmsg_update_string_table(svcmsg)?;
                }

                c if c == SvcMessages::SvcPacketEntities as u32 => {
                    let svcmsg = dota2protos::CsvcMsgPacketEntities::decode(data)?;
                    self.handle_svcmsg_packet_entities(svcmsg)?;
                }

                _ => {}
            }
        }

        Ok(())
    }

    fn handle_svcmsg_create_string_table(
        &mut self,
        svcmsg: dota2protos::CsvcMsgCreateStringTable,
    ) -> Result<()> {
        let string_table = self.string_tables.create_string_table_mut(
            svcmsg.name(),
            svcmsg.user_data_fixed_size(),
            svcmsg.user_data_size(),
            svcmsg.user_data_size_bits(),
            svcmsg.flags(),
            svcmsg.using_varint_bitcounts(),
        )?;

        let string_data = if svcmsg.data_compressed() {
            let sd = svcmsg.string_data();
            let decompress_len = snap::raw::decompress_len(sd)?;
            snap::raw::Decoder::new().decompress(sd, &mut self.buf)?;
            &self.buf[..decompress_len]
        } else {
            svcmsg.string_data()
        };
        string_table.parse_update(&mut BitReader::new(string_data), svcmsg.num_entries())?;

        if string_table.name.as_ref().eq(INSTANCE_BASELINE_TABLE_NAME) {
            if let Some(entity_classes) = self.entity_classes.as_ref() {
                self.instance_baseline
                    .update(string_table, entity_classes.classes)?;
            }
        }

        Ok(())
    }

    fn handle_svcmsg_update_string_table(
        &mut self,
        svcmsg: dota2protos::CsvcMsgUpdateStringTable,
    ) -> Result<()> {
        let string_table = self
            .string_tables
            .get_table_mut(svcmsg.table_id.expect("table id") as usize)
            .expect("table");

        string_table.parse_update(
            &mut BitReader::new(svcmsg.string_data()),
            svcmsg.num_changed_entries(),
        )?;

        if string_table.name.as_ref().eq(INSTANCE_BASELINE_TABLE_NAME) {
            if let Some(entity_classes) = self.entity_classes.as_ref() {
                self.instance_baseline
                    .update(string_table, entity_classes.classes)?;
            }
        }

        Ok(())
    }

    // NOTE: handle_svcmsg_packet_entities is partially based on
    // ReadPacketEntities in engine/client.cpp
    fn handle_svcmsg_packet_entities(
        &mut self,
        svcmsg: dota2protos::CsvcMsgPacketEntities,
    ) -> Result<()> {
        use entities::*;

        // SAFETY: safety here can only be guaranteed by the fact that entity
        // classes and flattened serializers become available before packet
        // entities.
        let entity_classes = unsafe { self.entity_classes.as_ref().unwrap_unchecked() };
        let flattened_serializers =
            unsafe { self.flattened_serializers.as_ref().unwrap_unchecked() };
        let instance_baseline = &self.instance_baseline;

        let entity_data = svcmsg.entity_data.expect("entity data");
        let mut br = BitReader::new(&entity_data);

        let mut entidx: i32 = -1;
        for _ in (0..svcmsg.updated_entries.expect("updated entries")).rev() {
            entidx += br.read_ubitvar()? as i32 + 1;

            let update_flags = parse_delta_header(&mut br)?;
            let update_type = determine_update_type(update_flags);

            match update_type {
                UpdateType::EnterPVS => {
                    let entity = self.entities.handle_create(
                        entidx,
                        &mut br,
                        entity_classes,
                        instance_baseline,
                        flattened_serializers,
                    )?;
                    self.visitor.visit_entity(update_flags, update_type, entity);
                }
                UpdateType::LeavePVS => {
                    if (update_flags & FHDR_DELETE) != 0 {
                        let entity = self.entities.handle_delete(entidx);
                        self.visitor
                            .visit_entity(update_flags, update_type, &entity);
                    }
                }
                UpdateType::DeltaEnt => {
                    let entity = self.entities.handle_update(entidx, &mut br)?;
                    self.visitor.visit_entity(update_flags, update_type, entity);
                }
            }
        }

        Ok(())
    }
}
