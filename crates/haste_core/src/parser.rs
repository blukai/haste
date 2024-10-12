use std::io::{self, SeekFrom};

use anyhow::Result;
use prost::Message;
use valveprotos::common::{
    CDemoFullPacket, CDemoPacket, CDemoStringTables, CsvcMsgCreateStringTable,
    CsvcMsgPacketEntities, CsvcMsgServerInfo, CsvcMsgUpdateStringTable, EDemoCommands, SvcMessages,
};

use crate::bitreader::BitReader;
use crate::demofile::{DemoHeaderError, DEMO_RECORD_BUFFER_SIZE};
use crate::demostream::{CmdHeader, DemoStream};
use crate::entities::{DeltaHeader, Entity, EntityContainer};
use crate::entityclasses::EntityClasses;
use crate::fielddecoder::FieldDecodeContext;
use crate::flattenedserializers::FlattenedSerializerContainer;
use crate::instancebaseline::{InstanceBaseline, INSTANCE_BASELINE_TABLE_NAME};
use crate::stringtables::StringTableContainer;

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

// NOTE: primary purpose of Context is to to be able to expose state to the
// public; attempts to put parser into arguments of Visitor's method did not
// result in anything satisfyable.
//
// TODO: consider turing Context into an enum with Initialized and Uninitialized variants. though
// there also must be an intermediary variant (or maybe stuff can be piled into Uninitialized
// variant) for incremental initialization. this may improve public api because string_tables,
// serializer, and other methods will not have to return Option when context is initialized.
pub struct Context {
    string_tables: StringTableContainer,
    instance_baseline: InstanceBaseline,
    serializers: Option<FlattenedSerializerContainer>,
    entity_classes: Option<EntityClasses>,
    entities: EntityContainer,
    tick_interval: f32,
    full_packet_interval: i32,
    tick: i32,
    prev_tick: i32,
}

impl Context {
    // NOTE: following methods are public-facing api; do not use them internally

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
    pub fn entity_classes(&self) -> Option<&EntityClasses> {
        self.entity_classes.as_ref()
    }

    #[inline]
    pub fn entities(&self) -> Option<&EntityContainer> {
        if self.entities.is_empty() {
            None
        } else {
            Some(&self.entities)
        }
    }

    #[inline]
    pub fn tick_interval(&self) -> f32 {
        self.tick_interval
    }

    #[inline]
    pub fn tick(&self) -> i32 {
        self.tick
    }
}

pub trait Visitor {
    // TODO: include updated fields (list of field paths?)
    #[allow(unused_variables)]
    fn on_entity(
        &mut self,
        ctx: &Context,
        delta_header: DeltaHeader,
        entity: &Entity,
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

    #[allow(unused_variables)]
    fn on_tick_end(&mut self, ctx: &Context) -> Result<()> {
        Ok(())
    }
}

/// ControlFlow indicates the desired behavior of the run loop.
enum ControlFlow {
    /// indicates that the command should be handled by the parser.
    HandleCmd,
    /// indicates that the command should be skipped; the stream position will be advanced by the
    /// size of the command using `SeekFrom::Current(cmd_header.size)`.
    SkipCmd,
    /// indicates that the command should not be handled nor skipped, suggesting that it has been
    /// handled in a different manner outside the regular flow.
    IgnoreCmd,
    /// stops further processing and indicates that any work performed during the current cycle
    /// must be undone.
    Break,
}

// TODO: maybe rename to DemoPlayer (or DemoRunner?)
pub struct Parser<D: DemoStream, V: Visitor> {
    demo_stream: D,
    buf: Vec<u8>,
    visitor: V,
    ctx: Context,
    // NOTE(blukai): is this the place for this? can it be moved "closer" to entities somewhere?
    field_decode_ctx: FieldDecodeContext,
}

impl<D: DemoStream, V: Visitor> Parser<D, V> {
    pub fn from_stream_with_visitor(demo_stream: D, visitor: V) -> Result<Self, DemoHeaderError> {
        Ok(Self {
            demo_stream,
            buf: vec![0; DEMO_RECORD_BUFFER_SIZE],
            visitor,
            ctx: Context {
                entities: EntityContainer::new(),
                string_tables: StringTableContainer::default(),
                instance_baseline: InstanceBaseline::default(),
                serializers: None,
                entity_classes: None,
                tick_interval: 0.0,
                full_packet_interval: 0,
                tick: -1,
                prev_tick: -1,
            },
            field_decode_ctx: FieldDecodeContext::default(),
        })
    }

