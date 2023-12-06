use crate::{
    bitbuf::BitReader,
    demofile::{CmdHeader, DemoFile},
    entities::{self, Entities},
    entityclasses::EntityClasses,
    flattenedserializers::FlattenedSerializers,
    haste_dota2_protos::{self, prost::Message, EDemoCommands},
    instancebaseline::{InstanceBaseline, INSTANCE_BASELINE_TABLE_NAME},
    stringtables::StringTables,
};
use std::{
    io::{Read, Seek, SeekFrom},
    marker::PhantomData,
    ops::ControlFlow,
};

// as can be observed when dumping commands. also as specified in clarity
// (src/main/java/skadistats/clarity/model/engine/AbstractDotaEngineType.java)
// and documented in manta (string_table.go).
const FULL_PACKET_INTERVAL: i32 = 1800;

pub type Error = Box<dyn std::error::Error>;

pub type Result<T> = std::result::Result<T, Error>;

pub trait Visitor {
    #[allow(unused_variables)]
    fn visit_entity(
        &self,
        update_flags: usize,
        update_type: entities::UpdateType,
        // TODO: include updated fields (list of field paths?)
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
    starting_position: u64,
    demo_file: DemoFile<R>,
    buf: Vec<u8>,
    string_tables: StringTables,
    instance_baseline: InstanceBaseline,
    flattened_serializers: Option<FlattenedSerializers>,
    entity_classes: Option<EntityClasses>,
    entities: Entities,
    tick: i32,
    visitor: V,
    _p: PhantomData<&'p ()>,
}

impl<'p, R: Read + Seek, V: Visitor> Parser<'p, R, V> {
    pub fn from_reader(rdr: R, visitor: V) -> Result<Self> {
        let mut demo_file = DemoFile::from_reader(rdr);
        let _demo_header = demo_file.read_demo_header()?;

        Ok(Self {
            starting_position: demo_file.stream_position()?,
            demo_file,
            buf: vec![0; 1024 * 1024 * 2], // 2mb
            string_tables: StringTables::default(),
            instance_baseline: InstanceBaseline::default(),
            flattened_serializers: None,
            entity_classes: None,
            entities: Entities::default(),
            tick: -1,
            visitor,
            _p: PhantomData,
        })
    }

    fn run<F>(&mut self, mut handler: F) -> Result<()>
    where
        F: FnMut(&mut Self) -> Result<ControlFlow<()>>,
    {
        loop {
            match handler(self) {
                Ok(cf) => match cf {
                    ControlFlow::Break(_) => return Ok(()),
                    ControlFlow::Continue(_) => {}
                },
                Err(err) => {
                    if self.demo_file.is_eof().unwrap_or_default() {
                        return Ok(());
                    }
                    return Err(Error::from(err));
                }
            }
        }
    }

    // TODO: rename parse_all to run_to_end
    pub fn parse_all(&mut self) -> Result<()> {
        self.run(|notnotself| {
            let cmd_header = notnotself.demo_file.read_cmd_header()?;
            notnotself.tick = cmd_header.tick;
            notnotself.handle_cmd(cmd_header)?;
            Ok(ControlFlow::Continue(()))
        })
    }

    // TODO: rename parse_to_tick to run_to_tick
    pub fn parse_to_tick(&mut self, target_tick: i32) -> Result<()> {
        // TODO: do not allow tick to be less then 0

        // TODO: do not allow tick to be greater then total ticks

        // TODO: do not clear if seeking forward and there's no full packet on
        // the way to the wanted tick / if target tick is closer then full
        // packet interval

        self.demo_file
            .seek(SeekFrom::Start(self.starting_position))?;
        self.string_tables.clear();
        self.instance_baseline.clear();
        self.entities.clear();

        // NOTE: EDemoCommands::DemSyncTick is the last command with 4294967295
        // tick (normlized to -1). last "initialization" command.
        let mut did_reach_first_sync_tick = false;

        // NOTE: EDemoCommands::DemFullPacket contains snapshot of everything
        let mut did_reach_last_full_packet = false;

        self.run(|notnotself| {
            let cmd_header = notnotself.demo_file.read_cmd_header()?;
            if cmd_header.tick > target_tick {
                notnotself.demo_file.backup(&cmd_header)?;
                return Ok(ControlFlow::Break(()));
            }

            notnotself.tick = cmd_header.tick;

            // init string tables, flattened serializers and entity classes
            if !did_reach_first_sync_tick {
                did_reach_first_sync_tick = cmd_header.command == EDemoCommands::DemSyncTick;
                notnotself.handle_cmd(cmd_header)?;
                return Ok(ControlFlow::Continue(()));
            }

            // skip uptil full packet
            if !did_reach_last_full_packet {
                let target_tick_distance = target_tick - notnotself.tick;
                // TODO: what if there's no full packet? maybe dem file is
                // corrupted or something
                let is_last_full_packet_close = target_tick_distance < FULL_PACKET_INTERVAL + 100;
                if !is_last_full_packet_close {
                    notnotself.demo_file.skip(&cmd_header)?;
                    return Ok(ControlFlow::Continue(()));
                }

                let is_full_packet = cmd_header.command == EDemoCommands::DemFullPacket;
                if !is_full_packet {
                    notnotself.demo_file.skip(&cmd_header)?;
                    return Ok(ControlFlow::Continue(()));
                }

                debug_assert!(is_full_packet);
                did_reach_last_full_packet = true;

                let data = notnotself
                    .demo_file
                    .read_cmd(&cmd_header, &mut notnotself.buf)?;
                notnotself.visitor.visit_cmd(&cmd_header, data)?;

                let cmd = haste_dota2_protos::CDemoFullPacket::decode(data)?;
                notnotself.handle_cmd_full_packet(cmd)?;

                return Ok(ControlFlow::Continue(()));
            }

            notnotself.handle_cmd(cmd_header)?;
            Ok(ControlFlow::Continue(()))
        })
    }

