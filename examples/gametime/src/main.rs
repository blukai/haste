/// this example shows how to compute game time in deadlock.
///
/// logic for dota 2 would be different. if you need an example - open an issue on github.
use anyhow::Context as _;
use haste::{
    entities::{make_field_key, Entity, UpdateType},
    fxhash,
    parser::{self, Context, Parser, Visitor},
    protos::{self, prost::Message},
};
use std::{fs::File, io::BufReader};

const DEADLOCK_GAMERULES_ENTITY: u64 = fxhash::hash_bytes(b"CCitadelGameRulesProxy");

#[derive(Default)]
struct MyVisitor {
    net_tick: u32,
    /// tick intervals in dota 2 and in deadlock are different.
    tick_interval: Option<f32>,
    /// game does not start when replay recording starts. in various deadlock replays i observed
    /// following values: 26.866669, 45.01667, 19.983334.
    game_start_time: Option<f32>,
    game_paused: bool,
    pause_start_tick: i32,
    total_paused_ticks: i32,
}

impl MyVisitor {
    fn handle_net_tick(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let msg = protos::CnetMsgTick::decode(data)?;
        if let Some(net_tick) = msg.tick {
            self.net_tick = net_tick;
        }
        Ok(())
    }

    fn handle_game_rules(&mut self, entity: &Entity) -> anyhow::Result<()> {
        debug_assert!(entity.serializer_name_heq(DEADLOCK_GAMERULES_ENTITY));

        let game_start_time: f32 = entity
            .get_value(&make_field_key(&["m_pGameRules", "m_flGameStartTime"]))
            .context("game start time field is missing")?
            .try_into()
            .context("game start time")?;
        // NOTE: 0.001 is an arbitrary number; nothing special.
        if game_start_time < 0.001 {
            return Ok(());
        }

        self.game_start_time = Some(game_start_time);

        self.game_paused = entity
            .get_value(&make_field_key(&["m_pGameRules", "m_bGamePaused"]))
            .context("game paused field is missing")?
            .try_into()
            .context("game paused")?;
        self.pause_start_tick = entity
            .get_value(&make_field_key(&["m_pGameRules", "m_nPauseStartTick"]))
            .context("pause start tick field is missing")?
            .try_into()
            .context("pause start tick")?;
        self.total_paused_ticks = entity
            .get_value(&make_field_key(&["m_pGameRules", "m_nTotalPausedTicks"]))
            .context("total paused ticks field is missing")?
            .try_into()
            .context("total paused ticks")?;

        Ok(())
    }

    /// `None` means that the game has not started yet.
    fn get_game_time(&self) -> Option<f32> {
        Some(
            ((self.net_tick as f32 - self.total_paused_ticks as f32) * self.tick_interval?)
                - self.game_start_time?,
        )
    }
}

impl Visitor for MyVisitor {
    fn on_cmd(
        &mut self,
        ctx: &Context,
        cmd_header: &haste::demofile::CmdHeader,
        _data: &[u8],
    ) -> parser::Result<()> {
        // DemSyncTick indicates that all initialization messages were handled and now actual data
        // will flow; at this point tick interval is known.
        if self.tick_interval.is_none() && cmd_header.command == protos::EDemoCommands::DemSyncTick
        {
            self.tick_interval = Some(ctx.tick_interval());
        }
        Ok(())
    }

    fn on_packet(&mut self, _ctx: &Context, packet_type: u32, data: &[u8]) -> parser::Result<()> {
        if packet_type == protos::NetMessages::NetTick as u32 {
            self.handle_net_tick(data)?;
        }
        Ok(())
    }

    fn on_entity(
        &mut self,
        _ctx: &Context,
        _update_flags: usize,
        _update_type: UpdateType,
        entity: &Entity,
    ) -> parser::Result<()> {
        if entity.serializer_name_heq(DEADLOCK_GAMERULES_ENTITY) {
            self.handle_game_rules(entity)?;
        }
        Ok(())
    }

    fn on_tick_end(&mut self, _ctx: &Context) -> parser::Result<()> {
        eprintln!("game_time: {:?}", self.get_game_time());
        Ok(())
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: gametime <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);
    let mut parser = Parser::from_reader_with_visitor(buf_reader, MyVisitor::default())?;
    parser.run_to_end()
}
