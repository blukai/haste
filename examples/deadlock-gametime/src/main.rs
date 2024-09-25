/// this example shows how to compute game time in deadlock.
///
/// logic for dota 2 would be different. if you need an example - open an issue on github.
use std::fs::File;
use std::io::BufReader;

use anyhow::Result;
use haste::entities::{fkey_from_path, DeltaHeader, Entity};
use haste::fxhash;
use haste::parser::{Context, Parser, Visitor};
use haste::valveprotos::common::{CnetMsgTick, EDemoCommands, NetMessages};
use haste::valveprotos::prost::Message;

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
        let msg = CnetMsgTick::decode(data)?;
        if let Some(net_tick) = msg.tick {
            self.net_tick = net_tick;
        }
        Ok(())
    }

    fn handle_game_rules(&mut self, entity: &Entity) -> anyhow::Result<()> {
        debug_assert!(entity.serializer_name_heq(DEADLOCK_GAMERULES_ENTITY));

        let game_start_time: f32 =
            entity.try_get_value(&fkey_from_path(&["m_pGameRules", "m_flGameStartTime"]))?;
        // NOTE: 0.001 is an arbitrary number; nothing special.
        if game_start_time < 0.001 {
            return Ok(());
        }

        self.game_start_time = Some(game_start_time);

        self.game_paused =
            entity.try_get_value(&fkey_from_path(&["m_pGameRules", "m_bGamePaused"]))?;
        self.pause_start_tick =
            entity.try_get_value(&fkey_from_path(&["m_pGameRules", "m_nPauseStartTick"]))?;
        self.total_paused_ticks =
            entity.try_get_value(&fkey_from_path(&["m_pGameRules", "m_nTotalPausedTicks"]))?;

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
    ) -> Result<()> {
        // DemSyncTick indicates that all initialization messages were handled and now actual data
        // will flow; at this point tick interval is known.
        if self.tick_interval.is_none() && cmd_header.cmd == EDemoCommands::DemSyncTick {
            self.tick_interval = Some(ctx.tick_interval());
        }
        Ok(())
    }

    fn on_packet(&mut self, _ctx: &Context, packet_type: u32, data: &[u8]) -> Result<()> {
        if packet_type == NetMessages::NetTick as u32 {
            self.handle_net_tick(data)?;
        }
        Ok(())
    }

    fn on_entity(
        &mut self,
        _ctx: &Context,
        _delta_header: DeltaHeader,
        entity: &Entity,
    ) -> Result<()> {
        if entity.serializer_name_heq(DEADLOCK_GAMERULES_ENTITY) {
            self.handle_game_rules(entity)?;
        }
        Ok(())
    }

    fn on_tick_end(&mut self, _ctx: &Context) -> Result<()> {
        eprintln!("game_time: {:?}", self.get_game_time());
        Ok(())
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: deadlock-gametime <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);
    let mut parser = Parser::from_reader_with_visitor(buf_reader, MyVisitor::default())?;
    parser.run_to_end()
}
