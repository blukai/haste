use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt::Debug;

use lazy_static::lazy_static;

use crate::bitreader::BitReader;

// NOTE: credit for figuring out field path encoding goes to invokr (github.com/dotabuff/manta) and
// spheenik (github.com/skadistats/clarity).

// NOTE: clarity's encodes all components into u64; see impl [1], and tech explanation [2].
//
// pretty cool stuff! but won't bring really any benefits / speedups to rust implemtnation; only
// cause extra overhead. unless i'm missing something, am i?
//
// [1] https://github.com/skadistats/clarity/blob/6dcdad4abe94a519b0c797576517461401adedee/src/main/java/skadistats/clarity/model/s2/S2LongFieldPathFormat.java
// [2] https://github.com/skadistats/clarity/commit/212eaddf7dc8b716c22faaec37952236f521a804#commitcomment-86037653

#[derive(Debug, Clone)]
pub struct FieldPath {
    pub(crate) data: [u8; 7],
    pub(crate) last: usize,
    pub(crate) finished: bool,
}

impl Default for FieldPath {
    #[inline(always)]
    fn default() -> Self {
        Self {
            data: [255, 0, 0, 0, 0, 0, 0],
            last: 0,
            finished: false,
        }
    }
}

impl FieldPath {
    // ops

    #[inline(always)]
    fn inc_at(&mut self, i: usize, v: i32) {
        self.data[i] = ((self.data[i] as i32 + v) & 0xFF) as u8;
    }

    #[inline(always)]
    fn inc_last(&mut self, v: i32) {
        self.inc_at(self.last, v);
    }

    #[inline(always)]
    fn push(&mut self, v: i32) {
        self.last += 1;
        self.data[self.last] = (v & 0xFF) as u8;
    }

    #[inline(always)]
    fn pop(&mut self, n: usize) {
        for _ in 0..n {
            self.data[self.last] = 0;
            self.last -= 1;
        }
    }

    // internal apis

    #[inline(always)]
    pub(crate) unsafe fn get_unchecked(&self, index: usize) -> usize {
        *self.data.get_unchecked(index) as usize
    }

    // public api

    // NOTE: using this method can hurt performance when used in critical code paths. use the
    // unsafe [`Self::get_unchecked`] instead.
    #[inline]
    pub fn get(&self, index: usize) -> Option<usize> {
        self.data.get(index).map(|component| *component as usize)
    }

