use anyhow::Result;
use muerta;

fn main() -> Result<()> {
    let mut dem_file = muerta::DemFile::open("./fixtures/6911306644_1806469309.dem", ())?;
    let file_info = dem_file.get_file_info()?;
    println!("{:#?}", file_info);
    Ok(())
}
