use crate::{
    bitbuf::BitReader,
    demofile::{CmdHeader, DemoFile, DemoHeader, DEMO_BUFFER_SIZE, DEMO_HEADER_SIZE},
    entities::{self, EntityContainer},
    entityclasses::EntityClasses,
    flattenedserializers::FlattenedSerializerContainer,
    instancebaseline::{InstanceBaseline, INSTANCE_BASELINE_TABLE_NAME},
    protos::{
        prost::Message, CDemoClassInfo, CDemoFileInfo, CDemoFullPacket, CDemoPacket,
        CDemoSendTables, CDemoStringTables, CsvcMsgCreateStringTable, CsvcMsgPacketEntities,
        CsvcMsgServerInfo, CsvcMsgUpdateStringTable, EDemoCommands, SvcMessages,
    },
    stringtables::StringTableContainer,
};
use std::io::{Read, Seek, SeekFrom};

// as can be observed when dumping commands. also as specified in clarity
// (src/main/java/skadistats/clarity/model/engine/AbstractDotaEngineType.java)
// and documented in manta (string_table.go).
//
// NOTE: full packet interval is 1800 only if tick interval is 1 / 30 - this is true for dota2, but
// deaclock's tick interval is x 2.
const DEFAULT_FULL_PACKET_INTERVAL: i32 = 1800;
// NOTE: tick interval is needed to be able to correctly decide simulation time values.
// dota2's tick interval is 1 / 30; deadlock's 1 / 60 - they are constant.
const DEFAULT_TICK_INTERVAL: f32 = 1.0 / 30.0;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;

// NOTE: primary purpose of Context is to to be able to expose state to the
// public; attempts to put parser into arguments of Visitor's method did not
// result in anything satisfyable.
pub struct Context {
    string_tables: StringTableContainer,
    instance_baseline: InstanceBaseline,
    serializers: Option<FlattenedSerializerContainer>,
    entity_classes: Option<EntityClasses>,
    entities: EntityContainer,
    tick: i32,
    prev_tick: i32,
    // TODO: pass tick_interval down into simulation time decoder
    // (InternalF32SimulationTimeDecoder).
    tick_interval: f32,
    full_packet_interval: i32,
}

impl Context {
    // NOTE: following methods are public-facing api; do not use them internally

    #[inline]
    pub fn tick(&self) -> i32 {
        self.tick
    }

    #[inline]
    pub fn string_tables(&self) -> Option<&StringTableContainer> {
        if self.string_tables.is_empty() {
            None
        } else {
            Some(&self.string_tables)
        }
    }

    #[inline]
    pub fn serializers(&self) -> Option<&FlattenedSerializerContainer> {
        self.serializers.as_ref()
    }

    #[inline]
    pub fn entities(&self) -> Option<&EntityContainer> {
        if self.entities.is_empty() {
            None
        } else {
            Some(&self.entities)
        }
    }
}

pub trait Visitor {
    #[allow(unused_variables)]
    fn on_entity(
        &mut self,
        ctx: &Context,
        update_flags: usize,
        update_type: entities::UpdateType,
        // TODO: include updated fields (list of field paths?)
        entity: &entities::Entity,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_cmd(&mut self, ctx: &Context, cmd_header: &CmdHeader, data: &[u8]) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_packet(&mut self, ctx: &Context, packet_type: u32, data: &[u8]) -> Result<()> {
        Ok(())
    }

    // TODO: come up with an example that would use / will rely on on_tick_end
    #[allow(unused_variables)]
    fn on_tick_end(&mut self, ctx: &Context) -> Result<()> {
        Ok(())
    }
}

// ControlFlow indicates the desired behavior of the run loop.
pub enum ControlFlow {
    // HandleCmd indicates that the command should be handled by the parser
    HandleCmd,
    // SkipCmd indicates that the command should be skipped; the stream position
    // will be advanced by the size of the command using
    // `SeekFrom::Current(cmd_header.size)`.
    SkipCmd,
    // IgnoreCmd indicates that the command should not be handled nor skipped,
    // suggesting that it has been handled in a different manner outside the
    // regular flow.
    IgnoreCmd,
    // Break stops further processing and indicates that any work performed
    // during the current cycle must be undone.
    Break,
}

// TODO: maybe rename to DemoPlayer (or DemoRunner?)
pub struct Parser<R: Read + Seek, V: Visitor> {
    demo_file: DemoFile<R>,
    buf: Vec<u8>,
    visitor: V,
    ctx: Context,
}

impl<R: Read + Seek, V: Visitor> Parser<R, V> {
    pub fn from_reader_with_visitor(rdr: R, visitor: V) -> Result<Self> {
        let mut demo_file = DemoFile::from_reader(rdr);
        let _demo_header = demo_file.read_demo_header()?;

        Ok(Self {
            demo_file,
            buf: vec![0; DEMO_BUFFER_SIZE],
            visitor,
            ctx: Context {
                entities: EntityContainer::new(),
                string_tables: StringTableContainer::default(),
                instance_baseline: InstanceBaseline::default(),
                serializers: None,
                entity_classes: None,
                tick: -1,
                prev_tick: -1,
                tick_interval: DEFAULT_TICK_INTERVAL,
                full_packet_interval: DEFAULT_FULL_PACKET_INTERVAL,
            },
        })
    }

