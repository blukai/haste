use crate::bitbuf::{self, BitReader};
use std::alloc::Allocator;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // crate
    #[error(transparent)]
    BitBuf(#[from] bitbuf::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct FieldPath {
    pub data: [i32; 7],
    pub position: usize,
    pub finished: bool,
}

impl FieldPath {
    fn new() -> Self {
        Self {
            data: [-1, 0, 0, 0, 0, 0, 0],
            position: 0,
            finished: false,
        }
    }

    #[inline(always)]
    fn push_back(&mut self, value: i32) {
        self.position += 1;
        self.data[self.position] = value;
    }

    #[inline(always)]
    fn pop(&mut self, n: usize) {
        for _ in 0..n {
            self.data[self.position] = 0;
            self.position -= 1;
        }
    }
}

// PlusOne
#[inline(always)]
fn plus_one(fp: &mut FieldPath, _br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 1;
    Ok(())
}

// PlusTwo
#[inline(always)]
fn plus_two(fp: &mut FieldPath, _br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 2;
    Ok(())
}

// PlusThree
#[inline(always)]
fn plus_three(fp: &mut FieldPath, _br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 3;
    Ok(())
}

// PlusFour
#[inline(always)]
fn plus_four(fp: &mut FieldPath, _br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 4;
    Ok(())
}

// PlusN
#[inline(always)]
fn plus_n(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += br.read_ubitvarfp()? as i32 + 5;
    Ok(())
}

// PushOneLeftDeltaZeroRightZero
#[inline(always)]
fn push_one_left_delta_zero_right_zero(fp: &mut FieldPath, _br: &mut BitReader) -> Result<()> {
    fp.push_back(0);
    Ok(())
}

