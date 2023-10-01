use crate::varint;
use std::intrinsics::unlikely;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // mod
    #[error("was about to overrun a buffer")]
    Overflow,
    #[error("operation could not be completed because there are not enough bits left")]
    Underflow,
    #[error("malformed varint")]
    MalformedVarint,
    #[error("string buffer is too small")]
    StringBufTooSmol,
}

pub type Result<T> = std::result::Result<T, Error>;

// public/coordsize.h
const COORD_INTEGER_BITS: usize = 14;
const COORD_FRACTIONAL_BITS: usize = 5;
const COORD_DENOMINATOR: f32 = (1 << COORD_FRACTIONAL_BITS) as f32;
const COORD_RESOLUTION: f32 = 1.0 / COORD_DENOMINATOR;

// (1 << i) - 1
// also see CBitWriteMasksInit in tier1/bitbuf.cpp
static EXTRA_MASKS: [u32; 33] = [
    0x0, 0x1, 0x3, 0x7, 0xf, 0x1f, 0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff, 0x1fff, 0x3fff,
    0x7fff, 0xffff, 0x1ffff, 0x3ffff, 0x7ffff, 0xfffff, 0x1fffff, 0x3fffff, 0x7fffff, 0xffffff,
    0x1ffffff, 0x3ffffff, 0x7ffffff, 0xfffffff, 0x1fffffff, 0x3fffffff, 0x7fffffff, 0xffffffff,
];

// public/bitvec.h
pub fn get_bit_for_bitnum(bitnum: i32) -> i32 {
    const BITS_PER_INT: i32 = 32;
    static BITS_FOR_BITNUM: [i32; BITS_PER_INT as usize] = [
        1 << 0,
        1 << 1,
        1 << 2,
        1 << 3,
        1 << 4,
        1 << 5,
        1 << 6,
        1 << 7,
        1 << 8,
        1 << 9,
        1 << 10,
        1 << 11,
        1 << 12,
        1 << 13,
        1 << 14,
        1 << 15,
        1 << 16,
        1 << 17,
        1 << 18,
        1 << 19,
        1 << 20,
        1 << 21,
        1 << 22,
        1 << 23,
        1 << 24,
        1 << 25,
        1 << 26,
        1 << 27,
        1 << 28,
        1 << 29,
        1 << 30,
        1 << 31,
    ];
    BITS_FOR_BITNUM[(bitnum & (BITS_PER_INT - 1)) as usize]
}

// BitRead is a port of valve's CBitRead(or/and old_bf_read) from valve's tier1
// lib.
pub struct BitReader<'d> {
    // The current buffer.
    data: &'d [u32],
    data_bits: usize,
    // Where we are in the buffer.
    curr_bit: usize,
}

impl<'d> BitReader<'d> {
    pub fn new(data: &'d [u8]) -> Self {
        Self {
            data: unsafe { std::mem::transmute(data) },
            // << 3 is same as * 8, but faster
            data_bits: data.len() << 3,
            curr_bit: 0,
        }
    }

    // FORCEINLINE  int                     Tell( void ) const
    // FORCEINLINE  size_t                  TotalBytesAvailable( void ) const

    // FORCEINLINE  int                     GetNumBitsLeft() const
    #[inline(always)]
    pub fn get_num_bits_left(&self) -> usize {
        self.data_bits - self.curr_bit
    }

    // FORCEINLINE  int                     GetNumBytesLeft() const
    #[inline(always)]
    pub fn get_num_bytes_left(&self) -> usize {
        self.get_num_bits_left() >> 3
    }

    //              bool                    Seek( int nPosition );
    pub fn seek(&mut self, bit: usize) -> Result<usize> {
        if unlikely(bit > self.data_bits) {
            Err(Error::Overflow)
        } else {
            self.curr_bit = bit;
            Ok(self.curr_bit)
        }
    }

    // FORCEINLINE  bool                    SeekRelative( int nOffset )
    //
    // seek_relative seeks to an offset from the current position
    #[inline(always)]
    pub fn seek_relative(&mut self, bit_delta: isize) -> Result<usize> {
        let bit = self.curr_bit as isize + bit_delta;
        if unlikely(bit < 0) {
            Err(Error::Underflow)
        } else {
            self.seek(bit as usize)
        }
    }

    // FORCEINLINE  unsigned char const *   GetBasePointer()
    //              void                    StartReading( const void *pData, int nBytes, int iStartBit = 0, int nBits = -1 );
    // FORCEINLINE  int                     GetNumBitsRead( void ) const;
    // FORCEINLINE  int                     GetNumBytesRead( void ) const;
    // FORCEINLINE  void                    GrabNextDWord( bool bOverFlowImmediately = false );
    // FORCEINLINE  void                    FetchNext( void );