    // ----

    pub fn run<F>(&mut self, mut handler: F) -> Result<()>
    where
        F: FnMut(&mut Self, &CmdHeader) -> Result<ControlFlow>,
    {
        loop {
            match self.demo_file.read_cmd_header() {
                Ok(cmd_header) => {
                    self.ctx.prev_tick = self.ctx.tick;
                    self.ctx.tick = cmd_header.tick;
                    match handler(self, &cmd_header) {
                        Ok(cf) => match cf {
                            ControlFlow::HandleCmd => {
                                self.handle_cmd(&cmd_header)?;
                                if self.ctx.prev_tick != self.ctx.tick {
                                    self.visitor.on_tick_end(&self.ctx)?;
                                }
                            }
                            ControlFlow::SkipCmd => self.demo_file.skip_cmd(&cmd_header)?,
                            ControlFlow::IgnoreCmd => {}
                            ControlFlow::Break => {
                                self.demo_file.unread_cmd_header(&cmd_header)?;
                                self.ctx.tick = self.ctx.prev_tick;
                                return Ok(());
                            }
                        },
                        Err(err) => return Err(Error::from(err)),
                    }
                }
                Err(err) => {
                    if self.demo_file.is_eof().unwrap_or_default() {
                        return Ok(());
                    }
                    return Err(Error::from(err));
                }
            }
        }
    }

    pub fn run_to_end(&mut self) -> Result<()> {
        self.run(|_notnotself, _cmd_header| Ok(ControlFlow::HandleCmd))
    }

    // TODO: this probably has to be private?
    pub fn reset(&mut self) -> Result<()> {
        self.demo_file
            .seek(SeekFrom::Start(DEMO_HEADER_SIZE as u64))?;
        self.ctx.string_tables.clear();
        self.ctx.instance_baseline.clear();
        self.ctx.entities.clear();
        self.ctx.prev_tick = -1;
        self.ctx.tick = -1;
        Ok(())
    }

