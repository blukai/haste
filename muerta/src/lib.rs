#![feature(allocator_api, error_generic_member_access, provide_any, is_some_and)]

#[allow(clippy::all)]
pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}

#[allow(non_snake_case)]
pub mod fops {
    include!(concat!(env!("OUT_DIR"), "/fops.rs"));
}

pub mod bitreader;
mod client;
mod dem;
mod entity_classes;
pub mod error;
pub mod field_path;
mod field_prop;
mod flattened_serializers;
mod packet;
pub mod parser;
mod quantized_float;
mod string_tables;
mod varint;
