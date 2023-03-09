#![feature(allocator_api, error_generic_member_access, provide_any)]

#[allow(clippy::all)]
pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}

mod bitreader;
mod dem;
mod entity_classes;
mod error;
mod flattened_serializers;
mod packet;
mod packet_entitiy;
pub mod parser;
mod string_tables;
mod varint;