    // TODO: rename parse_to_tick to run_to_tick
    pub fn run_to_tick(&mut self, target_tick: i32) -> Result<()> {
        // TODO: do not allow tick to be less then -1

        // TODO: do not allow tick to be greater then total ticks

        // TODO: do not clear if seeking forward and there's no full packet on
        // the way to the wanted tick / if target tick is closer then full
        // packet interval
        self.reset()?;

        // NOTE: EDemoCommands::DemSyncTick is the last command with 4294967295
        // tick (normlized to -1). last "initialization" command.
        let mut did_handle_first_sync_tick = false;

        // NOTE: EDemoCommands::DemFullPacket contains snapshot of everything...
        // everything? it does not seem like it: string tables must be handled.
        let mut did_handle_last_full_packet = false;

        self.run(|notnotself, cmd_header| {
            if cmd_header.tick > target_tick {
                return Ok(ControlFlow::Break);
            }

            // init string tables, flattened serializers and entity classes
            if !did_handle_first_sync_tick {
                did_handle_first_sync_tick = cmd_header.command == EDemoCommands::DemSyncTick;
                return Ok(ControlFlow::HandleCmd);
            }

            let is_full_packet = cmd_header.command == EDemoCommands::DemFullPacket;
            let distance_to_target_tick = target_tick - notnotself.ctx.tick;
            // TODO: what if there's no full packet ahead? maybe dem file is
            // corrupted or something... scan for full packets before enterint
            // the "run"?
            let has_full_packet_ahead =
                distance_to_target_tick > notnotself.ctx.full_packet_interval + 100;
            if is_full_packet {
                let cmd_data = notnotself.demo_file.read_cmd(cmd_header)?;
                notnotself
                    .visitor
                    .on_cmd(&notnotself.ctx, cmd_header, cmd_data)?;

                let mut cmd = CDemoFullPacket::decode(cmd_data)?;
                if has_full_packet_ahead {
                    // NOTE: clarity seem to ignore "intermediary" full packet's
                    // packet
                    //
                    // TODO: verify that is okay to ignore "intermediary" full
                    // packet's packet
                    cmd.packet = None;
                }
                notnotself.handle_cmd_full_packet(cmd)?;

                did_handle_last_full_packet = !has_full_packet_ahead;

                if notnotself.ctx.prev_tick != notnotself.ctx.tick {
                    notnotself.visitor.on_tick_end(&notnotself.ctx)?;
                }

                return Ok(ControlFlow::IgnoreCmd);
            }

            if did_handle_last_full_packet {
                Ok(ControlFlow::HandleCmd)
            } else {
                Ok(ControlFlow::SkipCmd)
            }
        })
    }

    // important initialization messages:
    // 1. DemSignonPacket (SvcCreateStringTable)
    // 2. DemSendTables (flattened serializers; never update)
    // 3. DemClassInfo (never update)
    fn handle_cmd(&mut self, cmd_header: &CmdHeader) -> Result<()> {
        let data = self.demo_file.read_cmd(cmd_header)?;
        self.visitor.on_cmd(&self.ctx, cmd_header, data)?;

        match cmd_header.command {
            EDemoCommands::DemPacket | EDemoCommands::DemSignonPacket => {
                let cmd = CDemoPacket::decode(data)?;
                self.handle_cmd_packet(cmd)?;
            }

            EDemoCommands::DemSendTables => {
                // NOTE: this check exists because seeking exists, there's no
                // need to re-parse flattened serializers
                if self.ctx.serializers.is_some() {
                    return Ok(());
                }

                let cmd = CDemoSendTables::decode(data)?;
                self.ctx.serializers = Some(FlattenedSerializerContainer::parse(cmd)?);
            }

            EDemoCommands::DemClassInfo => {
                // NOTE: this check exists because seeking exists, there's no
                // need to re-parse entity classes
                if self.ctx.entity_classes.is_some() {
                    return Ok(());
                }

                let cmd = CDemoClassInfo::decode(data)?;
                self.ctx.entity_classes = Some(EntityClasses::parse(cmd));

                // NOTE: DemClassInfo message becomes available after
                // SvcCreateStringTable(which has instancebaselines). to know
                // how long vec that will contain instancebaseline values needs
                // to be (to allocate precicely how much we need) we need to
                // wait for DemClassInfos.
                if let Some(string_table) = self
                    .ctx
                    .string_tables
                    .find_table(INSTANCE_BASELINE_TABLE_NAME)
                {
                    // SAFETY: entity_classes value was assigned above ^.
                    let entity_classes =
                        unsafe { self.ctx.entity_classes.as_ref().unwrap_unchecked() };
                    self.ctx
                        .instance_baseline
                        .update(string_table, entity_classes.classes)?;
                }
            }

            _ => {
                // ignore
            }
        }

        Ok(())
    }

