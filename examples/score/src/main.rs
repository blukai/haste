use haste::{
    entities::{make_field_key, DeltaHeader, Entity},
    fxhash,
    parser::{self, Context, Parser, Visitor},
};
use std::{fs::File, io::BufReader};

struct MyVisitor;

impl Visitor for MyVisitor {
    fn on_entity(
        &mut self,
        _ctx: &Context,
        _delta_header: DeltaHeader,
        entity: &Entity,
    ) -> parser::Result<()> {
        if entity
            .serializer()
            .serializer_name
            .hash
            .eq(&fxhash::hash_bytes(b"CDOTATeam"))
        {
            const TEAM_NUM_KEY: u64 = make_field_key(&["m_iTeamNum"]);
            let team_num: u8 = entity.try_get_value(&TEAM_NUM_KEY)?;
            if team_num == 2 || team_num == 3 {
                let hero_kills_key = make_field_key(&["m_iHeroKills"]);
                let hero_kills: u16 = entity.try_get_value(&hero_kills_key)?;
                println!("team_num: {:?}; hero_kills: {:?}", team_num, hero_kills);
            }
        }

        Ok(())
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: score <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);
    let mut parser = Parser::from_reader_with_visitor(buf_reader, MyVisitor)?;
    parser.run_to_end()
}
