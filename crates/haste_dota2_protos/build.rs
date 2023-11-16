fn main() -> std::io::Result<()> {
    let dir_entries = std::fs::read_dir("./protos")?;
    let proto_paths = dir_entries
        .into_iter()
        .map(|path| Ok(path?.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    prost_build::compile_protos(&proto_paths, &["./protos"])
}
