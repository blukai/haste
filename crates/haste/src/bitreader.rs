use dungers::bitbuf;

// NOCOMMIT
pub type Error = bitbuf::Error;

pub type Result<T> = std::result::Result<T, Error>;

// public/coordsize.h
const COORD_INTEGER_BITS: usize = 14;
const COORD_FRACTIONAL_BITS: usize = 5;
const COORD_DENOMINATOR: f32 = (1 << COORD_FRACTIONAL_BITS) as f32;
const COORD_RESOLUTION: f32 = 1.0 / COORD_DENOMINATOR;

// public/coordsize.h
const NORMAL_FRACTIONAL_BITS: usize = 11;
const NORMAL_DENOMINATOR: f32 = ((1 << (NORMAL_FRACTIONAL_BITS)) - 1) as f32;
const NORMAL_RESOLUTION: f32 = 1.0 / (NORMAL_DENOMINATOR);

// BitRead is a port of valve's CBitRead(or/and old_bf_read) from valve's tier1 lib.
pub struct BitReader<'d> {
    inner: bitbuf::BitReader<'d>,
}

impl<'d> BitReader<'d> {
    pub fn new(data: &'d [u8]) -> Self {
        Self {
            inner: bitbuf::BitReader::new(data),
        }
    }

    #[inline(always)]
    pub fn num_bits_left(&self) -> usize {
        self.inner.num_bits_left()
    }

    #[inline(always)]
    pub fn num_bytes_left(&self) -> usize {
        self.inner.num_bytes_left()
    }

    #[inline(always)]
    pub fn read_ubitlong(&mut self, num_bits: usize) -> Result<u32> {
        debug_assert!(num_bits < 33, "trying to read more than 32 bits");

        // NOCOMMIT
        self.inner
            .read_ubit64(num_bits)
            .map(|v| v as u32)
            .map_err(Error::from)
    }

    // ubitvar is "valve's own variable-length integer encoding" (c) butterfly.
    //
    // valve's refs:
    // - [1] https://github.com/ValveSoftware/csgo-demoinfo/blob/049f8dbf49099d3cc544ec5061a7f7252cce7b82/demoinfogo/demofilebitbuf.cpp#L171
    // - [2]: https://github.com/ValveSoftware/source-sdk-2013/blob/0d8dceea4310fde5706b3ce1c70609d72a38efdf/sp/src/public/tier1/bitbuf.h#L756
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
    #[inline(always)]
    pub fn read_ubitvar(&mut self) -> Result<u32> {
        let ret = self.read_ubitlong(6)?;
        let v = match ret & (16 | 32) {
            16 => (ret & 15) | (self.read_ubitlong(4)? << 4),
            32 => (ret & 15) | (self.read_ubitlong(8)? << 4),
            48 => (ret & 15) | (self.read_ubitlong(32 - 4)? << 4),
            _ => ret,
        };
        Ok(v)
    }

    #[inline(always)]
    pub fn read_bitfloat(&mut self) -> Result<f32> {
        self.read_ubitlong(32).map(f32::from_bits)
    }

    pub fn read_bitcoord(&mut self) -> Result<f32> {
        let mut value: f32 = 0.0;

        // Read the required integer and fraction flags
        let has_intval = self.read_bool()?;
        let has_fractval = self.read_bool()?;

        // If we got either parse them, otherwise it's a zero.
        if has_intval || has_fractval {
            // Read the sign bit
            let signbit = self.read_bool()?;

            // If there's an integer, read it in
            let mut intval = 0;
            if has_intval {
                // Adjust the integers from [0..MAX_COORD_VALUE-1] to [1..MAX_COORD_VALUE]
                intval = self.read_ubitlong(COORD_INTEGER_BITS)? + 1;
            }

            // If there's a fraction, read it in
            let mut fractval = 0;
            if has_fractval {
                fractval = self.read_ubitlong(COORD_FRACTIONAL_BITS)?;
            }

            // Calculate the correct floating point value
            value = intval as f32 + (fractval as f32 * COORD_RESOLUTION);

            // Fixup the sign if negative.
            if signbit {
                value = -value;
            }
        }

        Ok(value)
    }

    pub fn read_bitnormal(&mut self) -> Result<f32> {
        // read the sign bit
        let signbit = self.read_bool()?;

        // read the fractional part
        let fractval = self.read_ubitlong(NORMAL_FRACTIONAL_BITS)?;

        // calculate the correct floating point value
        let mut value = fractval as f32 * NORMAL_RESOLUTION;

        // fixup the sign if negative.
        if signbit {
            value = -value;
        }

        return Ok(value);
    }

