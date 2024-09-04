// TODO(blukai): document how to fetch new protos
// $ cd crates/haste-protos/protos
// $ or file in *; curl -LO "https://raw.githubusercontent.com/SteamDatabase/GameTracking-Dota2/master/Protobufs/$file"
// ref: https://discord.com/channels/1275127765879754874/1275127766228009139/1279881501588197377

use std::{env, path::PathBuf};

use which::which;

fn protoc_path() -> Option<PathBuf> {
    env::var_os("PROTOC")
        .map(PathBuf::from)
        .or_else(|| which("protoc").ok())
}

fn main() -> std::io::Result<()> {
    // tell cargo that if the given file changes, to rerun this build script.
    println!("cargo::rerun-if-changed=protos");

    // TODO(blukai): document protoc stuff (readme.me?).
    //
    // do not force people to compile protoc. see if they've got one and attempt to compile if not.
    // refs:
    // - https://discord.com/channels/1275127765879754874/1275127766228009139/1279892327439007784
    // - https://discord.com/channels/1275127765879754874/1275127766228009139/1280983765522452490
    if protoc_path().is_none() {
        env::set_var("PROTOC", protobuf_src::protoc());
    }

    let shared_protos = vec![
        "demo.proto",
        "netmessages.proto",
        "network_connection.proto",
        "networkbasetypes.proto",
        "usermessages.proto",
    ];

    #[cfg(feature = "deadlock")]
    let deadlock_protos = vec![
        "citadel_gcmessages_common.proto",
        "citadel_usermessages.proto",
        "gameevents.proto",
        "gcsdk_gcmessages.proto",
        "steammessages.proto",
        "steammessages_steamlearn.steamworkssdk.proto",
        "steammessages_unified_base.steamworkssdk.proto",
    ];

    #[cfg(feature = "dota2")]
    let dota2_protos = vec![
        "dota_commonmessages.proto",
        "dota_shared_enums.proto",
        "dota_usermessages.proto",
    ];

    let protos = vec![
        shared_protos,
        #[cfg(feature = "deadlock")]
        deadlock_protos,
        #[cfg(feature = "dota2")]
        dota2_protos,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<&str>>();

    let includes = [
        "protos",
        #[cfg(feature = "deadlock")]
        "protos/deadlock",
        #[cfg(feature = "dota2")]
        "protos/dota2",
    ];

    prost_build::compile_protos(&protos, &includes)
}
