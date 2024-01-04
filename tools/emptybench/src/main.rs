use haste_dota2::parser::{Parser, Visitor};
use std::{fs::File, io::BufReader};

struct MyVisitor;
impl Visitor for MyVisitor {}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: emptybench <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);
    let mut parser = Parser::from_reader(buf_reader, MyVisitor)?;
    parser.parse_to_end()
}
