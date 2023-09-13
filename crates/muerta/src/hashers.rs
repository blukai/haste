use std::hash::{BuildHasherDefault, Hasher};

// those hashes are designed to take a little more speed out; stolen from:
// https://github.com/chris-morgan/anymap/blob/2e9a5704/src/lib.rs#L599

#[derive(Default)]
pub(crate) struct I32Hasher(i32);

impl Hasher for I32Hasher {
    #[inline]
    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!("This I32Hasher can only handle i32s")
    }

    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.0 = i;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0 as u64
    }
}

pub(crate) type I32HashBuilder = BuildHasherDefault<I32Hasher>;

// A hasher designed to take a little more speed out.
// stolen from: https://github.com/chris-morgan/anymap/blob/2e9a5704/src/lib.rs#L599
#[derive(Default)]
pub(crate) struct U64Hasher(u64);

impl Hasher for U64Hasher {
    #[inline]
    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!("This U64Hasher can only handle u64s")
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

pub(crate) type U64HashBuiler = BuildHasherDefault<U64Hasher>;
