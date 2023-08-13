use crate::bitbuf::{self, BitReader};

// NOTE: this is composite of stuff from butterfly, clarity, manta and leaked
// csgo.

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // crate
    #[error(transparent)]
    BitBuf(#[from] bitbuf::Error),
    // mod
    #[error("encode flags are both round up and down, these flags are mutually exclusive")]
    InvalidEncodeFlags,
    #[error("invalid range")]
    InvalidRange,
}

pub type Result<T> = std::result::Result<T, Error>;

const QFE_ROUNDDOWN: i32 = 1 << 0;
const QFE_ROUNDUP: i32 = 1 << 1;
const QFE_ENCODE_ZERO_EXACTLY: i32 = 1 << 2;
const QFE_ENCODE_INTEGERS_EXACTLY: i32 = 1 << 3;

// stolen from quantizedfloat.go in manta
fn compute_encode_flags(encode_flags: i32, low_value: f32, high_value: f32) -> Result<i32> {
    let mut efs = encode_flags;

    if efs == 0 {
        return Ok(efs);
    }

    // Discard zero flag when encoding min / max set to 0
    if (low_value == 0.0 && (efs & QFE_ROUNDDOWN) != 0)
        || (high_value == 0.0 && (efs & QFE_ROUNDUP) != 0)
    {
        efs &= !QFE_ENCODE_ZERO_EXACTLY;
    }

    // If min / max is zero when encoding zero, switch to round up / round down
    // instead
    if low_value == 0.0 && (efs & QFE_ENCODE_ZERO_EXACTLY) != 0 {
        efs |= QFE_ROUNDDOWN;
        efs &= !QFE_ENCODE_ZERO_EXACTLY;
    }
    if high_value == 0.0 && (efs & QFE_ENCODE_ZERO_EXACTLY) != 0 {
        efs |= QFE_ROUNDUP;
        efs &= !QFE_ENCODE_ZERO_EXACTLY;
    }

    // If the range doesn't span across zero, then also don't need the zero flag
    if !(low_value < 0.0 && high_value > 0.0) {
        efs &= !QFE_ENCODE_ZERO_EXACTLY;
    }

    // If we are left with encode zero, only leave integer flag
    if (efs & QFE_ENCODE_INTEGERS_EXACTLY) != 0 {
        efs &= !(QFE_ROUNDUP | QFE_ROUNDDOWN | QFE_ENCODE_ZERO_EXACTLY);
    }

    // Verify that we don;t have roundup / rounddown set
    if efs & (QFE_ROUNDDOWN | QFE_ROUNDUP) == (QFE_ROUNDDOWN | QFE_ROUNDUP) {
        return Err(Error::InvalidEncodeFlags);
    }

    Ok(efs)
}

const EQUAL_EPSILON: f32 = 0.001;

// public/mathlib/mathlib.h
fn close_enough(a: f32, b: f32, epsilon: f32) -> bool {
    (a - b).abs() <= epsilon
}

// public/dt_send.cpp
fn assign_range_multiplier(bit_count: i32, range: f64) -> Result<f32> {
    let high_value = if bit_count == 32 {
        0xFFFFFFFE
    } else {
        (1 << bit_count) - 1
    };

    // In C++, when you perform an operation between two different numeric
    // types, the result will be promoted to the type that can represent both
    // operands with the least loss of precision. This process is called "type
    // promotion" or "type coercion."

    let mut high_low_mul = if close_enough(range as f32, 0.0, EQUAL_EPSILON) {
        high_value as f32
    } else {
        (high_value as f64 / range) as f32
    };

    // If the precision is messing us up, then adjust it so it won't.
    if (high_low_mul as f64 * range) as u32 > high_value
        || (high_low_mul as f64 * range) > high_value as f64
    {
        // Squeeze it down smaller and smaller until it's going to produce an
        // integer in the valid range when given the highest value.
        const MULTIPLIERS: [f32; 5] = [0.9999, 0.99, 0.9, 0.8, 0.7];
        let mut i = 0;
        while i < MULTIPLIERS.len() {
            // fHighLowMul = (float)( iHighValue / range ) iHighValue is
            // unsigned long and range is a double -> the intermediate result
            // during the division will be a double due to type promotion in cpp.
            high_low_mul = (high_value as f64 / range) as f32 * MULTIPLIERS[i];

            // (unsigned long)(fHighLowMul * range) > iHighValue ||
            //   (fHighLowMul * range) > (double)iHighValue
            if ((high_low_mul as f64 * range) as u32 > high_value)
                || ((high_low_mul as f64 * range) > high_value as f64)
            {
                i += 1;
            } else {
                break;
            }
        }

        if i == MULTIPLIERS.len() {
            // Doh! We seem to be unable to represent this range.
            return Err(Error::InvalidRange);
        }
    }

    Ok(high_low_mul)
}