    #[inline]
    pub fn last(&self) -> usize {
        self.last
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &u8> {
        self.data.iter().take(self.last + 1)
    }
}

type FieldOp = fn(&mut FieldPath, &mut BitReader);

// PlusOne
fn plus_one(fp: &mut FieldPath, _br: &mut BitReader) {
    fp.inc_last(1);
}

// PlusTwo
fn plus_two(fp: &mut FieldPath, _br: &mut BitReader) {
    fp.inc_last(2);
}

// PlusThree
fn plus_three(fp: &mut FieldPath, _br: &mut BitReader) {
    fp.inc_last(3);
}

// PlusFour
fn plus_four(fp: &mut FieldPath, _br: &mut BitReader) {
    fp.inc_last(4);
}

// PlusN
fn plus_n(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(br.read_ubitvarfp() as i32 + 5);
}

// PushOneLeftDeltaZeroRightZero
fn push_one_left_delta_zero_right_zero(fp: &mut FieldPath, _br: &mut BitReader) {
    fp.push(0);
}

// PushOneLeftDeltaZeroRightNonZero
fn push_one_left_delta_zero_right_non_zero(fp: &mut FieldPath, br: &mut BitReader) {
    fp.push(br.read_ubitvarfp() as i32);
}

// PushOneLeftDeltaOneRightZero
fn push_one_left_delta_one_right_zero(fp: &mut FieldPath, _br: &mut BitReader) {
    fp.inc_last(1);
    fp.push(0);
}

// PushOneLeftDeltaOneRightNonZero
fn push_one_left_delta_one_right_non_zero(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(1);
    fp.push(br.read_ubitvarfp() as i32);
}

// PushOneLeftDeltaNRightZero
fn push_one_left_delta_n_right_zero(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(br.read_ubitvarfp() as i32);
    fp.push(0);
}

// PushOneLeftDeltaNRightNonZero
fn push_one_left_delta_n_right_non_zero(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(br.read_ubitvarfp() as i32 + 2);
    fp.push(br.read_ubitvarfp() as i32 + 1);
}

// PushOneLeftDeltaNRightNonZeroPack6Bits
fn push_one_left_delta_n_right_non_zero_pack6_bits(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(br.read_ubit64(3) as i32 + 2);
    fp.push(br.read_ubit64(3) as i32 + 1);
}

// PushOneLeftDeltaNRightNonZeroPack8Bits
fn push_one_left_delta_n_right_non_zero_pack8_bits(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(br.read_ubit64(4) as i32 + 2);
    fp.push(br.read_ubit64(4) as i32 + 1);
}

// PushTwoLeftDeltaZero
fn push_two_left_delta_zero(fp: &mut FieldPath, br: &mut BitReader) {
    fp.push(br.read_ubitvarfp() as i32);
    fp.push(br.read_ubitvarfp() as i32);
}

// PushTwoLeftDeltaOne
fn push_two_left_delta_one(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(1);
    fp.push(br.read_ubitvarfp() as i32);
    fp.push(br.read_ubitvarfp() as i32);
}

// PushTwoLeftDeltaN
fn push_two_left_delta_n(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(br.read_ubitvar() as i32 + 2);
    fp.push(br.read_ubitvarfp() as i32);
    fp.push(br.read_ubitvarfp() as i32);
}

// PushTwoPack5LeftDeltaZero
fn push_two_pack5_left_delta_zero(fp: &mut FieldPath, br: &mut BitReader) {
    fp.push(br.read_ubit64(5) as i32);
    fp.push(br.read_ubit64(5) as i32);
}

// PushTwoPack5LeftDeltaOne
fn push_two_pack5_left_delta_one(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(1);
    fp.push(br.read_ubit64(5) as i32);
    fp.push(br.read_ubit64(5) as i32);
}

// PushTwoPack5LeftDeltaN
fn push_two_pack5_left_delta_n(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(br.read_ubitvar() as i32 + 2);
    fp.push(br.read_ubit64(5) as i32);
    fp.push(br.read_ubit64(5) as i32);
}

// PushThreeLeftDeltaZero
fn push_three_left_delta_zero(fp: &mut FieldPath, br: &mut BitReader) {
    fp.push(br.read_ubitvarfp() as i32);
    fp.push(br.read_ubitvarfp() as i32);
    fp.push(br.read_ubitvarfp() as i32);
}

// PushThreeLeftDeltaOne
fn push_three_left_delta_one(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(1);
    fp.push(br.read_ubitvarfp() as i32);
    fp.push(br.read_ubitvarfp() as i32);
    fp.push(br.read_ubitvarfp() as i32);
}

// PushThreeLeftDeltaN
fn push_three_left_delta_n(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(br.read_ubitvar() as i32 + 2);
    fp.push(br.read_ubitvarfp() as i32);
    fp.push(br.read_ubitvarfp() as i32);
    fp.push(br.read_ubitvarfp() as i32);
}

// PushThreePack5LeftDeltaZero
fn push_three_pack5_left_delta_zero(fp: &mut FieldPath, br: &mut BitReader) {
    fp.push(br.read_ubit64(5) as i32);
    fp.push(br.read_ubit64(5) as i32);
    fp.push(br.read_ubit64(5) as i32);
}

// PushThreePack5LeftDeltaOne
fn push_three_pack5_left_delta_one(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(1);
    fp.push(br.read_ubit64(5) as i32);
    fp.push(br.read_ubit64(5) as i32);
    fp.push(br.read_ubit64(5) as i32);
}

// PushThreePack5LeftDeltaN
fn push_three_pack5_left_delta_n(fp: &mut FieldPath, br: &mut BitReader) {
    fp.inc_last(br.read_ubitvar() as i32 + 2);
    fp.push(br.read_ubit64(5) as i32);
    fp.push(br.read_ubit64(5) as i32);
    fp.push(br.read_ubit64(5) as i32);
}

// PushN
fn push_n(fp: &mut FieldPath, br: &mut BitReader) {
    let n = br.read_ubitvar() as usize;
    fp.inc_last(br.read_ubitvar() as i32);
    for _ in 0..n {
        fp.push(br.read_ubitvarfp() as i32);
    }
}

// PushNAndNonTopographical
fn push_n_and_non_topographical(fp: &mut FieldPath, br: &mut BitReader) {
    for i in 0..=fp.last {
        if br.read_bool() {
            fp.inc_at(i, br.read_varint32() + 1);
        }
    }
    let n = br.read_ubitvar() as usize;
    for _ in 0..n {
        fp.push(br.read_ubitvarfp() as i32);
    }
}

// PopOnePlusOne
fn pop_one_plus_one(fp: &mut FieldPath, _br: &mut BitReader) {
    fp.pop(1);
    fp.inc_last(1);
}

// PopOnePlusN
fn pop_one_plus_n(fp: &mut FieldPath, br: &mut BitReader) {
    fp.pop(1);
    fp.inc_last(br.read_ubitvarfp() as i32 + 1);
}

// PopAllButOnePlusOne
fn pop_all_but_one_plus_one(fp: &mut FieldPath, _br: &mut BitReader) {
    fp.pop(fp.last);
    fp.inc_last(1);
}

// PopAllButOnePlusN
fn pop_all_but_one_plus_n(fp: &mut FieldPath, br: &mut BitReader) {
    fp.pop(fp.last);
    fp.inc_last(br.read_ubitvarfp() as i32 + 1);
}

// PopAllButOnePlusNPack3Bits
fn pop_all_but_one_plus_n_pack3_bits(fp: &mut FieldPath, br: &mut BitReader) {
    fp.pop(fp.last);
    fp.inc_last(br.read_ubit64(3) as i32 + 1);
}

// PopAllButOnePlusNPack6Bits
fn pop_all_but_one_plus_n_pack6_bits(fp: &mut FieldPath, br: &mut BitReader) {
    fp.pop(fp.last);
    fp.inc_last(br.read_ubit64(6) as i32 + 1);
}

// PopNPlusOne
fn pop_n_plus_one(fp: &mut FieldPath, br: &mut BitReader) {
    fp.pop(br.read_ubitvarfp() as usize);
    fp.inc_last(1);
}

// PopNPlusN
fn pop_n_plus_n(fp: &mut FieldPath, br: &mut BitReader) {
    fp.pop(br.read_ubitvarfp() as usize);
    fp.inc_last(br.read_varint32());
}

// PopNAndNonTopographical
fn pop_n_and_non_topographical(fp: &mut FieldPath, br: &mut BitReader) {
    fp.pop(br.read_ubitvarfp() as usize);
    for i in 0..=fp.last {
        if br.read_bool() {
            fp.inc_at(i, br.read_varint32());
        }
    }
}

// NonTopoComplex
fn non_topo_complex(fp: &mut FieldPath, br: &mut BitReader) {
    for i in 0..=fp.last {
        if br.read_bool() {
            fp.inc_at(i, br.read_varint32());
        }
    }
}

// NonTopoPenultimatePluseOne
fn non_topo_penultimate_pluse_one(fp: &mut FieldPath, _br: &mut BitReader) {
    fp.inc_at(fp.last - 1, 1);
}

// NonTopoComplexPack4Bits
fn non_topo_complex_pack4_bits(fp: &mut FieldPath, br: &mut BitReader) {
    for i in 0..=fp.last {
        if br.read_bool() {
            fp.inc_at(i, br.read_ubit64(4) as i32 - 7);
        }
    }
}

// FieldPathEncodeFinish
fn field_path_encode_finish(fp: &mut FieldPath, _br: &mut BitReader) {
    // NOCOMMIT
    fp.finished = true
}

// NOTE: for some random reference, manual rust vtable impls:
// - https://doc.rust-lang.org/std/task/struct.RawWakerVTable.html
// - https://github.com/tokio-rs/tokio/blob/67bf9c36f347031ca05872d102a7f9abc8b465f0/tokio/src/task/raw.rs#L12-L42

#[derive(Debug)]
struct FieldOpDescriptor {
    weight: usize,
    op: FieldOp,
}

const FIELDOP_DESCRIPTORS: &[FieldOpDescriptor] = &[
    FieldOpDescriptor {
        weight: 36271,
        op: plus_one,
    },
    FieldOpDescriptor {
        weight: 10334,
        op: plus_two,
    },
    FieldOpDescriptor {
        weight: 1375,
        op: plus_three,
    },
    FieldOpDescriptor {
        weight: 646,
        op: plus_four,
    },
    FieldOpDescriptor {
        weight: 4128,
        op: plus_n,
    },
    FieldOpDescriptor {
        weight: 35,
        op: push_one_left_delta_zero_right_zero,
    },
    FieldOpDescriptor {
        weight: 3,
        op: push_one_left_delta_zero_right_non_zero,
    },
    FieldOpDescriptor {
        weight: 521,
        op: push_one_left_delta_one_right_zero,
    },
    FieldOpDescriptor {
        weight: 2942,
        op: push_one_left_delta_one_right_non_zero,
    },
    FieldOpDescriptor {
        weight: 560,
        op: push_one_left_delta_n_right_zero,
    },
    FieldOpDescriptor {
        weight: 471,
        op: push_one_left_delta_n_right_non_zero,
    },
    FieldOpDescriptor {
        weight: 10530,
        op: push_one_left_delta_n_right_non_zero_pack6_bits,
    },
    FieldOpDescriptor {
        weight: 251,
        op: push_one_left_delta_n_right_non_zero_pack8_bits,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_two_left_delta_zero,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_two_pack5_left_delta_zero,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_three_left_delta_zero,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_three_pack5_left_delta_zero,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_two_left_delta_one,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_two_pack5_left_delta_one,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_three_left_delta_one,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_three_pack5_left_delta_one,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_two_left_delta_n,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_two_pack5_left_delta_n,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_three_left_delta_n,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_three_pack5_left_delta_n,
    },
    FieldOpDescriptor {
        weight: 1,
        op: push_n,
    },
    FieldOpDescriptor {
        weight: 310,
        op: push_n_and_non_topographical,
    },
    FieldOpDescriptor {
        weight: 2,
        op: pop_one_plus_one,
    },
    FieldOpDescriptor {
        weight: 1,
        op: pop_one_plus_n,
    },
    FieldOpDescriptor {
        weight: 1837,
        op: pop_all_but_one_plus_one,
    },
    FieldOpDescriptor {
        weight: 149,
        op: pop_all_but_one_plus_n,
    },
    FieldOpDescriptor {
        weight: 300,
        op: pop_all_but_one_plus_n_pack3_bits,
    },
    FieldOpDescriptor {
        weight: 634,
        op: pop_all_but_one_plus_n_pack6_bits,
    },
    FieldOpDescriptor {
        weight: 1,
        op: pop_n_plus_one,
    },
    FieldOpDescriptor {
        weight: 1,
        op: pop_n_plus_n,
    },
    FieldOpDescriptor {
        weight: 1,
        op: pop_n_and_non_topographical,
    },
    FieldOpDescriptor {
        weight: 76,
        op: non_topo_complex,
    },
    FieldOpDescriptor {
        weight: 271,
        op: non_topo_penultimate_pluse_one,
    },
    FieldOpDescriptor {
        weight: 99,
        op: non_topo_complex_pack4_bits,
    },
    FieldOpDescriptor {
        weight: 25474,
        op: field_path_encode_finish,
    },
];

#[derive(Debug)]
enum Node<T: Debug> {
    Leaf {
        weight: usize,
        num: usize,
        value: T,
    },
    Branch {
        weight: usize,
        num: usize,
        left: Box<Node<T>>,
        right: Box<Node<T>>,
    },
}

impl<T: Debug> Node<T> {
    fn weight(&self) -> usize {
        match self {
            Self::Leaf { weight, .. } => *weight,
            Self::Branch { weight, .. } => *weight,
        }
    }

