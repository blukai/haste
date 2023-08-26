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
    #[inline(always)]
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

    #[inline(always)]
    pub fn get(&self, index: usize) -> usize {
        self.data[index] as usize
    }
}

// ----

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

#[inline(always)]
fn lookup_exec_op(id: u32, fp: &mut FieldPath, br: &mut BitReader) -> Result<bool> {
    // stolen from butterfly.
    // those ids are result of encoding sequence of bools into numeric
    // representation like so (in a loop): id = ( id << 1 ) | br.readBool().
    match id {
        0 => plus_one(fp, br)?,
        2 => field_path_encode_finish(fp, br)?,
        14 => plus_two(fp, br)?,
        15 => push_one_left_delta_n_right_non_zero_pack6_bits(fp, br)?,
        24 => push_one_left_delta_one_right_non_zero(fp, br)?,
        26 => plus_n(fp, br)?,
        50 => plus_three(fp, br)?,
        51 => pop_all_but_one_plus_one(fp, br)?,
        217 => push_one_left_delta_n_right_non_zero(fp, br)?,
        218 => push_one_left_delta_one_right_zero(fp, br)?,
        220 => push_one_left_delta_n_right_zero(fp, br)?,
        222 => pop_all_but_one_plus_n_pack6_bits(fp, br)?,
        223 => plus_four(fp, br)?,
        432 => pop_all_but_one_plus_n(fp, br)?,
        438 => push_one_left_delta_n_right_non_zero_pack8_bits(fp, br)?,
        439 => non_topo_penultimate_plus_one(fp, br)?,
        442 => pop_all_but_one_plus_n_pack3_bits(fp, br)?,
        443 => push_n_and_non_topological(fp, br)?,
        866 => non_topo_complex_pack4_bits(fp, br)?,
        1735 => non_topo_complex(fp, br)?,
        3469 => push_one_left_delta_zero_right_zero(fp, br)?,
        27745 => pop_one_plus_one(fp, br)?,
        27749 => push_one_left_delta_zero_right_non_zero(fp, br)?,
        55488 => pop_n_and_non_topographical(fp, br)?,
        55489 => pop_n_plus_n(fp, br)?,
        55492 => push_n(fp, br)?,
        55493 => push_three_pack5_left_delta_n(fp, br)?,
        55494 => pop_n_plus_one(fp, br)?,
        55495 => pop_one_plus_n(fp, br)?,
        55496 => push_two_left_delta_zero(fp, br)?,
        110994 => push_three_left_delta_zero(fp, br)?,
        110995 => push_two_pack5_left_delta_zero(fp, br)?,
        111000 => push_two_left_delta_n(fp, br)?,
        111001 => push_three_pack5_left_delta_one(fp, br)?,
        111002 => push_three_left_delta_n(fp, br)?,
        111003 => push_two_pack5_left_delta_n(fp, br)?,
        111004 => push_two_left_delta_one(fp, br)?,
        111005 => push_three_pack5_left_delta_zero(fp, br)?,
        111006 => push_three_left_delta_one(fp, br)?,
        111007 => push_two_pack5_left_delta_one(fp, br)?,
        _ => return Ok(false),
    };
    Ok(true)
}

pub fn read_field_paths_in<A: Allocator>(
    br: &mut BitReader,
    alloc: A,
) -> Result<Vec<FieldPath, A>> {
    let mut fp = FieldPath::new();
    // NOTE: 10 is just an arbitrary value that performs better then not
    // specifying capacity or specifying larger capacity (eg. 20); it's based on
    // frequency of fps.len();
    //
    // sort out.txt | uniq -c | sort -nr
    let mut fps = Vec::with_capacity_in(10, alloc);
    'epic_loop: loop {
        // stolen from butterfly
        let mut id = 0;
        // 17 is max depth of huffman tree I assume (didn't check)
        for _ in 0..17 {
            id = (id << 1) | (br.read_bool()? as u32);
            if !lookup_exec_op(id, &mut fp, br)? {
                continue;
            }
            if fp.finished {
                return Ok(fps);
            }
            fps.push(fp.clone());
            continue 'epic_loop;
        }

        // TODO: don't panic
        panic!("exhausted max operation bits");
    }
}
