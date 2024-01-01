const HASH_BASIS: u64 = 14695981039346656037;
const HASH_PRIME: u64 = 1099511628211;

macro_rules! generate_hash_fn {
    ($type:ty, $name:ident) => {
        #[inline(always)]
        pub const fn $name(values: &[$type]) -> u64 {
            let mut acc = HASH_BASIS;
            // NOTE: while is used because for loops and ranges don't work in
            // const fns.
            let mut i = 0;
            while i < values.len() {
                acc = (acc ^ values[i] as u64).wrapping_mul(HASH_PRIME);
                i += 1;
            }
            acc
        }
    };
}

generate_hash_fn!(u8, hash_u8);
generate_hash_fn!(u32, hash_u32);

// TODO: try fxhash / firefox hash; it seems to be faster then fnv, see
// https://github.com/cbreeden/fxhash/tree/master?tab=readme-ov-file#benchmarks
