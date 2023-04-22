use crate::error::Result;
use std::io::{Cursor, Read};

const VARINT32_MAX_BYTES: usize = 5;
const VARINT64_MAX_BYTES: usize = 10;

// coord consts are stolen from csgo public/coordsize.h
// OVERALL Coordinate Size Limits used in COMMON.C MSG_*BitCoord() Routines (and someday the HUD)
const COORD_INTEGER_BITS: u32 = 14;
const COORD_FRACTIONAL_BITS: u32 = 5;
const COORD_DENOMINATOR: u32 = 1 << (COORD_FRACTIONAL_BITS);
const COORD_RESOLUTION: f32 = 1.0 / COORD_DENOMINATOR as f32;

#[derive(thiserror::Error, Debug)]
pub enum BitReaderError {
    #[error("malformed varint")]
    MalformedVarint,
    #[error("buffer overflow")]
    BufferOverflow,
}

// TODO: can we create a trait and implement it for &[u8]?
// to also implement additional methods from outside.

#[derive(Debug)]
pub struct BitReader<'d> {
    cursor: Cursor<&'d [u8]>,
    data_len: usize,
    bit_count: u32,
    bit_value: u64,
}

impl<'d> BitReader<'d> {
    pub fn new(data: &'d [u8]) -> Self {
        Self {
            cursor: Cursor::new(data),
            data_len: data.len(),
            bit_count: 0,
            bit_value: 0,
        }
    }

    // NOTE: read is based on manta's reader.readBits method (/reader.go)
    #[inline(always)]
    pub fn read(&mut self, n: u32) -> Result<u32> {
        while n > self.bit_count {
            let mut buf = [0u8; 1];
            self.cursor.read_exact(&mut buf)?;

            self.bit_value |= (buf[0] as u64) << self.bit_count;
            self.bit_count += 8;
        }

        let v = self.bit_value & ((1 << n) - 1);
        self.bit_value >>= n;
        self.bit_count -= n;

        Ok(v as u32)
    }

    // read_bits reads the exact number of bits into the buffer. The function reads
    // in chunks of 8 bit until n is smaller than that and appends the left over
    // bits
    pub fn read_bits(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut remaining = buf.len();
        let mut i = 0;
        while remaining >= 8 {
            buf[i] = self.read(8)? as u8;
            remaining -= 8;
            i += 1;
        }
        if remaining > 8 {
            buf[i] = self.read(remaining as u32)? as u8;
        }
        Ok(())
    }

    pub fn read_bytes(&mut self, buf: &mut [u8]) -> Result<()> {
        if self.bit_count == 0 {
            self.cursor.read_exact(buf)?;
        } else {
            for elem in buf.iter_mut() {
                *elem = self.read(8)? as u8;
            }
        }
        Ok(())
    }

    pub fn skip_bytes(&mut self, n: u64) -> Result<()> {
        if self.bit_count == 0 {
            self.cursor.set_position(self.cursor.position() + n);
        } else {
            self.cursor.set_position(self.cursor.position() + n - 1);
            let n_bits = 8 - self.bit_count;
            self.bit_count = 0;
            self.bit_value = 0;
            self.read(n_bits)?;
        }
        Ok(())
    }

    // ubitvar is "valve's own variable-length integer encoding" (c) butterfly.
    //
    // valve's refs:
    // - src: https://github.com/ValveSoftware/csgo-demoinfo/blob/049f8dbf49099d3cc544ec5061a7f7252cce7b82/demoinfogo/demofilebitbuf.cpp#L171
    // - alt src (possible faster): https://github.com/ValveSoftware/source-sdk-2013/blob/0d8dceea4310fde5706b3ce1c70609d72a38efdf/sp/src/public/tier1/bitbuf.h#L756
    //
    // NOTE: butterfly, manta and clarity - all have same exact implementation.
    //
    // quote from clarity:
    // Thanks to Robin Dietrich for providing a clean version of this code :-)
    // The header looks like this: [XY00001111222233333333333333333333] where everything > 0 is optional.
    // The first 2 bits (X and Y) tell us how much (if any) to read other than the 6 initial bits:
    // Y set -> read 4
    // X set -> read 8
    // X + Y set -> read 28
    pub fn read_ubitvar(&mut self) -> Result<u32> {
        let ret = self.read(6)?;
        let v = match ret & 48 {
            16 => (ret & 15) | (self.read(4)? << 4),
            32 => (ret & 15) | (self.read(8)? << 4),
            48 => (ret & 15) | (self.read(28)? << 4),
            _ => ret,
        };
        Ok(v)
    }

