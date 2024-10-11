use std::fs::File;
use std::io::BufReader;
use std::time::Instant;

use anyhow::{Context, Result};
use haste::demofile::DemoFile;
use haste::demostream::DemoStream;
use haste::parser::Parser;
use rand::Rng;

const N_SEEKS: u64 = 1000;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1).context("usage: seek <filepath>")?;
    let file = File::open(filepath)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;
    let mut parser = Parser::from_stream(demo_file)?;

    let mut rng = rand::thread_rng();
    let rng_range = -1..parser.demo_stream_mut().total_ticks()?;

    let start = Instant::now();
    for _ in 0..N_SEEKS {
        let target_tick = rng.gen_range(rng_range.clone());
        println!(
            "seeking to {}/{}",
            target_tick,
            parser.demo_stream_mut().total_ticks()?
        );
        let start = Instant::now();
        parser.run_to_tick(target_tick)?;
        println!("seek took {:?}", start.elapsed());
        println!("----");
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
