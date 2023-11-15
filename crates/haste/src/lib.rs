#![feature(const_for)]
#![feature(core_intrinsics)]

// 3rd party crate re-exports
pub use prost;

// own crate re-exports
pub use dota2protos;

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
pub(crate) mod nohash;
pub mod parser;
pub mod quantizedfloat;
pub mod stringtables;
pub mod varint;

// TOOD: more optimizations, specifically look into
// https://agourlay.github.io/rust-performance-retrospective-part2/
