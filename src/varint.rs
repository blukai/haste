use crate::Error;
use anyhow::{anyhow, Result};
use std::io::Read;

// read_varu32 is a port of ReadVarInt32 from valve's demoinfo2/demoinfo.cpp
// src: https://developer.valvesoftware.com/wiki/Dota_2_Demo_Format
pub fn read_varu32(r: &mut impl Read) -> Result<u32> {
    let mut result = 0;
    let mut count = 0;
    loop {
        if count == 5 {
            return Err(anyhow!(Error::InvalidVarint));
        }

        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        let b = buf[0] as u32;

        result |= (b as u32 & 0x7f) << (7 * count);
        count += 1;

        if (b & 0x80) == 0 {
            break;
        }
    }
    Ok(result)
}
