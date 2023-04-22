use crate::{bitreader::BitReader, error::Result};

// THIS IS ABSOLUTE GARBAGE
// TODO: cleanup, the functional way!

pub struct CNetworkedQuantizedFloat {
    bit_count: i32,
    low_value: f32,
    high_value: f32,
    encode_flags: i32,

    computed_encode_flags: i32,
    high_low_multiplier: f32,
    decode_multiplier: f32,
}

// TODO: get rid of encode_flags or computed_encode_flags - keeep only one of them.

impl CNetworkedQuantizedFloat {
    const QFE_ROUNDDOWN: i32 = 1 << 0;
    const QFE_ROUNDUP: i32 = 1 << 1;
    const QFE_ENCODE_ZERO_EXACTLY: i32 = 1 << 2;
    const QFE_ENCODE_INTEGERS_EXACTLY: i32 = 1 << 3;

    pub fn new(bit_count: i32, low_value: f32, high_value: f32, encode_flags: i32) -> Self {
        let mut elf = Self {
            bit_count,
            low_value,
            high_value,
            encode_flags,
            computed_encode_flags: Self::compute_encode_flags(encode_flags, low_value, high_value),
            high_low_multiplier: 0.0,
            decode_multiplier: 0.0,
        };
        elf.initialize();
        return elf;
    }

    fn initialize(&mut self) {
        let mut offset: f32;
        let mut quanta = 1 << self.bit_count;

        // if ((flags & (QFE_ROUNDDOWN | QFE_ROUNDUP)) == (QFE_ROUNDDOWN | QFE_ROUNDUP)) {
        //     log.warn("Field %s was flagged to both round up and down, these flags are mutually exclusive [%f->%f]\n", fieldName, minValue, maxValue);
        // }

        if (self.encode_flags & Self::QFE_ROUNDDOWN) != 0 {
            offset = (self.high_value - self.low_value) / quanta as f32;
            self.high_value -= offset;
        } else if (self.encode_flags & Self::QFE_ROUNDUP) != 0 {
            offset = (self.high_value - self.low_value) / quanta as f32;
            self.low_value += offset;
        }

        if (self.encode_flags & Self::QFE_ENCODE_INTEGERS_EXACTLY) != 0 {
            let delta = self.low_value as i32 - self.high_value as i32;
            let true_range = 1 << Self::calc_bits_needed_for(delta.max(1) as i64);

            let mut n_bits = self.bit_count;
            while (1 << n_bits) < true_range {
                n_bits += 1;
            }
            if n_bits > self.bit_count {
                // log.warn("Field %s was flagged QFE_ENCODE_INTEGERS_EXACTLY, but didn't specify enough bits, upping bitcount from %d to %d for range [%f->%f]", fieldName, bitCount, nBits, minValue, maxValue);
                self.bit_count = n_bits;
                quanta = 1 << self.bit_count;
            }

            let float_range = true_range as f32;
            offset = float_range / quanta as f32;
            self.high_value = self.low_value + float_range - offset;
        }

        self.high_low_multiplier =
            Self::assign_range_multiplier(self.bit_count, self.high_value - self.low_value);
        self.decode_multiplier = 1.0 / (quanta - 1) as f32;
        if self.high_low_multiplier == 0.0 {
            // TODO: Result
            panic!("Assert failed: highLowMultiplier is zero!");
        }

        if (self.computed_encode_flags & Self::QFE_ROUNDDOWN) != 0 {
            if self.quantize(self.low_value) == self.low_value {
                self.computed_encode_flags &= !Self::QFE_ROUNDDOWN;
            }
        }
        if (self.computed_encode_flags & Self::QFE_ROUNDUP) != 0 {
            if self.quantize(self.high_value) == self.high_value {
                self.computed_encode_flags &= !Self::QFE_ROUNDUP;
            }
        }
        if (self.computed_encode_flags & Self::QFE_ENCODE_ZERO_EXACTLY) != 0 {
            if self.quantize(0.0) == 0.0 {
                self.computed_encode_flags &= !Self::QFE_ENCODE_ZERO_EXACTLY;
            }
        }
    }

