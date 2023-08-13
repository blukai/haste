#![feature(allocator_api)]
#![feature(const_for)]

#[allow(clippy::all)]
pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}

pub mod allocstring;
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
pub mod quantizedfloat;
pub mod stringtables;
pub mod varint;
