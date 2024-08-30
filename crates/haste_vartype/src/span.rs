use std::fmt::Debug;

// start and end are byte positions relative to the beginning of the input.
//
// it might be intuitive to use usize to represent byte positions, but there's absolutely no need
// for that. usize is a pointer-size type. u16 is more then enough to fit numbers that are needed
// to be fit. with u16' size of whole struct is 4 bytes while with usize' it would have been 16
// (unless u're running this on some obscure os / hardware.
#[derive(PartialEq, Eq, Clone)]
pub struct Span {
    pub start: u16,
    pub end: u16,
}

// NOTE: custom implementation of Debug trait makes Span's debug printing more compact which
// improves readability.
impl Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Span {{ {}, {} }}", self.start, self.end))
    }
}

impl Span {
    pub fn new(start: u16, end: u16) -> Self {
        debug_assert!(start <= end);
        Self { start, end }
    }

    pub fn to(self, end: Self) -> Self {
        Self::new(self.start, end.end)
    }
}
