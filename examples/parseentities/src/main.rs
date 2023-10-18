use haste::parser::{Parser, Visitor};
use std::{fs::File, io::BufReader};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct MyVisitor;

impl Visitor for MyVisitor {
    fn visit_entity(
        &self,
        _update_flags: usize,
        _update_type: haste::entities::UpdateType,
        _entity: &haste::entities::Entity,
    ) {
        // dbg!(
        //     update_flags,
        //     update_type,
        //     &entity.flattened_serializer.serializer_name,
        //     &entity.field_values
        // );
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: parseentities <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    // NOTE: BufReader makes io much more efficient (see BufReader's docs for
    // more info).
    let file = BufReader::new(file);
    let mut parser = Parser::from_reader(file, MyVisitor)?;
    parser.parse_all()
}
