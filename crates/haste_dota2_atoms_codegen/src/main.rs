use haste_dota2::parser::Parser;
use haste_dota2_atoms_codegen::var_type_idents;
use std::{fs::File, io::BufReader, path::PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const DEM_PATH: &str = "fixtures/auto-20240102-2233-start-____.dem";
const OUT_PATH: &str = "crates/haste_dota2_atoms";

fn inner_main(dem_path: &PathBuf, out_path: &PathBuf) -> Result<()> {
    let file = File::open(dem_path)?;
    let buf_reader = BufReader::new(file);
    let mut parser = Parser::from_reader(buf_reader)?;

    var_type_idents::build(&mut parser, out_path)?;

    Ok(())
}

// NOTE: this is not a build script (build.rs) because that would cause even
// more inconveniences with cyclic dependencies / workarounds.

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let dem_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEM_PATH));
    let out_path = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(OUT_PATH));

    inner_main(&dem_path, &out_path).map_err(|err| {
        eprintln!("error: {:?}", err);
        eprintln!("usage: haste_dota2_atoms_codegen <dempath={dem_path:?}> <outpath={out_path:?}>");
        std::process::exit(42);
    })
}
