#[derive(Debug)]
pub enum EntityOp {
    Update = 0,
    Leave,
    Create,
    Delete,
}

impl From<u32> for EntityOp {
    fn from(value: u32) -> Self {
        use EntityOp::*;
        // following logic is stolen from manta
        if value & 0x01 == 0 {
            if value & 0x02 != 0 {
                return Create;
            } else {
                return Update;
            }
        } else {
            if value & 0x02 != 0 {
                return Delete;
            } else {
                return Leave;
            }
        }
    }
}

pub struct PacketEntity {}

impl PacketEntity {}
