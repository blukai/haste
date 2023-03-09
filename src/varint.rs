use crate::error::Result;
use std::io::Read;

#[derive(thiserror::Error, Debug)]
pub enum VarIntError {
    #[error("invalid varint")]
    InvalidVarint,
}

pub trait VarIntRead: Read {
    fn read_varu32(&mut self) -> Result<u32>;
}

impl<T: Read> VarIntRead for T {
    // varu32_from_reader is a port of ReadVarInt32 from valve's demoinfo2/demoinfo.cpp
    // src: https://developer.valvesoftware.com/wiki/Dota_2_Demo_Format
    // TODO: it might be nice to return a tuple of (result, n_read) instead of just result
    fn read_varu32(&mut self) -> Result<u32> {
        let mut result = 0;
        let mut count = 0;
        loop {
            if count == 5 {
                // If we get here it means that the fifth bit had its high bit
                // set, which implies corrupt data.
                return Err(VarIntError::InvalidVarint.into());
            }

            let mut buf = [0u8; 1];
            self.read_exact(&mut buf)?;
            let b = buf[0];

            result |= (b as u32 & 0x7f) << (7 * count);
            count += 1;

            if (b & 0x80) == 0 {
                break;
            }
        }
        Ok(result)
    }
}
