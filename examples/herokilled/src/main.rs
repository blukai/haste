use handler::{HandlerVisitor, Player, PlayerState};
use haste::{
    entities::{self, Entity},
    fieldvalue::FieldValue,
    parser::{self, Context, Parser},
    protos::{self, CCitadelUserMsgChatMsg, CCitadelUserMsgHeroKilled},
    stringtables::StringTable,
};
use std::{collections::HashMap, fs::File, io::BufReader};

mod handler;

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
    players: HashMap<i32, Player>,
}

impl PlayerState for State {
    fn set_player(&mut self, player: Player) {
        self.players.insert(player.slot, player);
    }

    fn get_player(&self, slot: &i32) -> Option<&Player> {
        self.players.get(slot)
    }
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

fn chat(state: &mut State, ctx: &Context, msg: &CCitadelUserMsgChatMsg) -> parser::Result<()> {
    let player = state
        .get_player(&msg.player_slot())
        .ok_or(parser::Error::from("unknown player"))?;
    let channel = if msg.all_chat() {
        format!("(all ) {}>", player.name)
    } else {
        format!("({}  ) {}>", player.team_id, player.name)
    };
    println!("{} {}", channel, msg.text());

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
    let mut visitor = HandlerVisitor::with_state(state)
        .with(
            protos::CitadelUserMessageIds::KEUserMsgHeroKilled as u32,
            hero_killed,
        )
        .with(protos::CitadelUserMessageIds::KEUserMsgChatMsg as u32, chat);
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
