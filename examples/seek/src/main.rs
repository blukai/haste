use haste_dota2::parser::{self, Parser, Visitor};
use std::{fs::File, io::BufReader};

struct MyVisitor;

impl Visitor for MyVisitor {
    fn visit_cmd(
        &self,
        cmd_header: &haste_dota2::demofile::CmdHeader,
        _data: &[u8],
    ) -> parser::Result<()> {
        eprintln!("cmd {:>10} {:?}", cmd_header.tick, cmd_header.command);
        Ok(())
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: seek <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);
    let mut parser = Parser::from_reader_with_visitor(buf_reader, MyVisitor)?;

    parser.parse_to_tick(80085)?;
    parser.parse_to_tick(42)?;
    parser.parse_to_tick(0)?;

    Ok(())
}
