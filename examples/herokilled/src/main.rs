use std::{collections::HashMap, fs::File, io::BufReader};

use handler::HandlerVisitor;
use haste::{
    entities::{self, Entity},
    parser::{self, Context, Parser},
    stringtables::StringTable,
    valveprotos::deadlock::{CCitadelUserMsgHeroKilled, CitadelUserMessageIds},
};

mod handler;

fn get_entity_name<'a>(entity: &'a Entity, entity_names: &'a StringTable) -> Option<&'a str> {
    const NAME_STRINGTABLE_INDEX_KEY: u64 =
        entities::fkey_from_path(&["m_pEntity", "m_nameStringableIndex"]);
    let name_stringtable_index: i32 = entity.get_value(&NAME_STRINGTABLE_INDEX_KEY)?;

    let (_, name_stringtable_item) = entity_names
        .items()
        .find(|(i, _)| **i == name_stringtable_index)?;
    let raw_string = name_stringtable_item.string.as_ref()?;
    std::str::from_utf8(raw_string).ok()
}

#[derive(Default)]
struct Score {
    kills: usize,
    deaths: usize,
}

#[derive(Default)]
struct State {
    hero_scores: HashMap<String, Score>,
}

fn hero_killed(
    state: &mut State,
    ctx: &Context,
    msg: &CCitadelUserMsgHeroKilled,
) -> parser::Result<()> {
    let entities = ctx.entities().unwrap();

    let string_tables = ctx.string_tables().unwrap();
    let entity_names = string_tables.find_table("EntityNames").unwrap();

    let scorer_name = entities
        .get(&msg.entindex_scorer())
        .and_then(|entindex| get_entity_name(entindex, entity_names))
        .unwrap_or("<some-other-unit>");

    let victim_name = entities
        .get(&msg.entindex_victim())
        .and_then(|entindex| get_entity_name(entindex, entity_names))
        .unwrap();

    println!("{} killed {}", scorer_name, victim_name);

    state
        .hero_scores
        .entry(scorer_name.to_string())
        .or_default()
        .kills += 1;
    state
        .hero_scores
        .entry(victim_name.to_string())
        .or_default()
        .deaths += 1;

    Ok(())
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: herokilled <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);
    let state = State::default();
    let mut visitor = HandlerVisitor::with_state(state).with(
        CitadelUserMessageIds::KEUserMsgHeroKilled as u32,
        hero_killed,
    );
    let mut parser = Parser::from_reader_with_visitor(buf_reader, &mut visitor)?;
    parser.run_to_end()?;

    println!();

    for (hero, score) in visitor.state().hero_scores.iter() {
        println!(
            "{} got {} kills and died {} times",
            hero, score.kills, score.deaths
        );
    }

    Ok(())
}
