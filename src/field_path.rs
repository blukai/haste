use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    fmt::{Display, Write},
};

use crate::{bitreader::BitReader, error::Result};

#[derive(Debug, Clone)]
pub struct FieldPath {
    pub data: [i32; 7],
    pub position: usize,
    pub finished: bool,
}

impl FieldPath {
    pub fn new() -> Self {
        Self {
            data: [-1, 0, 0, 0, 0, 0, 0],
            position: 0,
            finished: false,
        }
    }

    fn push_back(&mut self, value: i32) {
        self.position += 1;
        self.data[self.position] = value;
    }

    fn pop(&mut self, n: usize) {
        for _ in 0..n {
            self.data[self.position] = 0;
            self.position -= 1;
        }
    }
}

impl Display for FieldPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..=self.position {
            f.write_str(&self.data[i].to_compact_string())?;
            if i < self.position {
                f.write_char('/')?;
            }
        }
        Ok(())
    }
}

// NOTE: i don't want to allow snake case for entire file, it seems like it can
// be applied exclusibely to mod, that's good.
#[allow(non_snake_case)]
mod op {
    use super::FieldPath;
    use crate::{bitreader::BitReader, error::Result};

    #[inline(always)]
    pub fn PlusOne(f: &mut FieldPath, _b: &mut BitReader) -> Result<()> {
        f.data[f.position] += 1;
        Ok(())
    }

    #[inline(always)]
    pub fn PlusTwo(f: &mut FieldPath, _b: &mut BitReader) -> Result<()> {
        f.data[f.position] += 2;
        Ok(())
    }

    #[inline(always)]
    pub fn PlusThree(f: &mut FieldPath, _b: &mut BitReader) -> Result<()> {
        f.data[f.position] += 3;
        Ok(())
    }

    #[inline(always)]
    pub fn PlusFour(f: &mut FieldPath, _b: &mut BitReader) -> Result<()> {
        f.data[f.position] += 4;
        Ok(())
    }

    #[inline(always)]
    pub fn PlusN(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        f.data[f.position] += b.read_fpbitvar()? + 5;
        Ok(())
    }

    #[inline(always)]
    pub fn PushOneLeftDeltaZeroRightZero(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        f.position += 1;
        f.data[f.position] = 0;
        Ok(())
    }

    #[inline(always)]
    pub fn PushOneLeftDeltaZeroRightNonZero(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data.push_back(b.readFPBitVar());
    }

    #[inline(always)]
    pub fn PushOneLeftDeltaOneRightZero(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        f.data[f.position] += 1;
        f.push_back(0);
        Ok(())
    }

    #[inline(always)]
    pub fn PushOneLeftDeltaOneRightNonZero(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += 1;
        // f.data.push_back(b.readFPBitVar());
    }

    #[inline(always)]
    pub fn PushOneLeftDeltaNRightZero(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += b.readFPBitVar();
        // f.data.push_back(0);
    }

    #[inline(always)]
    pub fn PushOneLeftDeltaNRightNonZero(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += b.readFPBitVar() + 2;
        // f.data.push_back(b.readFPBitVar() + 1);
    }

