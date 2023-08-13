use std::alloc::Allocator;

// WackyString is a Display'able and Debug'able wrapper for strings allocated in
// Vec<u8, A>. The problem is that rust's String does not support allocator_api.
#[derive(Clone)]
pub struct AllocString<A: Allocator> {
    vec: Vec<u8, A>,
}

impl<A: Allocator> AllocString<A> {
    #[inline]
    #[must_use]
    pub fn new_in(alloc: A) -> Self {
        Self {
            vec: Vec::new_in(alloc),
        }
    }

    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.vec
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        // if underlying vec was constructed from String or &str - this is safe,
        // and it's the whole thing is immutable.
        unsafe { std::str::from_utf8_unchecked(&self.vec) }
    }
}

// ----

pub trait AllocStringFromIn<T, A>: Sized {
    /// Converts to this type from the input type.
    #[must_use]
    fn from_in(value: T, alloc: A) -> Self;
}

impl<A: Allocator> AllocStringFromIn<&String, A> for AllocString<A> {
    #[inline]
    fn from_in(value: &String, alloc: A) -> Self {
        Self {
            vec: value.as_bytes().to_vec_in(alloc),
        }
    }
}

impl<A: Allocator> AllocStringFromIn<&str, A> for AllocString<A> {
    #[inline]
    fn from_in(value: &str, alloc: A) -> Self {
        Self {
            vec: value.as_bytes().to_vec_in(alloc),
        }
    }
}

impl<A: Allocator> AllocStringFromIn<&[u8], A> for AllocString<A> {
    #[inline]
    fn from_in(value: &[u8], alloc: A) -> Self {
        Self {
            vec: value.to_vec_in(alloc),
        }
    }
}

impl<A: Allocator> From<Vec<u8, A>> for AllocString<A> {
    #[inline]
    fn from(value: Vec<u8, A>) -> Self {
        Self { vec: value }
    }
}

// ----

impl<A: Allocator> PartialEq for AllocString<A> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.vec.eq(&other.vec)
    }
}

impl<A: Allocator> PartialEq<String> for AllocString<A> {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.vec.eq(other.as_bytes())
    }
}

impl<A: Allocator> PartialEq<str> for AllocString<A> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.vec.eq(other.as_bytes())
    }
}

impl<A: Allocator> PartialEq<[u8]> for AllocString<A> {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.vec.eq(other)
    }
}

impl<A: Allocator> PartialEq<Vec<u8, A>> for AllocString<A> {
    #[inline]
    fn eq(&self, other: &Vec<u8, A>) -> bool {
        self.vec.eq(other)
    }
}

// ----

impl<A: Allocator> std::fmt::Display for AllocString<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(unsafe { std::str::from_utf8_unchecked(&self.vec) })
    }
}

impl<A: Allocator> std::fmt::Debug for AllocString<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(unsafe { std::str::from_utf8_unchecked(&self.vec) })
    }
}
