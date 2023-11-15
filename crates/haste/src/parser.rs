use crate::{
    bitbuf::BitReader,
    demofile::{CmdHeader, DemoFile},
    dota2protos,
    entities::{self, Entities},
    entityclasses::EntityClasses,
    flattenedserializers::FlattenedSerializers,
    instancebaseline::{InstanceBaseline, INSTANCE_BASELINE_TABLE_NAME},
    prost::Message,
    stringtables::StringTables,
};
use std::{
    io::{Read, Seek},
    marker::PhantomData,
};

pub type Error = Box<dyn std::error::Error>;

pub type Result<T> = std::result::Result<T, Error>;

pub trait Visitor {
    #[allow(unused_variables)]
    fn visit_entity(
        &self,
        update_flags: usize,
        update_type: entities::UpdateType,
        entity: &entities::Entity,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn visit_cmd(&self, cmd_header: &CmdHeader, data: &[u8]) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn visit_packet(&self, packet_type: u32, data: &[u8]) -> Result<()> {
        Ok(())
    }
}

pub struct Parser<'p, R: Read + Seek, V: Visitor + 'p> {
    demo_file: DemoFile<R>,
    buf: Vec<u8>,
    string_tables: StringTables,
    instance_baseline: InstanceBaseline,
    flattened_serializers: Option<FlattenedSerializers>,
    entity_classes: Option<EntityClasses>,
    entities: Entities,
    visitor: V,
    _visitor_phantom: PhantomData<&'p ()>,
}

impl<'p, R: Read + Seek, V: Visitor> Parser<'p, R, V> {
    pub fn from_reader(rdr: R, visitor: V) -> Result<Self> {
        let mut demo_file = DemoFile::from_reader(rdr);
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
            _visitor_phantom: PhantomData,
        })
    }

    // TODO: implement interator that would combine read_cmd_header + handle_cmd

    pub fn parse_all(&mut self) -> Result<()> {
        loop {
            match self.demo_file.read_cmd_header() {
                Ok(cmd_header) => self.handle_cmd(cmd_header)?,
                Err(err) => {
                    if self.demo_file.is_eof().unwrap_or(false) {
                        return Ok(());
                    }
                    return Err(Error::from(err));
                }
            }
        }
    }

    pub fn parse_to_tick(&mut self, _tick: u32) -> Result<()> {
        unimplemented!()
    }

    fn handle_cmd(&mut self, cmd_header: CmdHeader) -> Result<()> {
        let data = self.demo_file.read_cmd(&cmd_header, &mut self.buf)?;
        self.visitor.visit_cmd(&cmd_header, data)?;

        use dota2protos::EDemoCommands;
        match cmd_header.command {
            EDemoCommands::DemPacket | EDemoCommands::DemSignonPacket => {
                let cmd = dota2protos::CDemoPacket::decode(data)?;
                self.handle_packet(cmd)?;
            }

            EDemoCommands::DemSendTables => {
                let cmd = dota2protos::CDemoSendTables::decode(data)?;
                self.flattened_serializers = Some(FlattenedSerializers::parse(cmd)?);
            }

            EDemoCommands::DemClassInfo => {
                let cmd = dota2protos::CDemoClassInfo::decode(data)?;
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
                // ignore
            }
        }

        Ok(())
    }

    fn handle_packet(&mut self, proto: dota2protos::CDemoPacket) -> Result<()> {
        let proto_data = proto.data.unwrap_or_default();
        let mut br = BitReader::new(&proto_data);
        while br.get_num_bits_left() > 8 {
            let command = br.read_ubitvar()?;
            let size = br.read_uvarint32()? as usize;

            let buf = &mut self.buf[..size];
            br.read_bytes(buf)?;
            let buf: &_ = buf;

            self.visitor.visit_packet(command, buf)?;

            use dota2protos::SvcMessages;
            match command {
                c if c == SvcMessages::SvcCreateStringTable as u32 => {
                    let msg = dota2protos::CsvcMsgCreateStringTable::decode(buf)?;
                    self.handle_svc_create_string_table(msg)?;
                }

                c if c == SvcMessages::SvcUpdateStringTable as u32 => {
                    let msg = dota2protos::CsvcMsgUpdateStringTable::decode(buf)?;
                    self.handle_svc_update_string_table(msg)?;
                }

                c if c == SvcMessages::SvcPacketEntities as u32 => {
                    let msg = dota2protos::CsvcMsgPacketEntities::decode(buf)?;
                    self.handle_svc_packet_entities(msg)?;
                }

                _ => {}
            }
        }

        Ok(())
    }

    fn handle_svc_create_string_table(
        &mut self,
        msg: dota2protos::CsvcMsgCreateStringTable,
    ) -> Result<()> {
        let string_table = self.string_tables.create_string_table_mut(
            msg.name(),
            msg.user_data_fixed_size(),
            msg.user_data_size(),
            msg.user_data_size_bits(),
            msg.flags(),
            msg.using_varint_bitcounts(),
        )?;

        let string_data = if msg.data_compressed() {
            let sd = msg.string_data();
            let decompress_len = snap::raw::decompress_len(sd)?;
            snap::raw::Decoder::new().decompress(sd, &mut self.buf)?;
            &self.buf[..decompress_len]
        } else {
            msg.string_data()
        };
        string_table.parse_update(&mut BitReader::new(string_data), msg.num_entries())?;

        if string_table.name.as_ref().eq(INSTANCE_BASELINE_TABLE_NAME) {
            if let Some(entity_classes) = self.entity_classes.as_ref() {
                self.instance_baseline
                    .update(string_table, entity_classes.classes)?;
            }
        }

        Ok(())
    }

    fn handle_svc_update_string_table(
        &mut self,
        msg: dota2protos::CsvcMsgUpdateStringTable,
    ) -> Result<()> {
        let string_table = self
            .string_tables
            .get_table_mut(msg.table_id.expect("table id") as usize)
            .expect("table");

        string_table.parse_update(
            &mut BitReader::new(msg.string_data()),
            msg.num_changed_entries(),
        )?;

        if string_table.name.as_ref().eq(INSTANCE_BASELINE_TABLE_NAME) {
            if let Some(entity_classes) = self.entity_classes.as_ref() {
                self.instance_baseline
                    .update(string_table, entity_classes.classes)?;
            }
        }

        Ok(())
    }

    // NOTE: handle_msg_packet_entities is partially based on
    // ReadPacketEntities in engine/client.cpp
    fn handle_svc_packet_entities(
        &mut self,
        msg: dota2protos::CsvcMsgPacketEntities,
    ) -> Result<()> {
        use entities::*;

        // SAFETY: safety here can only be guaranteed by the fact that entity
        // classes and flattened serializers become available before packet
        // entities.
        let entity_classes = unsafe { self.entity_classes.as_ref().unwrap_unchecked() };
        let flattened_serializers =
            unsafe { self.flattened_serializers.as_ref().unwrap_unchecked() };
        let instance_baseline = &self.instance_baseline;

        let entity_data = msg.entity_data.expect("entity data");
        let mut br = BitReader::new(&entity_data);

        let mut entidx: i32 = -1;
        for _ in (0..msg.updated_entries.expect("updated entries")).rev() {
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
                    self.visitor
                        .visit_entity(update_flags, update_type, entity)?;
                }
                UpdateType::LeavePVS => {
                    if (update_flags & FHDR_DELETE) != 0 {
                        let entity = self.entities.handle_delete(entidx);
                        self.visitor
                            .visit_entity(update_flags, update_type, &entity)?;
                    }
                }
                UpdateType::DeltaEnt => {
                    let entity = self.entities.handle_update(entidx, &mut br)?;
                    self.visitor
                        .visit_entity(update_flags, update_type, entity)?;
                }
            }
        }

        Ok(())
    }
}
