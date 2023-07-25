const HASH_BASIS: u64 = 14695981039346656037;
const HASH_PRIME: u64 = 1099511628211;

#[inline(always)]
pub const fn hash(bytes: &[u8]) -> u64 {
    let mut acc = HASH_BASIS;
    let mut i = 0;
    while i < bytes.len() {
        acc = (acc ^ bytes[i] as u64).wrapping_mul(HASH_PRIME);
        i += 1;
    }
    acc
}
