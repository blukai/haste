#![feature(core_intrinsics)]

// own crate re-exports
pub use haste_common::bitbuf;
pub use haste_common::fxhash;
pub use haste_common::varint;
pub use haste_dota2_deflat as dota2_deflat;
pub use haste_dota2_protos as dota2_protos;

// TODO: figure pub scopes for all the things
pub mod demofile;
pub mod entities;
pub mod entityclasses;
pub mod fielddecoder;
pub mod fieldmetadata;
pub mod fieldpath;
pub mod fieldvalue;
pub mod flattenedserializers;
pub mod instancebaseline;
pub mod parser;
pub mod quantizedfloat;
pub mod stringtables;

// TOOD: more optimizations, specifically look into
// https://agourlay.github.io/rust-performance-retrospective-part2/

// TODO: change type of buf from &[u8] to Bytes to maybe avoid some copying; see
// https://github.com/tokio-rs/prost/issues/571. or maybe look into zerycopy
// thingie https://github.com/google/zerocopy

// TODO: compose performance comparisons (manta and clarity); use ticks per
// second as metric (inspired by
// https://github.com/markus-wa/demoinfocs-golang?tab=readme-ov-file#performance--benchmarks).

// TODO: don't ignore length of vectors (dynamic length arrays) because they may
// contain garbage; see
// https://github.com/markus-wa/demoinfocs-golang/issues/450 for details.

// TODO: generate list of entities (/flattened serializers) where it'll be
// possible to get "the thing" by name hash and construct it.
// probably use RecvTable and RecvProp "terms".
// refs?
// - https://developer.valvesoftware.com/wiki/Networking_Entities
// - public/dt_recv.h
// - engine/dt_recv_eng.h

// TODO: abilities, items, heroes, modifiers, etc.. are represented as strings
// in replays, for example "dota_npc_hero_zuus" (according to Ken); string
// comparisons are expensive. valve mostly uses CUtlStringToken which computes a
// hash and uses it for comparisons, it discards the string - this is nice. see
// public/tier1/utlstringtoken.h
