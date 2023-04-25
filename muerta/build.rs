use std::env;
use std::fs;
use std::io::{Result, Write};
use std::path;

fn main() -> Result<()> {
    let paths = fs::read_dir("../protos")?;
    let protos = paths
        .into_iter()
        .map(|path| Ok(path?.path()))
        .collect::<Result<Vec<_>>>()?;
    prost_build::compile_protos(&protos, &["../protos"])?;

    // ----

    let fops = muerta_codgen::build_fops();
    let target: path::PathBuf = env::var_os("OUT_DIR").expect("out dir").into();
    let mut file = fs::File::create(target.join("fops.rs"))?;
    write!(&mut file, "{}", fops.to_string())?;
    file.flush()
}