    #[inline(always)]
    pub fn PushOneLeftDeltaNRightNonZeroPack6Bits(
        f: &mut FieldPath,
        b: &mut BitReader,
    ) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += b.read(3) + 2;
        // f.data.push_back(b.read(3) + 1);
    }

    #[inline(always)]
    pub fn PushOneLeftDeltaNRightNonZeroPack8Bits(
        f: &mut FieldPath,
        b: &mut BitReader,
    ) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += b.read(4) + 2;
        // f.data.push_back(b.read(4) + 1);
    }

    #[inline(always)]
    pub fn PushTwoLeftDeltaZero(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data.push_back(b.readFPBitVar());
        // f.data.push_back(b.readFPBitVar());
    }

    #[inline(always)]
    pub fn PushTwoLeftDeltaOne(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += 1;
        // f.data.push_back(b.readFPBitVar());
        // f.data.push_back(b.readFPBitVar());
    }

    #[inline(always)]
    pub fn PushTwoLeftDeltaN(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += b.readUBitVar() + 2;
        // f.data.push_back(b.readFPBitVar());
        // f.data.push_back(b.readFPBitVar());
    }

    #[inline(always)]
    pub fn PushTwoPack5LeftDeltaZero(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data.push_back(b.read(5));
        // f.data.push_back(b.read(5));
    }

    #[inline(always)]
    pub fn PushTwoPack5LeftDeltaOne(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += 1;
        // f.data.push_back(b.read(5));
        // f.data.push_back(b.read(5));
    }

    #[inline(always)]
    pub fn PushTwoPack5LeftDeltaN(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += b.readUBitVar() + 2;
        // f.data.push_back(b.read(5));
        // f.data.push_back(b.read(5));
    }

    #[inline(always)]
    pub fn PushThreeLeftDeltaZero(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data.push_back(b.readFPBitVar());
        // f.data.push_back(b.readFPBitVar());
        // f.data.push_back(b.readFPBitVar());
    }

    #[inline(always)]
    pub fn PushThreeLeftDeltaOne(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += 1;
        // f.data.push_back(b.readFPBitVar());
        // f.data.push_back(b.readFPBitVar());
        // f.data.push_back(b.readFPBitVar());
    }

    #[inline(always)]
    pub fn PushThreeLeftDeltaN(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += b.readUBitVar() + 2;
        // f.data.push_back(b.readFPBitVar());
        // f.data.push_back(b.readFPBitVar());
        // f.data.push_back(b.readFPBitVar());
    }

    #[inline(always)]
    pub fn PushThreePack5LeftDeltaZero(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data.push_back(b.read(5));
        // f.data.push_back(b.read(5));
        // f.data.push_back(b.read(5));
    }

    #[inline(always)]
    pub fn PushThreePack5LeftDeltaOne(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += 1;
        // f.data.push_back(b.read(5));
        // f.data.push_back(b.read(5));
        // f.data.push_back(b.read(5));
    }

    #[inline(always)]
    pub fn PushThreePack5LeftDeltaN(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data[f.position] += b.readUBitVar() + 2;
        // f.data.push_back(b.read(5));
        // f.data.push_back(b.read(5));
        // f.data.push_back(b.read(5));
    }

    #[inline(always)]
    pub fn PushN(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // uint32_t n = b.readUBitVar();

        // for (uint32_t i = 0; i < n; ++i) {
        //     f.data.push_back(b.readFPBitVar());
        // }
    }

    #[inline(always)]
    pub fn PushNAndNonTopological(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // for (auto &idx : f.data) {
        //     if (b.read(1)) idx += b.readVarSInt32() + 1;
        // }

        // uint32_t n = b.readUBitVar();
        // for (uint32_t i = 0; i < n; ++i) {
        //     f.data.push_back(b.readFPBitVar());
        // }
    }

    #[inline(always)]
    pub fn PopOnePlusOne(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data.pop_back();
        // f.data[f.position] += 1;
    }

    #[inline(always)]
    pub fn PopOnePlusN(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data.pop_back();
        // f.data[f.position] += b.readFPBitVar() + 1;
    }

    #[inline(always)]
    pub fn PopAllButOnePlusOne(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        f.pop(f.position);
        f.data[f.position] += 1;
        Ok(())
    }

    #[inline(always)]
    pub fn PopAllButOnePlusN(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // f.data.resize(1);
        // f.data[f.position] += b.readFPBitVar() + 1;
    }

    #[inline(always)]
    pub fn PopAllButOnePlusNPack3Bits(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        f.pop(f.position);
        f.data[f.position] = b.read(3)? as i32 + 1;
        Ok(())
    }

    #[inline(always)]
    pub fn PopAllButOnePlusNPack6Bits(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        f.pop(f.position);
        f.data[f.position] = b.read(6)? as i32 + 1;
        Ok(())
    }

    #[inline(always)]
    pub fn PopNPlusOne(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // uint32_t nsize = f.data.size() - b.readFPBitVar();
        // ASSERT_TRUE(nsize < 7 && nsize > 0,  "Invalid fp size for op");

        // f.data.resize(nsize);
        // f.data[f.position] += 1;
    }

    #[inline(always)]
    pub fn PopNPlusN(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // uint32_t nsize = f.data.size() - b.readFPBitVar();
        // ASSERT_TRUE(nsize < 7 && nsize > 0,  "Invalid fp size for op");

        // f.data.resize(nsize);
        // f.data[f.position] += b.readVarSInt32();
    }

    #[inline(always)]
    pub fn PopNAndNonTopographical(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // uint32_t nsize = f.data.size() - b.readFPBitVar();
        // ASSERT_TRUE(nsize < 7 && nsize > 0,  "Invalid fp size for op");

        // f.data.resize(nsize);

        // for (auto &idx : f.data) {
        //     if (b.read(1)) idx += b.readVarSInt32();
        // }
    }

    #[inline(always)]
    pub fn NonTopoComplex(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        for i in 0..=f.position {
            if b.read_bool()? {
                f.data[i] += b.read_vari32()?;
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub fn NonTopoPenultimatePlusOne(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        unimplemented!()
        // ASSERT_TRUE(f.data.size() >= 2, "Invalid fp size for op");
        // f.data[f.data.size() - 2] += 1;
    }

    #[inline(always)]
    pub fn NonTopoComplexPack4Bits(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        for i in 0..=f.position {
            if b.read_bool()? {
                f.data[i] += b.read(4)? as i32 - 7;
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub fn FieldPathEncodeFinish(f: &mut FieldPath, b: &mut BitReader) -> Result<()> {
        f.finished = true;
        Ok(())
    }
}

use compact_str::{CompactString, ToCompactString};
use op::*;

pub struct FieldOp {
    pub name: &'static str,
    pub weight: u32,
    pub fp: fn(fp: &mut FieldPath, br: &mut BitReader) -> Result<()>,
}

pub const FIELD_OPS: [FieldOp; 40] = [
    FieldOp {
        name: "PlusOne",
        weight: 36271,
        fp: PlusOne,
    },
    FieldOp {
        name: "PlusTwo",
        weight: 10334,
        fp: PlusTwo,
    },
    FieldOp {
        name: "PlusThree",
        weight: 1375,
        fp: PlusThree,
    },
    FieldOp {
        name: "PlusFour",
        weight: 646,
        fp: PlusFour,
    },
    FieldOp {
        name: "PlusN",
        weight: 4128,
        fp: PlusN,
    },
    FieldOp {
        name: "PushOneLeftDeltaZeroRightZero",
        weight: 35,
        fp: PushOneLeftDeltaZeroRightZero,
    },
    FieldOp {
        name: "PushOneLeftDeltaZeroRightNonZero",
        weight: 3,
        fp: PushOneLeftDeltaZeroRightNonZero,
    },
    FieldOp {
        name: "PushOneLeftDeltaOneRightZero",
        weight: 521,
        fp: PushOneLeftDeltaOneRightZero,
    },
    FieldOp {
        name: "PushOneLeftDeltaOneRightNonZero",
        weight: 2942,
        fp: PushOneLeftDeltaOneRightNonZero,
    },
    FieldOp {
        name: "PushOneLeftDeltaNRightZero",
        weight: 560,
        fp: PushOneLeftDeltaNRightZero,
    },
    FieldOp {
        name: "PushOneLeftDeltaNRightNonZero",
        weight: 471,
        fp: PushOneLeftDeltaNRightNonZero,
    },
    FieldOp {
        name: "PushOneLeftDeltaNRightNonZeroPack6Bits",
        weight: 10530,
        fp: PushOneLeftDeltaNRightNonZeroPack6Bits,
    },
    FieldOp {
        name: "PushOneLeftDeltaNRightNonZeroPack8Bits",
        weight: 251,
        fp: PushOneLeftDeltaNRightNonZeroPack8Bits,
    },
    FieldOp {
        name: "PushTwoLeftDeltaZero",
        weight: 1,
        fp: PushTwoLeftDeltaZero,
    },
    FieldOp {
        name: "PushTwoPack5LeftDeltaZero",
        weight: 1,
        fp: PushTwoPack5LeftDeltaZero,
    },
    FieldOp {
        name: "PushThreeLeftDeltaZero",
        weight: 1,
        fp: PushThreeLeftDeltaZero,
    },
    FieldOp {
        name: "PushThreePack5LeftDeltaZero",
        weight: 1,
        fp: PushThreePack5LeftDeltaZero,
    },
    FieldOp {
        name: "PushTwoLeftDeltaOne",
        weight: 1,
        fp: PushTwoLeftDeltaOne,
    },
    FieldOp {
        name: "PushTwoPack5LeftDeltaOne",
        weight: 1,
        fp: PushTwoPack5LeftDeltaOne,
    },
    FieldOp {
        name: "PushThreeLeftDeltaOne",
        weight: 1,
        fp: PushThreeLeftDeltaOne,
    },
    FieldOp {
        name: "PushThreePack5LeftDeltaOne",
        weight: 1,
        fp: PushThreePack5LeftDeltaOne,
    },
    FieldOp {
        name: "PushTwoLeftDeltaN",
        weight: 1,
        fp: PushTwoLeftDeltaN,
    },
    FieldOp {
        name: "PushTwoPack5LeftDeltaN",
        weight: 1,
        fp: PushTwoPack5LeftDeltaN,
    },
    FieldOp {
        name: "PushThreeLeftDeltaN",
        weight: 1,
        fp: PushThreeLeftDeltaN,
    },
    FieldOp {
        name: "PushThreePack5LeftDeltaN",
        weight: 1,
        fp: PushThreePack5LeftDeltaN,
    },
    FieldOp {
        name: "PushN",
        weight: 1,
        fp: PushN,
    },
    FieldOp {
        name: "PushNAndNonTopological",
        weight: 310,
        fp: PushNAndNonTopological,
    },
    FieldOp {
        name: "PopOnePlusOne",
        weight: 2,
        fp: PopOnePlusOne,
    },
    FieldOp {
        name: "PopOnePlusN",
        weight: 1,
        fp: PopOnePlusN,
    },
    FieldOp {
        name: "PopAllButOnePlusOne",
        weight: 1837,
        fp: PopAllButOnePlusOne,
    },
    FieldOp {
        name: "PopAllButOnePlusN",
        weight: 149,
        fp: PopAllButOnePlusN,
    },
    FieldOp {
        name: "PopAllButOnePlusNPack3Bits",
        weight: 300,
        fp: PopAllButOnePlusNPack3Bits,
    },
    FieldOp {
        name: "PopAllButOnePlusNPack6Bits",
        weight: 634,
        fp: PopAllButOnePlusNPack6Bits,
    },
    FieldOp {
        name: "PopNPlusOne",
        weight: 1,
        fp: PopNPlusOne,
    },
    FieldOp {
        name: "PopNPlusN",
        weight: 1,
        fp: PopNPlusN,
    },
    FieldOp {
        name: "PopNAndNonTopographical",
        weight: 1,
        fp: PopNAndNonTopographical,
    },
    FieldOp {
        name: "NonTopoComplex",
        weight: 76,
        fp: NonTopoComplex,
    },
    FieldOp {
        name: "NonTopoPenultimatePlusOne",
        weight: 271,
        fp: NonTopoPenultimatePlusOne,
    },
    FieldOp {
        name: "NonTopoComplexPack4Bits",
        weight: 99,
        fp: NonTopoComplexPack4Bits,
    },
    FieldOp {
        name: "FieldPathEncodeFinish",
        weight: 25474,
        fp: FieldPathEncodeFinish,
    },
];

// based on https://github.com/Lakret/huffman-rs/blob/4e2f759e2ca384108e5c95bc9cd365fad1d48364/src/huffman.rs
// NOTE: Tree does not have an A: Allocator generic param because it's not
// supposed to be used in real code, but for code generation.
#[derive(Debug, PartialEq, Eq)]
pub enum Tree<V> {
    Leaf {
        weight: u32,
        value: V,
    },
    Node {
        weight: u32,
        left: Box<Tree<V>>,
        right: Box<Tree<V>>,
    },
}

impl<V: Clone> Tree<V> {
    pub fn weight(&self) -> u32 {
        match self {
            Self::Leaf { weight, .. } => *weight,
            Self::Node { weight, .. } => *weight,
        }
    }

    pub fn value(&self) -> Option<V> {
        match self {
            Self::Leaf { value, .. } => Some(value.clone()),
            Self::Node { .. } => None,
        }
    }

    pub fn left(&self) -> Option<&Tree<V>> {
        match self {
            Self::Node { left, .. } => Some(left),
            Self::Leaf { .. } => None,
        }
    }

    pub fn right(&self) -> Option<&Tree<V>> {
        match self {
            Self::Node { right, .. } => Some(right),
            Self::Leaf { .. } => None,
        }
    }
}

impl<V: Clone + Eq> Ord for Tree<V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.weight().cmp(&other.weight())
    }
}

impl<V: Clone + Eq> PartialOrd for Tree<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn build_field_ops_tree() -> Tree<usize> {
    let mut heap = BinaryHeap::with_capacity(FIELD_OPS.len());
    for (i, fop) in FIELD_OPS.iter().enumerate() {
        heap.push(Reverse(Tree::Leaf {
            weight: fop.weight,
            value: i,
        }));
    }

    while heap.len() > 1 {
        let a = heap.pop().unwrap().0;
        let b = heap.pop().unwrap().0;
        let merged_node = Tree::Node {
            weight: a.weight() + b.weight(),
            left: Box::new(a),
            right: Box::new(b),
        };
        heap.push(Reverse(merged_node));
    }

    heap.pop().unwrap().0
}

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn test_build_field_ops_tree() {
//         dbg!(super::build_field_ops_tree());
//     }
// }
