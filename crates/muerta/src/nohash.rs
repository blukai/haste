use std::{
    hash::{BuildHasherDefault, Hasher},
    marker::PhantomData,
};

// those hashes are designed to take a little more speed out; stolen from:
// https://github.com/chris-morgan/anymap/blob/2e9a5704/src/lib.rs#L599
// and https://docs.rs/nohash/latest/nohash/struct.NoHashHasher.html

#[derive(Default)]
pub struct NoHashHasher<T> {
    value: u64,
    _t: PhantomData<T>,
}

impl<T> Hasher for NoHashHasher<T> {
    #[inline]
    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!("invalid use of NoHashHasher")
    }

    #[inline]
    fn write_i32(&mut self, v: i32) {
        self.value = v as u64;
    }

    #[inline]
    fn write_u64(&mut self, v: u64) {
        self.value = v;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.value
    }
}

pub type NoHashHasherBuilder<T> = BuildHasherDefault<NoHashHasher<T>>;