// PushOneLeftDeltaZeroRightNonZero
#[inline(always)]
fn push_one_left_delta_zero_right_non_zero(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushOneLeftDeltaOneRightZero
#[inline(always)]
fn push_one_left_delta_one_right_zero(fp: &mut FieldPath, _br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 1;
    fp.push_back(0);
    Ok(())
}

// PushOneLeftDeltaOneRightNonZero
#[inline(always)]
fn push_one_left_delta_one_right_non_zero(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 1;
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushOneLeftDeltaNRightZero
#[inline(always)]
fn push_one_left_delta_n_right_zero(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += br.read_ubitvarfp()? as i32;
    fp.push_back(0);
    Ok(())
}

// PushOneLeftDeltaNRightNonZero
#[inline(always)]
fn push_one_left_delta_n_right_non_zero(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += br.read_ubitvarfp()? as i32 + 2;
    fp.push_back(br.read_ubitvarfp()? as i32 + 1);
    Ok(())
}

// PushOneLeftDeltaNRightNonZeroPack6Bits
#[inline(always)]
fn push_one_left_delta_n_right_non_zero_pack6_bits(
    fp: &mut FieldPath,
    br: &mut BitReader,
) -> Result<()> {
    fp.data[fp.position] += br.read_ubitlong(3)? as i32 + 2;
    fp.push_back(br.read_ubitlong(3)? as i32 + 1);
    Ok(())
}

// PushOneLeftDeltaNRightNonZeroPack8Bits
#[inline(always)]
fn push_one_left_delta_n_right_non_zero_pack8_bits(
    fp: &mut FieldPath,
    br: &mut BitReader,
) -> Result<()> {
    fp.data[fp.position] += br.read_ubitlong(4)? as i32 + 2;
    fp.push_back(br.read_ubitlong(4)? as i32 + 1);
    Ok(())
}

// PushTwoLeftDeltaZero
#[inline(always)]
fn push_two_left_delta_zero(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushTwoLeftDeltaOne
#[inline(always)]
fn push_two_left_delta_one(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 1;
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushTwoLeftDeltaN
#[inline(always)]
fn push_two_left_delta_n(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += br.read_ubitvar()? as i32 + 2;
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushTwoPack5LeftDeltaZero
#[inline(always)]
fn push_two_pack5_left_delta_zero(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    Ok(())
}

// PushTwoPack5LeftDeltaOne
#[inline(always)]
fn push_two_pack5_left_delta_one(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 1;
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    Ok(())
}

// PushTwoPack5LeftDeltaN
#[inline(always)]
fn push_two_pack5_left_delta_n(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += br.read_ubitvar()? as i32 + 2;
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    Ok(())
}

// PushThreeLeftDeltaZero
#[inline(always)]
fn push_three_left_delta_zero(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushThreeLeftDeltaOne
#[inline(always)]
fn push_three_left_delta_one(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 1;
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushThreeLeftDeltaN
#[inline(always)]
fn push_three_left_delta_n(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += br.read_ubitvar()? as i32 + 2;
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushThreePack5LeftDeltaZero
#[inline(always)]
fn push_three_pack5_left_delta_zero(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    Ok(())
}

// PushThreePack5LeftDeltaOne
#[inline(always)]
fn push_three_pack5_left_delta_one(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 1;
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    Ok(())
}

// PushThreePack5LeftDeltaN
#[inline(always)]
fn push_three_pack5_left_delta_n(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += br.read_ubitvar()? as i32 + 2;
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    Ok(())
}

// PushN
#[inline(always)]
fn push_n(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    let n = br.read_ubitvar()? as usize;
    fp.data[fp.position] += br.read_ubitvar()? as i32;
    for _ in 0..n {
        fp.push_back(br.read_ubitvarfp()? as i32);
    }
    Ok(())
}

// PushNAndNonTopological
#[inline(always)]
fn push_n_and_non_topological(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    for i in 0..=fp.position {
        if br.read_bool()? {
            fp.data[i] += br.read_varint32()? + 1;
        }
    }
    let n = br.read_ubitvar()? as usize;
    for _ in 0..n {
        fp.push_back(br.read_ubitvarfp()? as i32);
    }
    Ok(())
}

// PopOnePlusOne
#[inline(always)]
fn pop_one_plus_one(fp: &mut FieldPath, _br: &mut BitReader) -> Result<()> {
    fp.pop(1);
    fp.data[fp.position] += 1;
    Ok(())
}

// PopOnePlusN
#[inline(always)]
fn pop_one_plus_n(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.pop(1);
    fp.data[fp.position] += br.read_ubitvarfp()? as i32 + 1;
    Ok(())
}

// PopAllButOnePlusOne
#[inline(always)]
fn pop_all_but_one_plus_one(fp: &mut FieldPath, _br: &mut BitReader) -> Result<()> {
    fp.pop(fp.position);
    fp.data[fp.position] += 1;
    Ok(())
}

// PopAllButOnePlusN
#[inline(always)]
fn pop_all_but_one_plus_n(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.pop(fp.position);
    fp.data[fp.position] += br.read_ubitvarfp()? as i32 + 1;
    Ok(())
}

// PopAllButOnePlusNPack3Bits
#[inline(always)]
fn pop_all_but_one_plus_n_pack3_bits(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.pop(fp.position);
    fp.data[fp.position] += br.read_ubitlong(3)? as i32 + 1;
    Ok(())
}

// PopAllButOnePlusNPack6Bits
#[inline(always)]
fn pop_all_but_one_plus_n_pack6_bits(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.pop(fp.position);
    fp.data[fp.position] += br.read_ubitlong(6)? as i32 + 1;
    Ok(())
}

// PopNPlusOne
#[inline(always)]
fn pop_n_plus_one(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.pop(br.read_ubitvarfp()? as usize);
    fp.data[fp.position] += 1;
    Ok(())
}

// PopNPlusN
#[inline(always)]
fn pop_n_plus_n(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.pop(br.read_ubitvarfp()? as usize);
    fp.data[fp.position] += br.read_varint32()?;
    Ok(())
}

// PopNAndNonTopographical
#[inline(always)]
fn pop_n_and_non_topographical(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.pop(br.read_ubitvarfp()? as usize);
    for i in 0..=fp.position {
        if br.read_bool()? {
            fp.data[i] += br.read_varint32()?;
        }
    }
    Ok(())
}

// NonTopoComplex
#[inline(always)]
fn non_topo_complex(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    for i in 0..=fp.position {
        if br.read_bool()? {
            fp.data[i] += br.read_varint32()?;
        }
    }
    Ok(())
}

// NonTopoPenultimatePlusOne
#[inline(always)]
fn non_topo_penultimate_plus_one(fp: &mut FieldPath, _br: &mut BitReader) -> Result<()> {
    fp.data[fp.position - 1] += 1;
    Ok(())
}

// NonTopoComplexPack4Bits
#[inline(always)]
fn non_topo_complex_pack4_bits(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    for i in 0..=fp.position {
        if br.read_bool()? {
            fp.data[i] += br.read_ubitlong(4)? as i32 - 7;
        }
    }
    Ok(())
}

// FieldPathEncodeFinish
#[inline(always)]
fn field_path_encode_finish(fp: &mut FieldPath, _br: &mut BitReader) -> Result<()> {
    fp.finished = true;
    Ok(())
}

// ----

type FieldPathOp = fn(&mut FieldPath, &mut BitReader) -> Result<()>;

#[inline(always)]
fn lookup_op(id: u32) -> Option<FieldPathOp> {
    // stolen from butterfly.
    // those ids are result of encoding sequence of bools into numeric
    // representation like so (in a loop): id = ( id << 1 ) | br.readBool().
    match id {
        0 => Some(plus_one),
        2 => Some(field_path_encode_finish),
        14 => Some(plus_two),
        15 => Some(push_one_left_delta_n_right_non_zero_pack6_bits),
        24 => Some(push_one_left_delta_one_right_non_zero),
        26 => Some(plus_n),
        50 => Some(plus_three),
        51 => Some(pop_all_but_one_plus_one),
        217 => Some(push_one_left_delta_n_right_non_zero),
        218 => Some(push_one_left_delta_one_right_zero),
        220 => Some(push_one_left_delta_n_right_zero),
        222 => Some(pop_all_but_one_plus_n_pack6_bits),
        223 => Some(plus_four),
        432 => Some(pop_all_but_one_plus_n),
        438 => Some(push_one_left_delta_n_right_non_zero_pack8_bits),
        439 => Some(non_topo_penultimate_plus_one),
        442 => Some(pop_all_but_one_plus_n_pack3_bits),
        443 => Some(push_n_and_non_topological),
        866 => Some(non_topo_complex_pack4_bits),
        1735 => Some(non_topo_complex),
        3469 => Some(push_one_left_delta_zero_right_zero),
        27745 => Some(pop_one_plus_one),
        27749 => Some(push_one_left_delta_zero_right_non_zero),
        55488 => Some(pop_n_and_non_topographical),
        55489 => Some(pop_n_plus_n),
        55492 => Some(push_n),
        55493 => Some(push_three_pack5_left_delta_n),
        55494 => Some(pop_n_plus_one),
        55495 => Some(pop_one_plus_n),
        55496 => Some(push_two_left_delta_zero),
        110994 => Some(push_three_left_delta_zero),
        110995 => Some(push_two_pack5_left_delta_zero),
        111000 => Some(push_two_left_delta_n),
        111001 => Some(push_three_pack5_left_delta_one),
        111002 => Some(push_three_left_delta_n),
        111003 => Some(push_two_pack5_left_delta_n),
        111004 => Some(push_two_left_delta_one),
        111005 => Some(push_three_pack5_left_delta_zero),
        111006 => Some(push_three_left_delta_one),
        111007 => Some(push_two_pack5_left_delta_one),
        _ => None,
    }
}

pub fn read_field_paths_in<A: Allocator>(
    br: &mut BitReader,
    alloc: A,
) -> Result<Vec<FieldPath, A>> {
    // TODO: create field path object pool
    let mut fp = FieldPath::new();
    let mut fps = Vec::new_in(alloc);

    loop {
        // stolen from butterfly.
        let mut id = 0;
        let mut op: Option<FieldPathOp> = None;

        // 17 is max depth of huffman tree I assume (didn't check).
        for _ in 0..17 {
            id = (id << 1) | (br.read_bool()? as u32);
            op = lookup_op(id);
            if op.is_some() {
                break;
            }
        }

        let op = op.expect("exhausted max operation bits");
        op(&mut fp, br)?;

        if fp.finished {
            break;
        }

        fps.push(fp.clone());
    }

    Ok(fps)
}