    // copypasta from csgo tier1/bitbuf.cpp
    //
    // Read 1-5 bytes in order to extract a 32-bit unsigned value from the
    // stream. 7 data bits are extracted from each byte with the 8th bit used
    // to indicate whether the loop should continue.
    // This allows variable size numbers to be stored with tolerable
    // efficiency. Numbers sizes that can be stored for various numbers of
    // encoded bits are:
    //  8-bits: 0-127
    // 16-bits: 128-16383
    // 24-bits: 16384-2097151
    // 32-bits: 2097152-268435455
    // 40-bits: 268435456-0xFFFFFFFF
    //
    // TODO: look into https://github.com/HebiRobotics/QuickBuffers/blob/main/runtime/src/main/java/us/hebi/quickbuf/ProtoSource.java#L802-L880
    // that is derived from https://github.com/protocolbuffers/protobuf/blob/main/java/core/src/main/java/com/google/protobuf/CodedInputStream.java#L978-L1118
    // discovered in: https://github.com/HebiRobotics/QuickBuffers/issues/40#issuecomment-1426898262
    pub fn read_varu32(&mut self) -> Result<u32> {
        let mut result: u32 = 0;
        let mut count = 0;

        loop {
            if count == VARINT32_MAX_BYTES {
                // If we get here it means that the fifth bit had its high bit
                // set, which implies corrupt data.
                return Err(BitReaderError::MalformedVarint.into());
            }

            let mut buf = [0u8; 1];
            self.read_bytes(&mut buf)?;
            let b = buf[0];

            result |= (b as u32 & 0x7f) << (7 * count);
            count += 1;

            if (b & 0x80) == 0 {
                break;
            }
        }

        Ok(result)
    }

    pub fn read_varu64(&mut self) -> Result<u64> {
        let mut result: u64 = 0;
        let mut count = 0;

        loop {
            if count == VARINT64_MAX_BYTES {
                return Err(BitReaderError::MalformedVarint.into());
            }

            let mut buf = [0u8; 1];
            self.read_bytes(&mut buf)?;
            let b = buf[0];

            result |= (b as u64 & 0x7F) << (7 * count);
            count += 1;

            if (b & 0x80) == 0 {
                break;
            }
        }

        Ok(result)
    }

    // stolen from csgo public/tier1/bitbuf.h
    // only decoders, encoders aren't here.
    //
    // ZigZag Transform:  Encodes signed integers so that they can be
    // effectively used with varint encoding.
    //
    // varint operates on unsigned integers, encoding smaller numbers into
    // fewer bytes.  If you try to use it on a signed integer, it will treat
    // this number as a very large unsigned integer, which means that even
    // small signed numbers like -1 will take the maximum number of bytes
    // (10) to encode.  ZigZagEncode() maps signed integers to unsigned
    // in such a way that those with a small absolute value will have smaller
    // encoded values, making them appropriate for encoding using varint.
    //
    //       int32 ->     uint32
    // -------------------------
    //           0 ->          0
    //          -1 ->          1
    //           1 ->          2
    //          -2 ->          3
    //         ... ->        ...
    //  2147483647 -> 4294967294
    // -2147483648 -> 4294967295
    //
    //        >> encode >>
    //        << decode <<
    #[inline(always)]
    fn zig_zag_decode32(n: u32) -> i32 {
        (n >> 1) as i32 ^ -((n & 1) as i32)
    }

    #[inline(always)]
    fn zig_zag_decode64(n: u64) -> i64 {
        (n >> 1) as i64 ^ -((n & 1) as i64)
    }

    pub fn read_vari32(&mut self) -> Result<i32> {
        self.read_varu32().map(Self::zig_zag_decode32)
    }

    pub fn read_vari64(&mut self) -> Result<i64> {
        self.read_varu64().map(Self::zig_zag_decode64)
    }

    pub fn is_empty(&mut self) -> bool {
        self.cursor.position() >= self.data_len as u64
    }

    pub fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read(1)? == 1)
    }

    // read_str reads a null-terminated string into the buffer, stops once it
    // reaches \0 or end of buffer. Err will be returned in case an overflow.
    pub fn read_string<'b>(&mut self, buf: &'b mut [u8]) -> Result<&'b [u8]> {
        for i in 0..buf.len() {
            self.read_bytes(&mut buf[i..i + 1])?;
            if buf[i] == 0 {
                return Ok(&buf[..i]);
            }
        }
        Err(BitReaderError::BufferOverflow.into())
    }

    // read_bit_coord is stolen from csgo tier1/bitbuf.cpp, and ported to rust
    // by chatgpt, fixed by me
    pub fn read_coord(&mut self) -> Result<f32> {
        // Read the required integer and fraction flags
        let mut int_val = self.read_bool()? as u32;
        let mut fract_val = self.read_bool()? as u32;

        let mut value = 0.0;

        // If we got either parse them, otherwise it's a zero.
        if int_val != 0 || fract_val != 0 {
            // Read the sign bit
            let sign_bit = self.read_bool()?;

            // If there's an integer, read it in
            if int_val != 0 {
                // Adjust the integers from [0..MAX_COORD_VALUE-1] to [1..MAX_COORD_VALUE]
                int_val = self.read(COORD_INTEGER_BITS)? + 1;
            }

            // If there's a fraction, read it in
            if fract_val != 0 {
                fract_val = self.read(COORD_FRACTIONAL_BITS)?;
            }

            // Calculate the correct floating point value
            value = int_val as f32 + (fract_val as f32 * COORD_RESOLUTION);

            // Fixup the sign if negative.
            if sign_bit {
                value = -value;
            }
        }

        Ok(value)
    }

    // read_fpbitbar Reads a fieldpath varint
    // stolen from butterfly
    pub fn read_fpbitvar(&mut self) -> Result<i32> {
        Ok(if self.read_bool()? {
            self.read(2)
        } else if self.read_bool()? {
            self.read(4)
        } else if self.read_bool()? {
            self.read(10)
        } else if self.read_bool()? {
            self.read(17)
        } else {
            self.read(31)
        }? as i32)
    }
}