    pub fn read_bitvec3coord(&mut self) -> Result<[f32; 3]> {
        let mut fa = [0f32; 3];

        let xflag = self.read_bool()?;
        let yflag = self.read_bool()?;
        let zflag = self.read_bool()?;

        if xflag {
            fa[0] = self.read_bitcoord()?;
        }
        if yflag {
            fa[1] = self.read_bitcoord()?;
        }
        if zflag {
            fa[2] = self.read_bitcoord()?;
        }

        Ok(fa)
    }

    pub fn read_bitvec3normal(&mut self) -> Result<[f32; 3]> {
        let mut fa = [0f32; 3];

        let xflag = self.read_bool()?;
        let yflag = self.read_bool()?;

        if xflag {
            fa[0] = self.read_bitnormal()?;
        }
        if yflag {
            fa[1] = self.read_bitnormal()?;
        }

        // the first two imply the third (but not its sign)
        let znegative = self.read_bool()?;

        let fafafbfb = fa[0] * fa[0] + fa[1] * fa[1];
        if fafafbfb < 1.0 {
            fa[2] = (1.0 - fafafbfb).sqrt();
        }

        if znegative {
            fa[2] = -fa[2];
        }

        Ok(fa)
    }

    pub fn read_bytes(&mut self, buf: &mut [u8]) -> Result<()> {
        self.inner.read_bytes(buf)
    }

    pub fn read_bitangle(&mut self, num_bits: usize) -> Result<f32> {
        let shift = bitbuf::get_bit_for_bit_num(num_bits) as f32;

        let u = self.read_ubitlong(num_bits)?;
        let ret = (u as f32) * (360.0 / shift);

        Ok(ret)
    }

    #[inline(always)]
    pub fn read_bool(&mut self) -> Result<bool> {
        self.inner.read_bool()
    }

    #[inline(always)]
    pub fn read_byte(&mut self) -> Result<u8> {
        self.inner.read_byte()
    }

    pub fn read_bits(&mut self, buf: &mut [u8], num_bits: usize) -> Result<()> {
        self.inner.read_bits(buf, num_bits)
    }

    // Returns [dungers::varint::Error::BufferTooSmall] if buf isn't large enough to hold the
    // string.
    //
    // Always reads to the end of the string (so you can read the next piece of data waiting).
    //
    // If line is true, it stops when it reaches a '\n' or a null-terminator.
    //
    // buf is always null-terminated (unless buf.len() is 0).
    //
    // Returns the number of characters left in out when the routine is complete (this will never
    // exceed buf.len()-1).
    pub fn read_string(&mut self, buf: &mut [u8], line: bool) -> Result<usize> {
        debug_assert!(!buf.is_empty());

        let mut too_small = false;
        let mut num_chars = 0;
        loop {
            let val = self.read_byte()?;
            if val == 0 || (line && val == b'\n') {
                break;
            }

            if num_chars < (buf.len() - 1) {
                buf[num_chars] = val;
                num_chars += 1;
            } else {
                too_small = true;
            }
        }

        // Make sure it's null-terminated.
        debug_assert!(num_chars < buf.len());
        buf[num_chars] = 0;

        if too_small {
            Err(Error::BufferTooSmall)
        } else {
            Ok(num_chars)
        }
    }

    pub fn read_uvarint32(&mut self) -> Result<u32> {
        self.inner.read_uvarint32()
    }

    pub fn read_uvarint64(&mut self) -> Result<u64> {
        self.inner.read_uvarint64()
    }

    pub fn read_varint32(&mut self) -> Result<i32> {
        self.inner.read_varint32()
    }

    pub fn read_varint64(&mut self) -> Result<i64> {
        self.inner.read_varint64()
    }

    pub fn read_ubitvarfp(&mut self) -> Result<u32> {
        if self.read_bool()? {
            self.read_ubitlong(2)
        } else if self.read_bool()? {
            self.read_ubitlong(4)
        } else if self.read_bool()? {
            self.read_ubitlong(10)
        } else if self.read_bool()? {
            self.read_ubitlong(17)
        } else {
            self.read_ubitlong(31)
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_read_string() -> super::Result<()> {
        let buf = b"Life's but a walking shadow, a poor player.\0";
        let mut br = super::BitReader::new(buf);

        let mut out = vec![0u8; buf.len()];
        let num_chars = br.read_string(&mut out, false)?;
        assert_eq!(&out, &buf);
        assert_eq!(num_chars, buf.len() - 1);

        Ok(())
    }
}
