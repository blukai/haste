use std::fs::File;
use std::io::BufReader;
use std::time::Instant;

use haste::parser::Parser;
use rand::Rng;

const N_SEEKS: u64 = 1000;

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
    let mut parser = Parser::from_reader(buf_reader)?;

    let mut rng = rand::thread_rng();
    let rng_range = -1..parser.total_ticks()?;

    let start = Instant::now();
    for _ in 0..N_SEEKS {
        let target_tick = rng.gen_range(rng_range.clone());
        println!("-----------------------------------------------------------");
        println!("seeking to {}/{}", target_tick, parser.total_ticks()?);
        let start = Instant::now();
        parser.run_to_tick(target_tick)?;
        println!("seek took {:?}", start.elapsed());
    }
    let elapsed = start.elapsed().as_millis() as u64;
    println!(
        "{} seek operations took {}ms, {}ms/seek",
        N_SEEKS,
        elapsed,
        elapsed / N_SEEKS
    );

    Ok(())
}
