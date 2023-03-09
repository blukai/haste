use anyhow::Result;
use bumpalo::Bump;
use muerta::{self, Visitor};

struct Dem;

impl Visitor for Dem {}

fn main() -> Result<()> {
    let bump = Bump::with_capacity(1024);
    let mut dem_file =
        muerta::DemFile::open_in("./fixtures/6911306644_1806469309.dem", Dem {}, &bump)?;
    dem_file.parse()?;
    Ok(())
}