// public/dt_common.h
fn num_bits_for_count(n_max_elements: i32) -> i32 {
    let mut n_bits = 0;
    let mut n_max_elements = n_max_elements;

    while n_max_elements > 0 {
        n_bits += 1;
        n_max_elements >>= 1;
    }

    n_bits
}

pub struct QuantizedFloat {
    bit_count: i32,
    encode_flags: i32,
    low_value: f32,
    high_value: f32,

    high_low_mul: f32,
    decode_mul: f32,
}

impl QuantizedFloat {
    pub fn new(bit_count: i32, encode_flags: i32, low_value: f32, high_value: f32) -> Result<Self> {
        let mut qf = Self {
            bit_count,
            encode_flags,
            low_value,
            high_value,
            high_low_mul: 0.0,
            decode_mul: 0.0,
        };

        qf.encode_flags = compute_encode_flags(qf.encode_flags, qf.low_value, qf.high_value)?;
        let mut steps = 1 << qf.bit_count;

        let range = qf.high_value - qf.low_value;
        let offset = range / steps as f32;
        if qf.encode_flags & QFE_ROUNDDOWN != 0 {
            qf.high_value -= offset;
        } else if qf.encode_flags & QFE_ROUNDUP != 0 {
            qf.low_value += offset;
        }

        if qf.encode_flags & QFE_ENCODE_INTEGERS_EXACTLY != 0 {
            let delta = (qf.low_value as i32 - qf.high_value as i32).max(1);
            let range = 1 << num_bits_for_count(delta);

            let mut bc = qf.bit_count;
            while (1 << bc) < range {
                bc += 1;
            }
            if bc > qf.bit_count {
                qf.bit_count = bc;
                steps = 1 << bc;
            }

            let offset = range as f32 / steps as f32;
            qf.high_value = qf.low_value + range as f32 - offset;
        }

        let range = qf.high_value - qf.low_value;
        qf.high_low_mul = assign_range_multiplier(qf.bit_count, range as f64)?;
        qf.decode_mul = 1.0 / (steps - 1) as f32;

        // Remove unessecary flags
        if (qf.encode_flags & QFE_ROUNDDOWN) != 0 && qf.quantize(qf.low_value) == qf.low_value {
            qf.encode_flags &= !QFE_ROUNDDOWN;
        }
        if (qf.encode_flags & QFE_ROUNDUP) != 0 && qf.quantize(qf.high_value) == qf.high_value {
            qf.encode_flags &= !QFE_ROUNDUP;
        }
        if (qf.encode_flags & QFE_ENCODE_ZERO_EXACTLY) != 0 && qf.quantize(0.0) == 0.0 {
            qf.encode_flags &= !QFE_ENCODE_ZERO_EXACTLY;
        }

        Ok(qf)
    }

    fn quantize(&self, value: f32) -> f32 {
        if value < self.low_value {
            return self.low_value;
        } else if value > self.high_value {
            return self.high_value;
        }

        let range = self.high_value - self.low_value;
        let i = ((value - self.low_value) * self.high_low_mul) as i32;
        self.low_value + range * (i as f32 * self.decode_mul)
    }

    pub fn decode(&self, br: &mut BitReader) -> Result<f32> {
        if (self.encode_flags & QFE_ROUNDDOWN) != 0 && br.read_bool()? {
            return Ok(self.low_value);
        }

        if (self.encode_flags & QFE_ROUNDUP) != 0 && br.read_bool()? {
            return Ok(self.high_value);
        }

        if (self.encode_flags & QFE_ENCODE_ZERO_EXACTLY) != 0 && br.read_bool()? {
            return Ok(0.0);
        }

        let range = self.high_value - self.low_value;
        let value = br.read_ubitlong(self.bit_count as usize)?;
        Ok(self.low_value + range * (value as f32 * self.decode_mul))
    }
}
