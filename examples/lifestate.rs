use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use anyhow::{Context as _, Result};
use haste::demofile::DemoFile;
use haste::entities::{fkey_from_path, DeltaHeader, Entity};
use haste::parser::{Context, Parser, Visitor};

// public/const.h
const LIFE_ALIVE: u8 = 0; // alive
const LIFE_DEAD: u8 = 2; // dead. lying still.

#[derive(Default)]
struct MyVisitor {
    life_states: HashMap<i32, u8>,
}

impl Visitor for MyVisitor {
    fn on_entity(
        &mut self,
        ctx: &Context,
        _delta_header: DeltaHeader,
        entity: &Entity,
    ) -> Result<()> {
        const LIFE_STATE_KEY: u64 = fkey_from_path(&["m_lifeState"]);
        let Some(next_life_state) = entity.get_value(&LIFE_STATE_KEY) else {
            // NOTE: not all entities have life state field
            return Ok(());
        };

        let prev_life_state = *self.life_states.get(&entity.index()).unwrap_or(&LIFE_DEAD);
        if next_life_state == prev_life_state {
            return Ok(());
        }

        // TODO: parser must provide a list of changed fields
        self.life_states.insert(entity.index(), next_life_state);

        match next_life_state {
            LIFE_ALIVE => eprintln!(
                "{:>6}: {} at index {} has spawned",
                ctx.tick(),
                entity.serializer().serializer_name.str,
                entity.index(),
            ),
            LIFE_DEAD => eprintln!(
                "{:>6}: {} at index {} has died",
                ctx.tick(),
                entity.serializer().serializer_name.str,
                entity.index(),
            ),
            _ => {}
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1).context("usage: lifestate <filepath>")?;
    let file = File::open(filepath)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;
    let mut parser = Parser::from_stream_with_visitor(demo_file, MyVisitor::default())?;
    parser.run_to_end()
}
