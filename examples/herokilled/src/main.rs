use haste::{
    entities::{self, Entity},
    fieldvalue::FieldValue,
    parser::{self, Context, Parser, Visitor},
    protos::{self, prost::Message},
    stringtables::StringTable,
};
use std::{fs::File, io::BufReader};

struct MyVisitor;

fn get_entity_name<'a>(entity: &'a Entity, entity_names: &'a StringTable) -> Option<&'a str> {
    let name_si_key = entities::make_field_key(&["m_pEntity", "m_nameStringableIndex"]);
    let Some(FieldValue::I32(name_si)) = entity.get_value(&name_si_key) else {
        return None;
    };

    let Some((_, name_st_item)) = entity_names.items().find(|(i, _)| i.eq(&name_si)) else {
        return None;
    };

    let Some(raw_string) = name_st_item.string.as_ref() else {
        return None;
    };

    std::str::from_utf8(raw_string).ok()
}

impl Visitor for MyVisitor {
    fn on_packet(&mut self, ctx: &Context, packet_type: u32, data: &[u8]) -> parser::Result<()> {
        if packet_type == protos::CitadelUserMessageIds::KEUserMsgHeroKilled as u32 {
            let msg = protos::CCitadelUserMsgHeroKilled::decode(data)?;

            let entities = ctx.entities().unwrap();

            let string_tables = ctx.string_tables().unwrap();
            let entity_names = string_tables.find_table("EntityNames").unwrap();

            let scorer = entities.get(&msg.entindex_scorer()).unwrap();
            let scorer_name = get_entity_name(scorer, entity_names).unwrap();

            let victim = entities.get(&msg.entindex_victim()).unwrap();
            let victim_name = get_entity_name(victim, entity_names).unwrap();

            println!("{} killed {}", scorer_name, victim_name);
        }

        Ok(())
    }
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
    let mut parser = Parser::from_reader_with_visitor(buf_reader, MyVisitor)?;
    parser.run_to_end()
}
