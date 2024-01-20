fn main() -> std::io::Result<()> {
    std::env::set_var("PROTOC", protobuf_src::protoc());
    let proto_paths = std::fs::read_dir("./protos")?
        .into_iter()
        .map(|path| Ok(path?.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    prost_build::compile_protos(&proto_paths, &["./protos"])
}