    // FORCEINLINE  unsigned int            ReadUBitLong( int numbits );
    //
    // read_ubitlong reads the specified number of bits into a `u32`. The
    // function can read up to a maximum of 32 bits at a time. If the `num_bits`
    // exceeds the number of remaining bits, the function returns an
    // `Error::Underflow` error.
    #[inline(always)]
    pub fn read_ubitlong(&mut self, num_bits: usize) -> Result<u32> {
        debug_assert!(num_bits < 33, "trying to read more than 32 bits");

        if unlikely(self.get_num_bits_left() < num_bits) {
            return Err(Error::Underflow);
        }

        // Read the current dword.
        let dw1_offset = self.curr_bit >> 5;
        let mut dw1 = self.data[dw1_offset];

        dw1 >>= self.curr_bit & 31; // Get the bits we're interested in.

        self.curr_bit += num_bits;
        let mut ret = dw1;

        // Does it span this dword?
        if (self.curr_bit - 1) >> 5 == dw1_offset {
            if num_bits != 32 {
                ret &= EXTRA_MASKS[num_bits];
            }
        } else {
            let extra_bits = self.curr_bit & 31;
            let mut dw2 = self.data[dw1_offset + 1];

            dw2 &= EXTRA_MASKS[extra_bits];

            // No need to mask since we hit the end of the dword.
            // Shift the second dword's part into the high bits.
            ret |= dw2 << (num_bits - extra_bits);
        }

        Ok(ret)
    }

    // FORCEINLINE  int                     ReadSBitLong( int numbits );

    // FORCEINLINE  unsigned int            ReadUBitVar( void );
    //
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

    // FORCEINLINE  unsigned int            PeekUBitLong( int numbits );

    // FORCEINLINE  float                   ReadBitFloat( void );
    #[inline(always)]
    pub fn read_bitfloat(&mut self) -> Result<f32> {
        self.read_ubitlong(32).map(f32::from_bits)
    }

    //              float                   ReadBitCoord();
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

    //              float                   ReadBitCoordMP( EBitCoordType coordType );
    //              float                   ReadBitCellCoord( int bits, EBitCoordType coordType );
    //              float                   ReadBitNormal();
    //              void                    ReadBitVec3Coord( Vector& fa );
    //              void                    ReadBitVec3Normal( Vector& fa );
    //              void                    ReadBitAngles( QAngle& fa );

    //              bool                    ReadBytes(void *pOut, int nBytes);
    pub fn read_bytes(&mut self, out: &mut [u8]) -> Result<()> {
        self.read_bits(out, out.len() << 3)
    }

    //              float                   ReadBitAngle( int numbits );
    pub fn read_bitangle(&mut self, num_bits: usize) -> Result<f32> {
        let shift = get_bit_for_bitnum(num_bits as i32) as f32;

        let u = self.read_ubitlong(num_bits)?;
        let ret = (u as f32) * (360.0 / shift);

        Ok(ret)
    }

    // FORCEINLINE  int	                    ReadOneBit( void );
    #[inline(always)]
    pub fn read_bool(&mut self) -> Result<bool> {
        if unlikely(self.get_num_bits_left() < 1) {
            return Err(Error::Underflow);
        }

        let one_bit = self.data[self.curr_bit >> 5] >> (self.curr_bit & 31) & 1;
        self.curr_bit += 1;

        Ok(one_bit == 1)
    }

    // FORCEINLINE  int                     ReadLong( void );
    // FORCEINLINE  int                     ReadChar( void );

    // FORCEINLINE  int                     ReadByte( void );
    #[inline(always)]
    pub fn read_byte(&mut self) -> Result<u8> {
        self.read_ubitlong(8).map(|result| result as u8)
    }

    // FORCEINLINE  int                     ReadShort( void );
    // FORCEINLINE  int                     ReadWord( void );
    // FORCEINLINE  float                   ReadFloat( void );

    //              void                    ReadBits(void *pOut, int nBits);
    pub fn read_bits(&mut self, out: &mut [u8], num_bits: usize) -> Result<()> {
        let mut p_out = out.as_mut_ptr();
        let mut num_bits_left = num_bits;

        // align output to dword boundary
        while (p_out as usize & 3) != 0 && num_bits_left >= 8 {
            unsafe {
                *p_out = self.read_ubitlong(8)? as u8;
                p_out = p_out.add(1);
            }
            num_bits_left -= 8;
        }

        // read dwords
        while num_bits_left >= 32 {
            unsafe {
                *(p_out as *mut u32) = self.read_ubitlong(32)?;
                p_out = p_out.add(4);
            };
            num_bits_left -= 32;
        }

        // read remaining bytes
        while num_bits_left >= 8 {
            unsafe {
                *p_out = self.read_ubitlong(8)? as u8;
                p_out = p_out.add(1);
            }
            num_bits_left -= 8;
        }

        // read remaining bits
        if num_bits_left > 0 {
            unsafe {
                *p_out = self.read_ubitlong(num_bits_left)? as u8;
            }
        }

        Ok(())
    }

