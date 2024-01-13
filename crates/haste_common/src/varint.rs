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

// NOTE: nameing is somewhat inspired by go's encoding/binary -
// https://pkg.go.dev/encoding/binary

pub const CONTINUE_BIT: u8 = 0x80;
pub const PAYLOAD_BITS: u8 = 0x7f;
pub const MAX_VARINT32_BYTES: usize = 5;
pub const MAX_VARINT64_BYTES: usize = 10;

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
//
// TODO: look into faster ways to decode varints:
// - https://github.com/as-com/varint-simd
// - https://github.com/lemire/MaskedVByte
pub fn read_uvarint32<R: Read>(rdr: &mut R) -> Result<(u32, usize)> {
    let mut result = 0;
    let mut buf = [0u8; 1];
    for count in 0..=MAX_VARINT32_BYTES {
        rdr.read_exact(&mut buf)?;
        // SAFELY: this is completely safe
        let byte = unsafe { *buf.get_unchecked(0) };
        result |= ((byte & PAYLOAD_BITS) as u32) << (count * 7);
        if (byte & CONTINUE_BIT) == 0 {
            return Ok((result, count));
        }
    }
    // If we get here it means that the fifth bit had its high bit
    // set, which implies corrupt data.
    Err(Error::MalformedVarint)
}

// stolen from csgo public/tier1/bitbuf.h only decoders, encoders aren't here.
//
// ZigZag Transform:  Encodes signed integers so that they can be effectively
// used with varint encoding.
//
// varint operates on unsigned integers, encoding smaller numbers into fewer
// bytes.  If you try to use it on a signed integer, it will treat this number
// as a very large unsigned integer, which means that even small signed numbers
// like -1 will take the maximum number of bytes (10) to encode.  ZigZagEncode()
// maps signed integers to unsigned in such a way that those with a small
// absolute value will have smaller encoded values, making them appropriate for
// encoding using varint.
//
//       int32 ->     uint32
// -------------------------
//           0 ->          0
//          -1 ->          1
//           1 ->          2
//          -2 ->          3
//         ... ->        ...
//  2147483647 -> 4294967294 -2147483648 -> 4294967295
//
//        >> encode >>
//        << decode <<
#[inline(always)]
pub fn zigzag_decode32(n: u32) -> i32 {
    (n >> 1) as i32 ^ -((n & 1) as i32)
}

#[inline(always)]
pub fn zigzag_decode64(n: u64) -> i64 {
    (n >> 1) as i64 ^ -((n & 1) as i64)
}