    fn handle_cmd_packet(&mut self, cmd: CDemoPacket) -> Result<()> {
        let data = cmd.data.unwrap_or_default();
        let mut br = BitReader::new(&data);
        while br.get_num_bits_left() > 8 {
            let command = br.read_ubitvar()?;
            let size = br.read_uvarint32()? as usize;

            let buf = &mut self.buf[..size];
            br.read_bytes(buf)?;
            let buf: &_ = buf;

            self.visitor.on_packet(&self.ctx, command, buf)?;

            match command {
                c if c == SvcMessages::SvcCreateStringTable as u32 => {
                    let msg = CsvcMsgCreateStringTable::decode(buf)?;
                    self.handle_svc_create_string_table(msg)?;
                }

                c if c == SvcMessages::SvcUpdateStringTable as u32 => {
                    let msg = CsvcMsgUpdateStringTable::decode(buf)?;
                    self.handle_svc_update_string_table(msg)?;
                }

                c if c == SvcMessages::SvcPacketEntities as u32 => {
                    let msg = CsvcMsgPacketEntities::decode(buf)?;
                    self.handle_svc_packet_entities(msg)?;
                }

                c if c == SvcMessages::SvcServerInfo as u32 => {
                    let msg = CsvcMsgServerInfo::decode(buf)?;
                    if let Some(tick_interval) = msg.tick_interval {
                        self.ctx.tick_interval = tick_interval;
                        self.ctx.full_packet_interval = DEFAULT_FULL_PACKET_INTERVAL
                            * (DEFAULT_TICK_INTERVAL / tick_interval) as i32;
                    }
                }

                _ => {
                    // ignore
                }
            }
        }
        Ok(())
    }

