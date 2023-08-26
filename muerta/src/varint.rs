use std::io::Read;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // std
    #[error(transparent)]
    Io(#[from] std::io::Error),
    // mod
    #[error("malformed varint")]
    MalformedVarint,
}

pub type Result<T> = std::result::Result<T, Error>;

pub const CONTINUE_BIT: u32 = 0x80;
pub const PAYLOAD_BITS: u32 = 0x7f;
pub const MAX_VARINT32_BYTES: usize = 5;
pub const MAX_VARINT64_BYTES: usize = 10;

#[inline(always)]
fn read_byte<R: Read>(rdr: &mut R) -> Result<u8> {
    let mut buf = [0u8; 1];
    rdr.read_exact(&mut buf)?;
    Ok(buf[0])
}

// Each byte in the varint has a continuation bit that indicates if the byte
// that follows it is part of the varint. This is the most significant bit (MSB)
// of the byte. The lower 7 bits are a payload; the resulting integer is built
// by appending together the 7-bit payloads of its constituent bytes.
// This allows variable size numbers to be stored with tolerable
// efficiency. Numbers sizes that can be stored for various numbers of
// encoded bits are:
//  8-bits: 0-127
// 16-bits: 128-16383
// 24-bits: 16384-2097151
// 32-bits: 2097152-268435455
// 40-bits: 268435456-0xFFFFFFFF
// TODO: look into faster ways to decode varints:
// - https://github.com/as-com/varint-simd
// - https://github.com/lemire/MaskedVByte
pub fn read_uvarint32<R: Read>(rdr: &mut R) -> Result<(u32, usize)> {
    let mut result = 0;
    for count in 0..=MAX_VARINT32_BYTES {
        let byte = read_byte(rdr)? as u32;
        result |= (byte & PAYLOAD_BITS) << (count * 7);
        if (byte & CONTINUE_BIT) == 0 {
            return Ok((result, count));
        }
    }
    // If we get here it means that the fifth bit had its high bit
    // set, which implies corrupt data.
    Err(Error::MalformedVarint)
}

// NOTE: nameing is somewhat inspired by go's encoding/binary -
// https://pkg.go.dev/encoding/binary

pub fn uvarint32(buf: &[u8]) -> Result<(u32, usize)> {
    let mut result = 0;
    for count in 0..=MAX_VARINT32_BYTES {
        let byte = buf
            .get(count)
            .map(|b| *b as u32)
            .ok_or_else(|| Error::MalformedVarint)?;
        result |= (byte & PAYLOAD_BITS) << (count * 7);
        if (byte & CONTINUE_BIT) == 0 {
            return Ok((result, count));
        }
    }
    Err(Error::MalformedVarint)
}
