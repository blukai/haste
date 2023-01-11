#[allow(clippy::all)]
pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}

mod demfile;
pub use demfile::{DemFile, Visitor};

mod error;
pub use error::Error;

mod bitbuf;
pub(crate) use bitbuf::BitBuf;

mod entity_classes;
pub(crate) use entity_classes::EntityClasses;

mod flattened_serializers;
pub(crate) use flattened_serializers::FlattenedSerializers;

mod varint;
pub(crate) use varint::read_varu32;

mod string_tables;
pub(crate) use string_tables::StringTables;
