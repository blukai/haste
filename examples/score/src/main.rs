use haste_dota2::{
    entities::{Entity, UpdateType},
    fieldpath::FIELD_PATH_DATA_SIZE,
    fieldvalue::FieldValue,
    fnv1a,
    parser::{self, Parser, Visitor},
};
use std::{fs::File, io::BufReader};

fn get_field_key(entity: &Entity, field_path: &[&str]) -> Option<u64> {
    let mut field_path_data = [0u8; FIELD_PATH_DATA_SIZE];
    let mut fields = &entity.flattened_serializer.fields;
    let mut i = 0;
    for var_name in field_path {
        let var_name_hash = fnv1a::hash_u8(var_name.as_bytes());
        for (j, field) in fields.iter().enumerate() {
            if field.var_name_hash == var_name_hash {
                field_path_data[i] = j as u8;
                if let Some(field_serializer) = field.field_serializer.as_ref() {
                    fields = &field_serializer.fields;
                }
                break;
            }
        }
        i += 1;
    }
    if field_path.len().eq(&i) {
        Some(fnv1a::hash_u8(&field_path_data[..i]))
    } else {
        None
    }
}

struct MyVisitor;

impl Visitor for MyVisitor {
    fn visit_entity(
        &self,
        _update_flags: usize,
        _update_type: UpdateType,
        entity: &Entity,
    ) -> parser::Result<()> {
        if entity
            .flattened_serializer
            .serializer_name_hash
            .eq(&fnv1a::hash_u8(b"CDOTATeam"))
        {
            let team_num_key = get_field_key(entity, &["m_iTeamNum"]);
            if let Some(team_num_key) = team_num_key {
                let team_num = entity.field_values.get(&team_num_key);
                if team_num.is_some_and(|team_num| match team_num {
                    FieldValue::U32(team_num) if *team_num == 2 || *team_num == 3 => true,
                    _ => false,
                }) {
                    let hero_kills_key = get_field_key(entity, &["m_iHeroKills"]);
                    if let Some(hero_kills_key) = hero_kills_key {
                        let hero_kills = entity.field_values.get(&hero_kills_key);
                        println!("team_num: {:?}; hero_kills: {:?}", team_num, hero_kills);
                    }
                }
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
        eprintln!("usage: allchat <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);
    let mut parser = Parser::from_reader_with_visitor(buf_reader, MyVisitor)?;
    parser.parse_to_end()
}
