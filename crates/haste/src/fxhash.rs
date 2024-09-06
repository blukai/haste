//! custom implementation of hash functions? why? simply because i need them to be
//! const(/comptime); none of the crates that i went throuh had reasonable const(/comptime)
//! implementations.
//!
//! according to tests in <https://nnethercote.github.io/perf-book/hashing.html> fx hash is 6%
//! faster then fnv; ahash is 1-4% slower then fx.
//!
//! according to my tests (in scope of dota 2 replay parser (github.com/blukai/haste) fx hash is
//! indeed faster then fnv1a.
//!
//! the following text is copypaste from
//! <https://searchfox.org/mozilla-central/rev/633345116df55e2d37be9be6555aa739656c5a7d/mfbt/HashFunctions.h#109-153>
//!
//! > This is the meat of all our hash routines.  This hash function is not
//! > particularly sophisticated, but it seems to work well for our mostly
//! > plain-text inputs.  Implementation notes follow.
//! >
//! > Our use of the golden ratio here is arbitrary; we could pick almost any
//! > number which:
//! >
//! >  * is odd (because otherwise, all our hash values will be even)
//! >
//! >  * has a reasonably-even mix of 1's and 0's (consider the extreme case
//! >    where we multiply by 0x3 or 0xeffffff -- this will not produce good
//! >    mixing across all bits of the hash).
//! >
//! > The rotation length of 5 is also arbitrary, although an odd number is again
//! > preferable so our hash explores the whole universe of possible rotations.
//! >
//! > Finally, we multiply by the golden ratio *after* xor'ing, not before.
//! > Otherwise, if |aHash| is 0 (as it often is for the beginning of a
//! > message), the expression
//! >
//! >   mozilla::WrappingMultiply(kGoldenRatioU32, RotateLeft5(aHash))
//! >   |xor|
//! >   aValue
//! >
//! > evaluates to |aValue|.
//! >
//! > (Number-theoretic aside: Because any odd number |m| is relatively prime to
//! > our modulus (2**32), the list
//! >
//! >    [x * m (mod 2**32) for 0 <= x < 2**32]
//! >
//! > has no duplicate elements.  This means that multiplying by |m| does not
//! > cause us to skip any possible hash values.
//! >
//! > It's also nice if |m| has large-ish order mod 2**32 -- that is, if the
//! > smallest k such that m**k == 1 (mod 2**32) is large -- so we can safely
//! > multiply our hash value by |m| a few times without negating the
//! > multiplicative effect.  Our golden ratio constant has order 2**29, which is
//! > more than enough for our purposes.)
//!
//! a little bit more info on fx hash is available on
//! <https://nnethercote.github.io/2021/12/08/a-brutally-effective-hash-function-in-rust.html>

// NOTE: u64 golden ration is stolen from
// https://github.com/rust-lang/rustc-hash/blob/786ccda70fce97a3177d6088f21a22ac7f0f2f85/src/lib.rs#L67
const GOLDEN_RATIO: u64 = 0x517cc1b727220a95;
const ROTATION_LENGTH: u32 = 5;

#[inline(always)]
pub const fn add_u64_to_hash(hash: u64, value: u64) -> u64 {
    (hash.rotate_left(ROTATION_LENGTH) ^ value).wrapping_mul(GOLDEN_RATIO)
}

// TODO(blukai): figure out comptime alignment when issues when transmuting &[u8] into &[u64]. note
// that relying on core::ptr::copy won't help as its const impl is unstable / requires nightly.

/// hash some number of bytes.
///
/// original implementation:
/// <https://searchfox.org/mozilla-central/rev/e0a62f1391f7d58fab20418adc9310b23708a792/mfbt/HashFunctions.cpp#16>
///
/// note that this functions is not as fast as it could be due to not yet figured out rust comptile
/// issues with mem alignment. ultimately it would transmute u8s into u64s and hashe as many u64s
/// as possible, and then remaining u8s if any.
#[inline]
pub const fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0;

    let mut i = 0;
    while i < bytes.len() {
        hash = add_u64_to_hash(hash, bytes[i] as u64);
        i += 1;
    }

    hash
}
