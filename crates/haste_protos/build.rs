fn main() -> std::io::Result<()> {
    std::env::set_var("PROTOC", protobuf_src::protoc());

    let shared_protos = vec![
        "protos/demo.proto",
        "protos/netmessages.proto",
        "protos/network_connection.proto",
        "protos/networkbasetypes.proto",
        "protos/usermessages.proto",
    ];

    #[cfg(feature = "dota2")]
    let dota2_protos = vec![
        "dota_commonmessages.proto",
        "dota_shared_enums.proto",
        "dota_usermessages.proto",
    ];

    let protos = vec![
        shared_protos,
        #[cfg(feature = "dota2")]
        dota2_protos,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<&str>>();

    let includes = ["protos"];

    prost_build::compile_protos(&protos, &includes)
}