    fn handle_svc_create_string_table(&mut self, msg: CsvcMsgCreateStringTable) -> Result<()> {
        let string_table = self.ctx.string_tables.create_string_table_mut(
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

        if string_table.name().eq(INSTANCE_BASELINE_TABLE_NAME) {
            if let Some(entity_classes) = self.ctx.entity_classes.as_ref() {
                self.ctx
                    .instance_baseline
                    .update(string_table, entity_classes.classes)?;
            }
        }

        Ok(())
    }

    fn handle_svc_update_string_table(&mut self, msg: CsvcMsgUpdateStringTable) -> Result<()> {
        debug_assert!(msg.table_id.is_some(), "invalid table id");
        let table_id = msg.table_id() as usize;

        debug_assert!(
            self.ctx.string_tables.has_table(table_id),
            "tryting to update non-existent table"
        );
        let string_table = unsafe {
            self.ctx
                .string_tables
                .get_table_mut(table_id)
                .unwrap_unchecked()
        };
        string_table.parse_update(
            &mut BitReader::new(msg.string_data()),
            msg.num_changed_entries(),
        )?;

        if string_table.name().eq(INSTANCE_BASELINE_TABLE_NAME) {
            if let Some(entity_classes) = self.ctx.entity_classes.as_ref() {
                self.ctx
                    .instance_baseline
                    .update(string_table, entity_classes.classes)?;
            }
        }

        Ok(())
    }

    // NOTE: handle_msg_packet_entities is partially based on
    // ReadPacketEntities in engine/client.cpp
    fn handle_svc_packet_entities(&mut self, msg: CsvcMsgPacketEntities) -> Result<()> {
        use entities::*;

        // SAFETY: safety here can only be guaranteed by the fact that entity
        // classes and flattened serializers become available before packet
        // entities.
        let entity_classes = unsafe { self.ctx.entity_classes.as_ref().unwrap_unchecked() };
        let serializers = unsafe { self.ctx.serializers.as_ref().unwrap_unchecked() };
        let instance_baseline = &self.ctx.instance_baseline;

        let entity_data = msg.entity_data();
        let mut br = BitReader::new(entity_data);

        let mut entity_index: i32 = -1;
        for _ in (0..msg.updated_entries()).rev() {
            entity_index += br.read_ubitvar()? as i32 + 1;

            let update_flags = parse_delta_header(&mut br)?;
            let update_type = determine_update_type(update_flags);

            match update_type {
                UpdateType::EnterPVS => {
                    // SAFETY: borrow checker is not happy because handle_create
                    // requires mutable access to entities; rust's borrowing
                    // rules specify that you cannot have both mutable and
                    // immutable refs to the same data at the same time.
                    //
                    // alternative would be to call .handle_create and then
                    // .get, but that does not make any sense, that is redundant
                    // because .get is called inside of .handle_create. i can't
                    // think of any issues that may arrise because of my raw
                    // pointer approach.
                    let entity = unsafe {
                        let entity = self.ctx.entities.handle_create(
                            entity_index,
                            &mut br,
                            entity_classes,
                            instance_baseline,
                            serializers,
                        )?;
                        &*(entity as *const Entity)
                    };
                    self.visitor
                        .on_entity(&self.ctx, update_flags, update_type, entity)?;
                }
                UpdateType::LeavePVS => {
                    if (update_flags & FHDR_DELETE) != 0 {
                        let entity =
                            unsafe { self.ctx.entities.handle_delete_unchecked(entity_index) };
                        self.visitor
                            .on_entity(&self.ctx, update_flags, update_type, &entity)?;
                    }
                }
                UpdateType::DeltaEnt => {
                    // SAFETY: see comment above for .handle_create call in
                    // EnterPVS arm; same stuff.
                    let entity = unsafe {
                        let entity = self
                            .ctx
                            .entities
                            .handle_update_unchecked(entity_index, &mut br)?;
                        &*(entity as *const Entity)
                    };

                    self.visitor
                        .on_entity(&self.ctx, update_flags, update_type, entity)?;
                }
            }
        }

        Ok(())
    }

    fn handle_cmd_string_tables(&mut self, cmd: CDemoStringTables) -> Result<()> {
        self.ctx.string_tables.do_full_update(cmd);

        // SAFETY: entity_classes value is expected to be already assigned
        let entity_classes = unsafe { self.ctx.entity_classes.as_ref().unwrap_unchecked() };
        if let Some(string_table) = self
            .ctx
            .string_tables
            .find_table(INSTANCE_BASELINE_TABLE_NAME)
        {
            self.ctx
                .instance_baseline
                .update(string_table, entity_classes.classes)?;
        }

        Ok(())
    }

    fn handle_cmd_full_packet(&mut self, cmd: CDemoFullPacket) -> Result<()> {
        if let Some(string_table) = cmd.string_table {
            self.handle_cmd_string_tables(string_table)?;
        }

        if let Some(packet) = cmd.packet {
            self.handle_cmd_packet(packet)?;
        }

        Ok(())
    }

    // ----

    // NOTE: it wouldn't be very nice to expose DemoFile that is owned by Parser
    // because it'll violate encapsulation; parser will lack control over the
    // DemoFile's internal state which may lead to to unintended consequences.

    #[inline]
    pub fn demo_header(&self) -> &DemoHeader {
        // SAFETY: it is safe to call unchecked method here becuase Self's
        // constructor will return an error if demo header check (that is
        // executed during the construction) fails.
        unsafe { self.demo_file.demo_header_unchecked() }
    }

    #[inline]
    pub fn file_info(&mut self) -> Result<&CDemoFileInfo> {
        self.demo_file.file_info().map_err(Error::from)
    }

    #[inline]
    pub fn ticks_per_second(&mut self) -> Result<f32> {
        self.demo_file.ticks_per_second().map_err(Error::from)
    }

    #[inline]
    pub fn ticks_per_frame(&mut self) -> Result<f32> {
        self.demo_file.ticks_per_frame().map_err(Error::from)
    }

    #[inline]
    pub fn total_ticks(&mut self) -> Result<i32> {
        self.demo_file.total_ticks().map_err(Error::from)
    }

    // NOTE: following methods are public-facing api; do not use them internally

    #[inline]
    pub fn tick(&self) -> i32 {
        self.ctx.tick()
    }

    #[inline]
    pub fn string_tables(&self) -> Option<&StringTableContainer> {
        self.ctx.string_tables()
    }

    #[inline]
    pub fn serializers(&self) -> Option<&FlattenedSerializerContainer> {
        self.ctx.serializers()
    }

    #[inline]
    pub fn entities(&self) -> Option<&EntityContainer> {
        self.ctx.entities()
    }
}

pub struct NopVisitor;
impl Visitor for NopVisitor {}

impl<R: Read + Seek> Parser<R, NopVisitor> {
    #[inline]
    pub fn from_reader(rdr: R) -> Result<Self> {
        Self::from_reader_with_visitor(rdr, NopVisitor)
    }
}