    fn num(&self) -> usize {
        match self {
            Self::Leaf { num, .. } => *num,
            Self::Branch { num, .. } => *num,
        }
    }

    fn unwrap_left_branch(&self) -> &Self {
        match self {
            Self::Branch { ref left, .. } => left,
            _ => unreachable!(),
        }
    }

    fn unwrap_right_branch(&self) -> &Self {
        match self {
            Self::Branch { ref right, .. } => right,
            _ => unreachable!(),
        }
    }
}

impl<T: Debug> Ord for Node<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.weight() == other.weight() {
            self.num().cmp(&other.num())
        } else {
            other.weight().cmp(&self.weight())
        }
    }
}

impl<T: Debug> PartialOrd for Node<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Debug> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.weight() == other.weight() && self.num() == other.num()
    }
}

impl<T: Debug> Eq for Node<T> {}

fn build_fieldop_hierarchy() -> Node<FieldOp> {
    let mut bh = BinaryHeap::with_capacity(FIELDOP_DESCRIPTORS.len());

    // valve's huffman-tree uses a variation which takes the node number into account
    let mut num = 0;

    for fod in FIELDOP_DESCRIPTORS.iter() {
        bh.push(Node::Leaf {
            weight: fod.weight,
            num,
            value: fod.op,
        });
        num += 1;
    }

    while bh.len() > 1 {
        let left = bh.pop().unwrap();
        let right = bh.pop().unwrap();
        bh.push(Node::Branch {
            weight: left.weight() + right.weight(),
            num,
            left: Box::new(left),
            right: Box::new(right),
        });
        num += 1;
    }

    bh.pop().unwrap()
}