    // important initialization messages:
    // 1. DemSignonPacket (SvcCreateStringTable)
    // 2. DemSendTables (flattened serializers; never update)
    // 3. DemClassInfo (never update)
    fn handle_cmd(&mut self, cmd_header: CmdHeader) -> Result<()> {
        let data = self.demo_file.read_cmd(&cmd_header, &mut self.buf)?;
        self.visitor.visit_cmd(&cmd_header, data)?;

        match cmd_header.command {
            EDemoCommands::DemPacket | EDemoCommands::DemSignonPacket => {
                let cmd = haste_dota2_protos::CDemoPacket::decode(data)?;
                self.handle_cmd_packet(cmd)?;
            }

            EDemoCommands::DemSendTables => {
                // NOTE: this check exists because seeking exists, there's no
                // need to re-parse flattened serializers
                if self.flattened_serializers.is_some() {
                    return Ok(());
                }

                let cmd = haste_dota2_protos::CDemoSendTables::decode(data)?;
                self.flattened_serializers = Some(FlattenedSerializers::parse(cmd)?);
            }

            EDemoCommands::DemClassInfo => {
                // NOTE: this check exists because seeking exists, there's no
                // need to re-parse entity classes
                if self.entity_classes.is_some() {
                    return Ok(());
                }

                let cmd = haste_dota2_protos::CDemoClassInfo::decode(data)?;
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

    fn handle_cmd_packet(&mut self, cmd: haste_dota2_protos::CDemoPacket) -> Result<()> {
        let data = cmd.data.unwrap_or_default();
        let mut br = BitReader::new(&data);
        while br.get_num_bits_left() > 8 {
            let command = br.read_ubitvar()?;
            let size = br.read_uvarint32()? as usize;

            let buf = &mut self.buf[..size];
            br.read_bytes(buf)?;
            let buf: &_ = buf;

            self.visitor.visit_packet(command, buf)?;

            use haste_dota2_protos::SvcMessages;
            match command {
                c if c == SvcMessages::SvcCreateStringTable as u32 => {
                    let msg = haste_dota2_protos::CsvcMsgCreateStringTable::decode(buf)?;
                    self.handle_svc_create_string_table(msg)?;
                }

                c if c == SvcMessages::SvcUpdateStringTable as u32 => {
                    let msg = haste_dota2_protos::CsvcMsgUpdateStringTable::decode(buf)?;
                    self.handle_svc_update_string_table(msg)?;
                }

                c if c == SvcMessages::SvcPacketEntities as u32 => {
                    let msg = haste_dota2_protos::CsvcMsgPacketEntities::decode(buf)?;
                    self.handle_svc_packet_entities(msg)?;
                }

                _ => {}
            }
        }
        Ok(())
    }

    fn handle_svc_create_string_table(
        &mut self,
        msg: haste_dota2_protos::CsvcMsgCreateStringTable,
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
        msg: haste_dota2_protos::CsvcMsgUpdateStringTable,
    ) -> Result<()> {
        let string_table = self
            .string_tables
            .get_table_mut(msg.table_id.expect("table id") as usize)
            // TODO: do not panic
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
        msg: haste_dota2_protos::CsvcMsgPacketEntities,
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

    fn handle_cmd_string_tables(
        &mut self,
        cmd: haste_dota2_protos::CDemoStringTables,
    ) -> Result<()> {
        self.string_tables.do_full_update(cmd);

        // SAFETY: entity_classes value is expected to be already assigned
        let entity_classes = unsafe { self.entity_classes.as_ref().unwrap_unchecked() };
        if let Some(string_table) = self.string_tables.find_table(INSTANCE_BASELINE_TABLE_NAME) {
            self.instance_baseline
                .update(string_table, entity_classes.classes)?;
        }

        Ok(())
    }

    fn handle_cmd_full_packet(&mut self, cmd: haste_dota2_protos::CDemoFullPacket) -> Result<()> {
        if let Some(string_table) = cmd.string_table {
            self.handle_cmd_string_tables(string_table)?;
        }

        if let Some(packet) = cmd.packet {
            self.handle_cmd_packet(packet)?;
        }

        Ok(())
    }
}
