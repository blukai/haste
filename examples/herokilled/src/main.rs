use haste::{
    entities::{self, Entity},
    fieldvalue::FieldValue,
    handler::HandlerVisitor,
    parser::{self, Context, Parser},
    protos::{self, CCitadelUserMsgHeroKilled},
    stringtables::StringTable,
};
use std::{collections::HashMap, fs::File, io::BufReader};

fn get_entity_name<'a>(entity: &'a Entity, entity_names: &'a StringTable) -> Option<&'a str> {
    let name_si_key = entities::make_field_key(&["m_pEntity", "m_nameStringableIndex"]);
    let Some(FieldValue::I32(name_si)) = entity.get_value(&name_si_key) else {
        return None;
    };

    let (_, name_st_item) = entity_names.items().find(|(i, _)| i.eq(&name_si))?;

    let raw_string = name_st_item.string.as_ref()?;

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

    let scorer = entities.get(&msg.entindex_scorer()).unwrap();
    let scorer_name = get_entity_name(scorer, entity_names).unwrap();

    let victim = entities.get(&msg.entindex_victim()).unwrap();
    let victim_name = get_entity_name(victim, entity_names).unwrap();

    println!("{} killed {}", scorer_name, victim_name);

    if let Some(score) = state.hero_scores.get_mut(scorer_name) {
        score.kills += 1;
    } else {
        state.hero_scores.insert(
            scorer_name.to_string(),
            Score {
                kills: 1,
                deaths: 0,
            },
        );
    }

    if let Some(score) = state.hero_scores.get_mut(victim_name) {
        score.deaths += 1;
    } else {
        state.hero_scores.insert(
            victim_name.to_string(),
            Score {
                kills: 0,
                deaths: 1,
            },
        );
    }

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
        protos::CitadelUserMessageIds::KEUserMsgHeroKilled as u32,
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