    // TODO(blukai): what if parse_to_end and parse_to_tick will not be Parser's direct
    // "responsibility", but instead they will be implemented as "extensions" or something?
    //
    // why? for example dota 2 allows to record replays in real time (not anymore in matchmaking,
    // but in lobbies). those replays can be parsed in real time, but the process requires some
    // special handling (watch fs events (and in some cases poll) of demo file that is being
    // recorded).
    //
    // must be publicly exposed for this to be actually useful.
    fn run<F>(&mut self, mut handler: F) -> Result<()>
    where
        F: FnMut(&mut Self, &CmdHeader) -> Result<ControlFlow>,
    {
        loop {
            match self.demo_stream.read_cmd_header() {
                Ok(cmd_header) => {
                    self.ctx.prev_tick = self.ctx.tick;
                    self.ctx.tick = cmd_header.tick;
                    match handler(self, &cmd_header)? {
                        ControlFlow::HandleCmd => {
                            self.handle_cmd(&cmd_header)?;
                            if self.ctx.prev_tick != self.ctx.tick {
                                self.visitor.on_tick_end(&self.ctx)?;
                            }
                        }
                        ControlFlow::SkipCmd => self.demo_stream.skip_cmd(&cmd_header)?,
                        ControlFlow::IgnoreCmd => {}
                        ControlFlow::Break => {
                            self.demo_stream.unread_cmd_header(&cmd_header)?;
                            self.ctx.tick = self.ctx.prev_tick;
                            return Ok(());
                        }
                    }
                }
                Err(err) => {
                    if self.demo_stream.is_at_eof().unwrap_or_default() {
                        return Ok(());
                    }
                    return Err(err.into());
                }
            }
        }
    }

    pub fn run_to_end(&mut self) -> Result<()> {
        self.run(|_notnotself, _cmd_header| Ok(ControlFlow::HandleCmd))
    }

    fn reset(&mut self) -> Result<(), io::Error> {
        self.demo_stream
            .seek(SeekFrom::Start(self.demo_stream.start_position()))?;

        self.ctx.entities.clear();
        self.ctx.string_tables.clear();
        self.ctx.instance_baseline.clear();
        self.ctx.tick = -1;
        self.ctx.prev_tick = -1;

        Ok(())
    }

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
                did_handle_first_sync_tick = cmd_header.cmd == EDemoCommands::DemSyncTick;
                return Ok(ControlFlow::HandleCmd);
            }

            let is_full_packet = cmd_header.cmd == EDemoCommands::DemFullPacket;
            let distance_to_target_tick = target_tick - notnotself.ctx.tick;
            // TODO: what if there's no full packet ahead? maybe dem file is
            // corrupted or something... scan for full packets before enterint
            // the "run"?
            let has_full_packet_ahead =
                distance_to_target_tick > notnotself.ctx.full_packet_interval + 100;
            if is_full_packet {
                let cmd_body = notnotself.demo_stream.read_cmd(cmd_header)?;
                notnotself
                    .visitor
                    .on_cmd(&notnotself.ctx, cmd_header, cmd_body)?;

                let mut cmd = D::decode_cmd_full_packet(cmd_body)?;
                if has_full_packet_ahead {
                    // NOTE: clarity seem to ignore "intermediary" full packet's
                    // packet
                    //
                    // TODO: verify that is okay to ignore "intermediary" full
                    // packet's packet
                    cmd.packet = None;
                }
                notnotself.handle_cmd_full_packet(cmd)?;
                // NOTE: there's absolutely no reason to check if tick changed because it changed.
                notnotself.visitor.on_tick_end(&notnotself.ctx)?;

                did_handle_last_full_packet = !has_full_packet_ahead;

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
        // TODO: consider introducing CmdInstance thing that would allow to decode body once and
        // not read it, but skip, if unconsumed. note that to work temporary ownership of
        // demo_stream will need to be taken.
        let cmd_body = self.demo_stream.read_cmd(cmd_header)?;
        self.visitor.on_cmd(&self.ctx, cmd_header, cmd_body)?;

        match cmd_header.cmd {
            EDemoCommands::DemPacket | EDemoCommands::DemSignonPacket => {
                let cmd = D::decode_cmd_packet(cmd_body)?;
                self.handle_cmd_packet(cmd)?;
            }

            EDemoCommands::DemSendTables => {
                // NOTE: this check exists because seeking exists, there's no
                // need to re-parse flattened serializers
                if self.ctx.serializers.is_some() {
                    return Ok(());
                }

                let cmd = D::decode_cmd_send_tables(cmd_body)?;
                self.ctx.serializers = Some(FlattenedSerializerContainer::parse(cmd)?);
            }

            EDemoCommands::DemClassInfo => {
                // NOTE: this check exists because seeking exists, there's no
                // need to re-parse entity classes
                if self.ctx.entity_classes.is_some() {
                    return Ok(());
                }

                let cmd = D::decode_cmd_class_info(cmd_body)?;
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

        while br.num_bits_left() > 8 {
            let command = br.read_ubitvar();
            let size = br.read_uvarint32() as usize;

            let buf = &mut self.buf[..size];
            br.read_bytes(buf);
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

                        let ratio = DEFAULT_TICK_INTERVAL / tick_interval;
                        self.ctx.full_packet_interval = DEFAULT_FULL_PACKET_INTERVAL * ratio as i32;

                        // NOTE(blukai): field decoder context needs tick interval to be able to
                        // decode simulation time floats.
                        self.field_decode_ctx.tick_interval = tick_interval;
                    }
                }

                _ => {
                    // ignore
                }
            }
        }

