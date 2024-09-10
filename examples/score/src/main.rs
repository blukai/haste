use haste::{
    entities::{self, Entity, UpdateType},
    fieldvalue::FieldValue,
    fxhash,
    parser::{self, Context, Parser, Visitor},
};
use std::{fs::File, io::BufReader};

struct MyVisitor;

impl Visitor for MyVisitor {
    fn on_entity(
        &mut self,
        _ctx: &Context,
        _update_flags: usize,
        _update_type: UpdateType,
        entity: &Entity,
    ) -> parser::Result<()> {
        if entity
            .serializer()
            .serializer_name
            .hash
            .eq(&fxhash::hash_bytes(b"CDOTATeam"))
        {
            let team_num_key = entities::make_field_key(&["m_iTeamNum"]);
            let team_num = entity.get_value(&team_num_key);
            if team_num.is_some_and(|team_num| matches!(team_num, FieldValue::U8(team_num) if *team_num == 2 || *team_num == 3)) {
                let hero_kills_key = entities::make_field_key(&["m_iHeroKills"]);
                let hero_kills = entity.get_value(&hero_kills_key);
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
