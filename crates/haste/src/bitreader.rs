use dungers::bitbuf;

pub use bitbuf::Error as BitReaderError;

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
pub struct BitReader<'a> {
    inner: bitbuf::BitReader<'a>,
    did_check_overflow: bool,
}

impl<'a> Drop for BitReader<'a> {
    #[inline]
    fn drop(&mut self) {
        assert!(self.did_check_overflow, "when you are done reading, you must call `is_overflowed` to ensure that there were no out of bounds reads");
    }
}

/// rationale for using "unsafe" `_unchecked` methods of the underlying
/// [dungers::bitbuf::BitReader]:
///
/// what makes safe methods of [dungers::bitbuf::BitReader] safe is overflow checks.
///
/// bounds checking is not omitted, it is "deferred". custom [Drop] impl helps to ensure that it is
/// performed.
///
/// [BitReader]'s methods are being called very frequently, there's absolutely no value in
/// performing bounds checking each time something is needed to be read because that is not going
/// to help detect corrupt data.
///
/// deferred bounds checking allows to eliminate a very significant amout of branches which very
/// noticeably improves performance.
impl<'a> BitReader<'a> {
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            inner: bitbuf::BitReader::new(data),
            did_check_overflow: false,
        }
    }

    /// delegated from [dungers::bitbuf::BitReader].
    #[inline(always)]
    pub fn num_bits_left(&self) -> usize {
        self.inner.num_bits_left()
    }

    /// delegated from [dungers::bitbuf::BitReader].
    #[inline(always)]
    pub fn read_ubit64(&mut self, num_bits: usize) -> u64 {
        unsafe { self.inner.read_ubit64_unchecked(num_bits) }
    }

    /// delegated from [dungers::bitbuf::BitReader].
    #[inline(always)]
    pub fn read_bool(&mut self) -> bool {
        unsafe { self.inner.read_bool_unchecked() }
    }

    /// delegated from [dungers::bitbuf::BitReader].
    #[inline(always)]
    pub fn read_byte(&mut self) -> u8 {
        unsafe { self.inner.read_byte_unchecked() }
    }

    /// delegated from [dungers::bitbuf::BitReader].
    pub fn read_bits(&mut self, buf: &mut [u8], num_bits: usize) {
        unsafe { self.inner.read_bits_unchecked(buf, num_bits) }
    }

    /// delegated from [dungers::bitbuf::BitReader].
    pub fn read_bytes(&mut self, buf: &mut [u8]) {
        unsafe { self.inner.read_bytes_unchecked(buf) }
    }

    #[inline]
    pub fn is_overflowed(&mut self) -> bitbuf::Result<()> {
        self.did_check_overflow = true;
        self.inner.is_overflowed()
    }

    /// delegated from [dungers::bitbuf::BitReader].
    pub fn read_uvarint32(&mut self) -> u32 {
        unsafe { self.inner.read_uvarint32_unchecked() }
    }

    /// delegated from [dungers::bitbuf::BitReader].
    pub fn read_uvarint64(&mut self) -> u64 {
        unsafe { self.inner.read_uvarint64_unchecked() }
    }

    /// delegated from [dungers::bitbuf::BitReader].
    pub fn read_varint32(&mut self) -> i32 {
        unsafe { self.inner.read_varint32_unchecked() }
    }

    /// delegated from [dungers::bitbuf::BitReader].
    pub fn read_varint64(&mut self) -> i64 {
        unsafe { self.inner.read_varint64_unchecked() }
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
    pub fn read_ubitvar(&mut self) -> u32 {
        let ret = self.read_ubit64(6);
        let ret = match ret & (16 | 32) {
            16 => (ret & 15) | (self.read_ubit64(4) << 4),
            32 => (ret & 15) | (self.read_ubit64(8) << 4),
            48 => (ret & 15) | (self.read_ubit64(32 - 4) << 4),
            _ => ret,
        };
        ret as u32
    }

    #[inline(always)]
    pub fn read_bitfloat(&mut self) -> f32 {
        f32::from_bits(self.read_ubit64(32) as u32)
    }

    pub fn read_bitcoord(&mut self) -> f32 {
        let mut value: f32 = 0.0;

        // Read the required integer and fraction flags
        let has_intval = self.read_bool();
        let has_fractval = self.read_bool();

        // If we got either parse them, otherwise it's a zero.
        if has_intval || has_fractval {
            // Read the sign bit
            let signbit = self.read_bool();

            // If there's an integer, read it in
            let mut intval = 0;
            if has_intval {
                // Adjust the integers from [0..MAX_COORD_VALUE-1] to [1..MAX_COORD_VALUE]
                intval = self.read_ubit64(COORD_INTEGER_BITS) + 1;
            }

            // If there's a fraction, read it in
            let mut fractval = 0;
            if has_fractval {
                fractval = self.read_ubit64(COORD_FRACTIONAL_BITS);
            }

            // Calculate the correct floating point value
            value = intval as f32 + (fractval as f32 * COORD_RESOLUTION);

            // Fixup the sign if negative.
            if signbit {
                value = -value;
            }
        }

        value
    }

    pub fn read_bitnormal(&mut self) -> f32 {
        // read the sign bit
        let signbit = self.read_bool();

        // read the fractional part
        let fractval = self.read_ubit64(NORMAL_FRACTIONAL_BITS);

        // calculate the correct floating point value
        let mut value = fractval as f32 * NORMAL_RESOLUTION;

        // fixup the sign if negative.
        if signbit {
            value = -value;
        }

        value
    }

    pub fn read_bitvec3coord(&mut self) -> [f32; 3] {
        let mut fa = [0f32; 3];

        let xflag = self.read_bool();
        let yflag = self.read_bool();
        let zflag = self.read_bool();

        if xflag {
            fa[0] = self.read_bitcoord();
        }
        if yflag {
            fa[1] = self.read_bitcoord();
        }
        if zflag {
            fa[2] = self.read_bitcoord();
        }

        fa
    }

    pub fn read_bitvec3normal(&mut self) -> [f32; 3] {
        let mut fa = [0f32; 3];

        let xflag = self.read_bool();
        let yflag = self.read_bool();

        if xflag {
            fa[0] = self.read_bitnormal();
        }
        if yflag {
            fa[1] = self.read_bitnormal();
        }

        // the first two imply the third (but not its sign)
        let znegative = self.read_bool();

        let fafafbfb = fa[0] * fa[0] + fa[1] * fa[1];
        if fafafbfb < 1.0 {
            fa[2] = (1.0 - fafafbfb).sqrt();
        }

        if znegative {
            fa[2] = -fa[2];
        }

        fa
    }

    pub fn read_bitangle(&mut self, num_bits: usize) -> f32 {
        let shift = bitbuf::get_bit_for_bit_num(num_bits) as f32;

        let u = self.read_ubit64(num_bits);
        let ret = (u as f32) * (360.0 / shift);

        ret
    }

    // Always reads to the end of the string (so you can read the next piece of data waiting).
    //
    // If line is true, it stops when it reaches a '\n' or a null-terminator.
    //
    // buf is always null-terminated (unless buf.len() is 0).
    //
    // Returns the number of characters left in out when the routine is complete (this will never
    // exceed buf.len()-1).
    pub fn read_string(&mut self, buf: &mut [u8], line: bool) -> usize {
        debug_assert!(!buf.is_empty());

        let mut too_small = false;
        let mut num_chars = 0;
        loop {
            let val = self.read_byte();
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

        // make sure it's null-terminated.
        debug_assert!(num_chars < buf.len());
        buf[num_chars] = 0;

        // did it fit?
        debug_assert!(!too_small);

        num_chars
    }

    pub fn read_ubitvarfp(&mut self) -> u32 {
        let ret = if self.read_bool() {
            self.read_ubit64(2)
        } else if self.read_bool() {
            self.read_ubit64(4)
        } else if self.read_bool() {
            self.read_ubit64(10)
        } else if self.read_bool() {
            self.read_ubit64(17)
        } else {
            self.read_ubit64(31)
        };
        ret as u32
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_string() {
        let buf = b"Life's but a walking shadow, a poor player.\0";
        let mut br = BitReader::new(buf);

        let mut out = vec![0u8; buf.len()];
        let num_chars = br.read_string(&mut out, false);
        assert_eq!(&out, &buf);
        assert_eq!(num_chars, buf.len() - 1);
    }
}
