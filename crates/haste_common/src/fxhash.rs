// NOTE: hash functions are implemented manually instead of being pulled from
// crates because they need to be const(/comptime); none of the crates that i
// went throuh had reasonable const(/comptime) implementations.

// according to tests in
// https://nnethercote.github.io/perf-book/hashing.html fx hash is 6% faster
// then fnv; ahash is 1-4% slower then fx.

// following text is copypaste from
// https://searchfox.org/mozilla-central/rev/633345116df55e2d37be9be6555aa739656c5a7d/mfbt/HashFunctions.h#109-153

// This is the meat of all our hash routines.  This hash function is not
// particularly sophisticated, but it seems to work well for our mostly
// plain-text inputs.  Implementation notes follow.
//
// Our use of the golden ratio here is arbitrary; we could pick almost any
// number which:
//
//  * is odd (because otherwise, all our hash values will be even)
//
//  * has a reasonably-even mix of 1's and 0's (consider the extreme case
//    where we multiply by 0x3 or 0xeffffff -- this will not produce good
//    mixing across all bits of the hash).
//
// The rotation length of 5 is also arbitrary, although an odd number is again
// preferable so our hash explores the whole universe of possible rotations.
//
// Finally, we multiply by the golden ratio *after* xor'ing, not before.
// Otherwise, if |aHash| is 0 (as it often is for the beginning of a
// message), the expression
//
//   mozilla::WrappingMultiply(kGoldenRatioU32, RotateLeft5(aHash))
//   |xor|
//   aValue
//
// evaluates to |aValue|.
//
// (Number-theoretic aside: Because any odd number |m| is relatively prime to
// our modulus (2**32), the list
//
//    [x * m (mod 2**32) for 0 <= x < 2**32]
//
// has no duplicate elements.  This means that multiplying by |m| does not
// cause us to skip any possible hash values.
//
// It's also nice if |m| has large-ish order mod 2**32 -- that is, if the
// smallest k such that m**k == 1 (mod 2**32) is large -- so we can safely
// multiply our hash value by |m| a few times without negating the
// multiplicative effect.  Our golden ratio constant has order 2**29, which is
// more than enough for our purposes.)

// a little bit more info on fx hash is available on
// https://nnethercote.github.io/2021/12/08/a-brutally-effective-hash-function-in-rust.html

// NOTE: u64 golden ration is stolen from
// https://github.com/rust-lang/rustc-hash/blob/786ccda70fce97a3177d6088f21a22ac7f0f2f85/src/lib.rs#L67
const GOLDEN_RATIO: u64 = 0x517cc1b727220a95;
const ROTATION_LENGTH: u32 = 5;

// NOTE: FxHasher mimics std::hash::Hasher, but it does not directly implement
// the std::hash::Hasher trait because traits do not support const fns.
#[repr(transparent)]
pub struct Hasher {
    state: u64,
}

impl Hasher {
    #[inline(always)]
    pub const fn default() -> Self {
        Self { state: 0 }
    }

    #[inline(always)]
    pub const fn with_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    #[inline(always)]
    pub const fn finish(&self) -> u64 {
        self.state
    }

    // fn write(&mut self, bytes: &[u8]);

    // ----

    // fn write_u8(&mut self, i: u8) { ... }
    // fn write_u16(&mut self, i: u16) { ... }
    // fn write_u32(&mut self, i: u32) { ... }

    #[inline(always)]
    pub const fn write_u64(&mut self, i: u64) {
        self.state = (self.state.rotate_left(ROTATION_LENGTH) ^ i).wrapping_mul(GOLDEN_RATIO);
    }

    // fn write_u128(&mut self, i: u128) { ... }
    // fn write_usize(&mut self, i: usize) { ... }
    // fn write_i8(&mut self, i: i8) { ... }
    // fn write_i16(&mut self, i: i16) { ... }
    // fn write_i32(&mut self, i: i32) { ... }
    // fn write_i64(&mut self, i: i64) { ... }
    // fn write_i128(&mut self, i: i128) { ... }
    // fn write_isize(&mut self, i: isize) { ... }
    // fn write_length_prefix(&mut self, len: usize) { ... }
    // fn write_str(&mut self, s: &str) { ... }
}

// ----

#[inline(always)]
pub const fn hash_u8(values: &[u8]) -> u64 {
    let mut hasher = Hasher::default();
    // NOTE: while is used because for loops and ranges don't work in const fns.
    let mut i = 0;
    while i < values.len() {
        hasher.write_u64(values[i] as u64);
        i += 1;
    }
    hasher.finish()
}
