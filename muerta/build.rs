use std::fs;
use std::io::Result;

fn main() -> Result<()> {
    let paths = fs::read_dir("../protos")?;
    let protos = paths
        .into_iter()
        .map(|path| Ok(path?.path()))
        .collect::<Result<Vec<_>>>()?;
    prost_build::compile_protos(&protos, &["../protos"])?;
    Ok(())
}
