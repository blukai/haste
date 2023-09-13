mod protos {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}
pub use protos::*;