        br.is_overflowed()?;
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
        );

        let string_data = if msg.data_compressed() {
            let sd = msg.string_data();
            let decompress_len = snap::raw::decompress_len(sd)?;
            snap::raw::Decoder::new().decompress(sd, &mut self.buf)?;
            &self.buf[..decompress_len]
        } else {
            msg.string_data()
        };

        let mut br = BitReader::new(string_data);
        string_table.parse_update(&mut br, msg.num_entries())?;
        br.is_overflowed()?;

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

        let mut br = BitReader::new(msg.string_data());
        string_table.parse_update(&mut br, msg.num_changed_entries())?;
        br.is_overflowed()?;

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
            // TODO(blukai): maybe try to make naming consistent with valve; see
            // https://github.com/taylorfinnell/csgo-demoinfo/blob/74960c07c387b080a0965c4fc33d69ccf9bfe6c8/demoinfogo/demofiledump.cpp#L1153C18-L1153C29
            // and CL_ParseDeltaHeader in engine/client.cpp
            entity_index += br.read_ubitvar() as i32 + 1;

            let delta_header = DeltaHeader::from_bit_reader(&mut br);
            match delta_header {
                DeltaHeader::CREATE => {
                    let entity = unsafe {
                        let entity = self.ctx.entities.handle_create(
                            entity_index,
                            &mut self.field_decode_ctx,
                            &mut br,
                            entity_classes,
                            instance_baseline,
                            serializers,
                        )?;
                        // SAFETY: borrow checker is not happy because handle_create requires
                        // mutable access to entities; rust's borrowing rules specify that you
                        // cannot have both mutable and immutable refs to the same data at the same
                        // time.
                        //
                        // alternative would be to call .handle_create and then .get, but that does
                        // not make any sense, that is redundant because .get is called inside of
                        // .handle_create. i can't think of any issues that may arrise because of
                        // my raw pointer approach.
                        &*(entity as *const Entity)
                    };
                    self.visitor.on_entity(&self.ctx, delta_header, entity)?;
                }
                DeltaHeader::DELETE => {
                    let entity = unsafe { self.ctx.entities.handle_delete_unchecked(entity_index) };
                    self.visitor.on_entity(&self.ctx, delta_header, &entity)?;
                }
                DeltaHeader::UPDATE => {
                    let entity = unsafe {
                        let entity = self.ctx.entities.handle_update_unchecked(
                            entity_index,
                            &mut self.field_decode_ctx,
                            &mut br,
                        )?;
                        // SAFETY: see comment above (below .handle_create call); same stuff.
                        &*(entity as *const Entity)
                    };
                    self.visitor.on_entity(&self.ctx, delta_header, entity)?;
                }
                _ => {}
            }
        }

        br.is_overflowed()?;
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

    // public api
    // ----

    #[inline]
    pub fn demo_stream(&self) -> &D {
        &self.demo_stream
    }

    #[inline]
    pub fn demo_stream_mut(&mut self) -> &mut D {
        &mut self.demo_stream
    }

    #[inline]
    pub fn context(&self) -> &Context {
        &self.ctx
    }
}

pub struct NopVisitor;
impl Visitor for NopVisitor {}

impl<D: DemoStream> Parser<D, NopVisitor> {
    #[inline]
    pub fn from_stream(demo_stream: D) -> Result<Self, DemoHeaderError> {
        Self::from_stream_with_visitor(demo_stream, NopVisitor)
    }
}