    fn compute_encode_flags(encode_flags: i32, low_value: f32, high_value: f32) -> i32 {
        let mut encode_flags = encode_flags;

        // If the min or max value is exactly zero and we are encoding min or max exactly, then don't need zero flag
        if (low_value == 0.0 && (encode_flags & Self::QFE_ROUNDDOWN) != 0)
            || (high_value == 0.0 && (encode_flags & Self::QFE_ROUNDUP) != 0)
        {
            encode_flags &= !Self::QFE_ENCODE_ZERO_EXACTLY;
        }

        // If specified encode zero but min or max actual value is zero, then convert that encode directive to be encode min or max exactly instead
        if low_value == 0.0 && (encode_flags & Self::QFE_ENCODE_ZERO_EXACTLY) != 0 {
            encode_flags |= Self::QFE_ROUNDDOWN;
            encode_flags &= !Self::QFE_ENCODE_ZERO_EXACTLY;
        }
        if high_value == 0.0 && (encode_flags & Self::QFE_ENCODE_ZERO_EXACTLY) != 0 {
            encode_flags |= Self::QFE_ROUNDUP;
            encode_flags &= !Self::QFE_ENCODE_ZERO_EXACTLY;
        }

        // If the range doesn't span across zero, then also don't need the zero flag
        if !(low_value < 0.0 && high_value > 0.0) {
            // if ((f & QFE_ENCODE_ZERO_EXACTLY) != 0) {
            //     log.warn("Field %s was flagged to encode zero exactly, but min/max range doesn't span zero [%f->%f]", fieldName, minValue, maxValue);
            // }
            encode_flags &= !Self::QFE_ENCODE_ZERO_EXACTLY;
        }

        if (encode_flags & Self::QFE_ENCODE_INTEGERS_EXACTLY) != 0 {
            // Wipes out all other flags
            encode_flags &=
                !(Self::QFE_ROUNDUP | Self::QFE_ROUNDDOWN | Self::QFE_ENCODE_ZERO_EXACTLY);
        }

        encode_flags
    }

    // also exists in csgo, in AssignRangeMultiplier in public/dt_send.cpp
    fn assign_range_multiplier(n_bits: i32, range: f32) -> f32 {
        let high_value: u32;
        if n_bits == 32 {
            high_value = 0xFFFFFFFE;
        } else {
            high_value = (1u32 << n_bits) - 1;
        }

        let mut high_low_mul: f32 = high_value as f32 / range as f32;
        if range.abs() <= 0.001 {
            high_low_mul = high_value as f32;
        }

        // If the precision is messing us up, then adjust it so it won't.
        if (high_low_mul * range > high_value as f32)
            || ((high_low_mul * range) > high_value as f32)
        {
            // Squeeze it down smaller and smaller until it's going to produce an integer
            // in the valid range when given the highest value.
            const MULTIPLIERS: [f32; 5] = [0.9999, 0.99, 0.9, 0.8, 0.7];
            let mut i = 0;
            while i < MULTIPLIERS.len() {
                high_low_mul = (high_value as f32 / range as f32) * MULTIPLIERS[i];
                if (high_low_mul * range > high_value as f32)
                    || ((high_low_mul * range) > high_value as f32)
                {
                    i += 1;
                } else {
                    break;
                }
            }

            if i == MULTIPLIERS.len() {
                // Doh! We seem to be unable to represent this range.
                // TODO: Result
                panic!("Unable to represent this range.");
            }
        }

        return high_low_mul;
    }

    fn quantize(&self, value: f32) -> f32 {
        if value < self.low_value {
            return self.low_value;
        } else if value > self.high_value {
            return self.high_value;
        }

        let i = ((value - self.low_value) * self.high_low_multiplier) as i32;
        self.low_value + (self.high_value - self.low_value) * (i as f32 * self.decode_multiplier)
    }

    pub fn decode(&self, br: &mut BitReader) -> Result<f32> {
        if self.computed_encode_flags & Self::QFE_ROUNDDOWN != 0 && br.read_bool()? {
            return Ok(self.low_value);
        }
        if self.computed_encode_flags & Self::QFE_ROUNDUP != 0 && br.read_bool()? {
            return Ok(self.high_value);
        }
        if self.computed_encode_flags & Self::QFE_ENCODE_ZERO_EXACTLY != 0 && br.read_bool()? {
            return Ok(0.0);
        }
        let v = br.read(self.bit_count as u32)?;
        return Ok(
            self.low_value + (self.high_value - self.low_value) * v as f32 * self.decode_multiplier
        );
    }

    // TODO: move this to utils or something (together with read_varu32)
    fn calc_bits_needed_for(x: i64) -> i32 {
        if x == 0 {
            return 0;
        }
        let mut n = 32;
        let mut y = x as u64;
        if y <= 0x0000FFFF {
            n -= 16;
            y <<= 16;
        }
        if y <= 0x00FFFFFF {
            n -= 8;
            y <<= 8;
        }
        if y <= 0x0FFFFFFF {
            n -= 4;
            y <<= 4;
        }
        if y <= 0x3FFFFFFF {
            n -= 2;
            y <<= 2;
        }
        if y <= 0x7FFFFFFF {
            n -= 1;
        }
        n
    }
}
