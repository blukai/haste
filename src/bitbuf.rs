use anyhow::{anyhow, Result};
use std::io::{Cursor, Read};

use crate::Error;

pub struct BitBuf<'d> {
    data: Cursor<&'d [u8]>,
    data_len: usize,
    bit_count: u32,
    bit_value: u32,
}

impl<'d> BitBuf<'d> {
    pub fn new(data: &'d [u8]) -> Self {
        Self {
            data: Cursor::new(data),
            data_len: data.len(),
            bit_count: 0,
            bit_value: 0,
        }
    }

    pub fn read_bytes(&mut self, buf: &mut [u8]) -> Result<()> {
        if self.bit_count == 0 {
            self.data.read_exact(buf)?;
        } else {
            for elem in buf.iter_mut() {
                *elem = self.read(8)? as u8;
            }
        }
        Ok(())
    }

    // NOTE: read is based on manta's reader.readBits method (/reader.go)
    // NOTE: this function can read a maximum of 32 bits at once.
    pub fn read(&mut self, n: u32) -> Result<u32> {
        while n > self.bit_count {
            let mut buf = [0u8; 1];
            self.data.read_exact(&mut buf)?;
            let b = buf[0] as u32;

            self.bit_value |= b << self.bit_count;
            self.bit_count += 8;
        }

        let v = self.bit_value & ((1 << n) - 1);
        self.bit_value >>= n;
        self.bit_count -= n;

        Ok(v)
    }

    // ubitvar is supposedly "valve's own variable-length integer encoding" (c) butterfly.
    // valve's refs:
    // - src: https://github.com/ValveSoftware/csgo-demoinfo/blob/049f8dbf49099d3cc544ec5061a7f7252cce7b82/demoinfogo/demofilebitbuf.cpp#L171
    // - alt src (possible faster): https://github.com/ValveSoftware/source-sdk-2013/blob/0d8dceea4310fde5706b3ce1c70609d72a38efdf/sp/src/public/tier1/bitbuf.h#L756
    // NOTE: butterfly, manta and clarity - all of them have this same exact implementation.
    pub fn read_ubitvar(&mut self) -> Result<u32> {
        let ret = self.read(6)?;
        let v = match ret & 0x30 {
            16 => (ret & 15) | (self.read(4)? << 4),
            32 => (ret & 15) | (self.read(8)? << 4),
            48 => (ret & 15) | (self.read(28)? << 4),
            _ => ret,
        };
        Ok(v)
    }

    pub fn read_varu32(&mut self) -> Result<u32> {
        let mut result = 0;
        let mut count = 0;
        loop {
            if count == 5 {
                return Err(anyhow!(Error::InvalidVarint));
            }

            let mut buf = [0u8; 1];
            self.read_bytes(&mut buf)?;
            let b = buf[0] as u32;

            result |= (b & 0x7f) << (7 * count);
            count += 1;

            if (b & 0x80) == 0 {
                break;
            }
        }
        Ok(result)
    }

    pub fn is_empty(&mut self) -> bool {
        self.data.position() >= self.data_len as u64
    }

    pub fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read(1)? == 1)
    }

    // read_str reads a null-terminated string into the buffer, stops once it
    // reaches \0 or end of buffer. Err will be returned in case an overflow.
    pub fn read_str<'b>(&mut self, buf: &'b mut [u8]) -> Result<&'b [u8]> {
        for i in 0..buf.len() {
            self.read_bytes(&mut buf[i..i + 1])?;
            if buf[i] == 0 {
                return Ok(&buf[..i]);
            }
        }
        Err(anyhow!(Error::BufferOverflow))
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
}
