use anyhow::Result;
use muerta::{self, Visitor};

struct Dem;

impl Visitor for Dem {}

fn main() -> Result<()> {
    let mut dem_file = muerta::DemFile::open("./fixtures/6911306644_1806469309.dem", Dem {})?;
    dem_file.parse()?;
    Ok(())
}