lazy_static! {
    static ref FIELDOP_HIERARCHY: Node<FieldOp> = build_fieldop_hierarchy();
}

pub(crate) fn read_field_paths(br: &mut BitReader, fps: &mut [FieldPath]) -> usize {
    // NOTE: majority of field path reads are shorter then 32 (but some are beyond thousand).

    // it is more efficient to walk huffman tree, then to do static lookups by first accumulating
    // all the bits (like butterfly does [1]), because hierarchical structure allows making
    // decisions based on variable values which results in reduction of branch misses of otherwise
    // quite large (40 branches) match.
    //
    // accumulative lookups vs tree walking (the winner):
    // - ~14% branch miss reduction
    // - ~8% execution time reduction
    //
    // ^ representing improvements in percentages because milliseconds are meaningless because
    // replay sized / durations are different; perecentage improvement is consistent across
    // different replay sizes.
    //
    // [1] https://github.com/ButterflyStats/butterfly/blob/339e91a882cadc1a8f72446616f7d7f1480c3791/src/butterfly/private/entity.cpp#L93

    let mut fp = FieldPath::default();
    let mut i: usize = 0;

    let mut root: &Node<FieldOp> = &FIELDOP_HIERARCHY;

    loop {
        let next = if br.read_bool() {
            root.unwrap_right_branch()
        } else {
            root.unwrap_left_branch()
        };

        root = if let Node::Leaf { value: op, .. } = next {
            // NOTE: this is not any worse then a method call (on a struct for example), right?
            // because what vtables contain? they contain pointers.
            (op)(&mut fp, br);
            if fp.finished {
                return i;
            }
            fps[i] = fp.clone();

            i += 1;
            assert!(i <= fps.len());

            &FIELDOP_HIERARCHY
        } else {
            next
        };
    }
}