    // bool                                 ReadString( char *pStr, int bufLen, bool bLine=false, int *pOutNumChars=NULL );
    //
    // Returns Error::StringBufferTooSmall error if buf isn't large enough to hold the
    // string.
    //
    // Always reads to the end of the string (so you can read the
    // next piece of data waiting).
    //
    // If line is true, it stops when it reaches a '\n' or a null-terminator.
    //
    // buf is always null-terminated (unless buf.len() is 0).
    //
    // Returns the number of characters left in out when the routine is
    // complete (this will never exceed buf.len()-1).
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

        if unlikely(too_small) {
            Err(Error::StringBufTooSmol)
        } else {
            Ok(num_chars)
        }
    }

    //              bool                    ReadWString( OUT_Z_CAP(maxLenInChars) wchar_t *pStr, int maxLenInChars, bool bLine=false, int *pOutNumChars=NULL );
    //              char*                   ReadAndAllocateString( bool *pOverflow = 0 );
    //              int64                   ReadLongLong( void );

    // NOTE: read_uvarint is simillar to function with the same exact name
    // withint varint.rs, and yes, it is possible to have only one, BUT
    // implementing a Read trait for BitReader degrades performance quite
    // significantly.
    #[inline(always)]
    fn read_uvarint<const MAX_VARINT_BYTES: usize>(&mut self) -> Result<u64> {
        let mut result = 0;
        for count in 0..=MAX_VARINT_BYTES {
            let byte = self.read_byte()?;
            result |= ((byte & varint::PAYLOAD_BITS) as u64) << (count * 7);
            if (byte & varint::CONTINUE_BIT) == 0 {
                return Ok(result);
            }
        }
        Err(Error::MalformedVarint)
    }

    //              uint32                  ReadVarInt32();
    pub fn read_uvarint32(&mut self) -> Result<u32> {
        self.read_uvarint::<{ varint::MAX_VARINT32_BYTES }>()
            .map(|result| result as u32)
    }

    //              uint64                  ReadVarInt64();
    pub fn read_uvarint64(&mut self) -> Result<u64> {
        self.read_uvarint::<{ varint::MAX_VARINT64_BYTES }>()
    }

    //              int32                   ReadSignedVarInt32() { return bitbuf::ZigZagDecode32( ReadVarInt32() ); }
    pub fn read_varint32(&mut self) -> Result<i32> {
        self.read_uvarint32().map(varint::zigzag_decode32)
    }

    //              int64                   ReadSignedVarInt64() { return bitbuf::ZigZagDecode64( ReadVarInt64() ); }
    pub fn read_varint64(&mut self) -> Result<i64> {
        self.read_uvarint64().map(varint::zigzag_decode64)
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

    // NOTE: data for some tests is stolen from manta xd.

    #[test]
    fn test_read_ubitlong() -> super::Result<()> {
        let buf = [0xff; 8];
        let mut br = super::BitReader::new(&buf);

        assert_eq!(0x7f, br.read_ubitlong(7)?);
        assert_eq!(0xff, br.read_ubitlong(8)?);
        assert_eq!(0xffff, br.read_ubitlong(16)?);
        assert_eq!(0xffffffff, br.read_ubitlong(32)?);
        assert_eq!(0x01, br.read_ubitlong(1)?);

        Ok(())
    }

    #[test]
    fn test_read_varints() -> super::Result<()> {
        let buf = [0x01, 0xff, 0xff, 0xff, 0xff, 0x0f, 0x8c, 0x01];
        let mut br = super::BitReader::new(&buf);

        assert_eq!(1, br.read_uvarint32()?);
        assert_eq!(4294967295, br.read_uvarint32()?);
        assert_eq!(140, br.read_uvarint32()?);

        br.seek(0)?;
        assert_eq!(1, br.read_uvarint32()?);
        assert_eq!(-2147483648, br.read_varint32()?);

        br.seek(0)?;
        assert_eq!(1, br.read_uvarint64()?);
        assert_eq!(4294967295, br.read_uvarint64()?);
        assert_eq!(140, br.read_uvarint64()?);

        Ok(())
    }

    #[test]
    fn test_read_bits() -> super::Result<()> {
        let buf = [0xff; 4];
        let mut br = super::BitReader::new(&buf);

        let mut out = [0u8; 4];
        br.read_bits(&mut out, 32)?;
        assert_eq!(&out, &buf);

        Ok(())
    }

    #[test]
    fn test_read_bytes() -> super::Result<()> {
        let buf = [42, 0, 0, 0];
        let mut br = super::BitReader::new(&buf);

        let mut out = [0u8; 1];
        br.read_bytes(&mut out)?;
        assert_eq!(&out, &buf[..1]);

        Ok(())
    }

    #[test]
    fn test_read_bool() -> super::Result<()> {
        let buf = [1];
        let mut br = super::BitReader::new(&buf);

        assert!(br.read_bool()?);
        assert!(!(br.read_bool()?));

        Ok(())
    }

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
