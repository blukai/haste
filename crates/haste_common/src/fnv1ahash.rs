// NOTE: hash functions are implemented manually instead of being pulled from
// crates because they need to be const(/comptime); none of the crates that i
// went throuh had reasonable const(/comptime) implementations.

// NOTE: while looks are used because for loops and ranges don't work in
// const(/comptime) functions as of this moment.

// TODO: run more tests on fx hash, test collisions; maybe remove fnv1a
//
// implementation details about fnv1a can be found on wikipedia -
// https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function#FNV-1a_hash

const BASIS: u64 = 14695981039346656037;
const PRIME: u64 = 1099511628211;

macro_rules! impl_fn {
    ($type:ty, $name:ident) => {
        #[inline(always)]
        pub const fn $name(values: &[$type]) -> u64 {
            let mut hash = BASIS;
            let mut i = 0;
            while i < values.len() {
                hash = (hash ^ values[i] as u64).wrapping_mul(PRIME);
                i += 1;
            }
            hash
        }
    };
}

impl_fn!(u8, hash_u8);
impl_fn!(u32, hash_u32);
