#![feature(const_for)]
#![feature(core_intrinsics)]

// own crate re-exports
pub use haste_dota2_protos as dota2_protos;

// TODO: figure pub scopes for all the things
pub mod bitbuf;
pub mod demofile;
pub mod entities;
pub mod entityclasses;
pub mod fielddecoder;
pub mod fieldmetadata;
pub mod fieldpath;
pub mod fieldvalue;
pub mod flattenedserializers;
pub mod fnv1a;
pub mod instancebaseline;
pub mod nohash;
pub mod parser;
pub mod quantizedfloat;
pub mod stringtables;
pub mod varint;

// TOOD: more optimizations, specifically look into
// https://agourlay.github.io/rust-performance-retrospective-part2/

// TODO: change type of buf from &[u8] to Bytes to maybe avoid some copying; see
// https://github.com/tokio-rs/prost/issues/571. or maybe look into zerycopy
// thingie https://github.com/google/zerocopy
