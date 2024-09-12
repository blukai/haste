use haste::{
    entities::{make_field_key, DeltaHeader, Entity},
    fieldvalue::FieldValue,
    parser::{self, Context, Parser, Visitor},
};
use std::{collections::HashMap, fs::File, io::BufReader};

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
    ) -> parser::Result<()> {
        let life_state_key = make_field_key(&["m_lifeState"]);
        let Some(FieldValue::U8(next_life_state)) = entity.get_value(&life_state_key).cloned()
        else {
            return Ok(());
        };

        let prev_life_state = *self.life_states.get(&entity.index()).unwrap_or(&LIFE_DEAD);
        if next_life_state == prev_life_state {
            return Ok(());
        }

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

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: lifestate <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);
    let mut parser = Parser::from_reader_with_visitor(buf_reader, MyVisitor::default())?;
    parser.run_to_end()
}
