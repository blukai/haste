use std::fs::File;
use std::io::BufReader;

use anyhow::Result;
use haste::parser::Parser;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: emptybench <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);
    let mut parser = Parser::from_reader(buf_reader)?;
    parser.run_to_end()
}
