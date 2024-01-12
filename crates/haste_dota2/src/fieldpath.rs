use crate::{
    bitbuf::{self, BitReader},
    fxhash,
};
use std::cell::RefCell;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // crate
    #[error(transparent)]
    BitBuf(#[from] bitbuf::Error),
    // mod
    #[error("exhausted max operation bits")]
    ExhaustedMaxOpBits,
}

pub type Result<T> = std::result::Result<T, Error>;

pub const FIELD_PATH_DATA_SIZE: usize = 7;

// TODO: FieldPath should be just data with hash method; current field path
// should become FieldPathReader or decoder or something like that..

#[derive(Debug, Clone)]
pub struct FieldPath {
    pub data: [i32; FIELD_PATH_DATA_SIZE],
    pub position: usize,
    pub finished: bool,
}

impl Default for FieldPath {
    #[inline(always)]
    fn default() -> Self {
        Self {
            data: [-1, 0, 0, 0, 0, 0, 0],
            position: 0,
            finished: false,
        }
    }
}

impl FieldPath {
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
        unsafe { *self.data.get_unchecked(index) as usize }
    }

    // SAFETY: hash_unchecked is safe if replay data is correct. all items of
    // data array never go below 0 or beyond 255.
    #[inline(always)]
    pub unsafe fn hash_unchecked(&self) -> u64 {
        let slice: &[u32] = std::mem::transmute(&self.data[..=self.position]);
        fxhash::hash_u32(slice)
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
#[cold]
fn push_two_left_delta_one(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 1;
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushTwoLeftDeltaN
#[cold]
fn push_two_left_delta_n(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += br.read_ubitvar()? as i32 + 2;
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushTwoPack5LeftDeltaZero
#[cold]
fn push_two_pack5_left_delta_zero(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    Ok(())
}

// PushTwoPack5LeftDeltaOne
#[cold]
fn push_two_pack5_left_delta_one(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 1;
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    Ok(())
}

// PushTwoPack5LeftDeltaN
#[cold]
fn push_two_pack5_left_delta_n(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += br.read_ubitvar()? as i32 + 2;
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    Ok(())
}

// PushThreeLeftDeltaZero
#[cold]
fn push_three_left_delta_zero(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushThreeLeftDeltaOne
#[cold]
fn push_three_left_delta_one(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += 1;
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushThreeLeftDeltaN
#[cold]
fn push_three_left_delta_n(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.data[fp.position] += br.read_ubitvar()? as i32 + 2;
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    fp.push_back(br.read_ubitvarfp()? as i32);
    Ok(())
}

// PushThreePack5LeftDeltaZero
#[cold]
fn push_three_pack5_left_delta_zero(fp: &mut FieldPath, br: &mut BitReader) -> Result<()> {
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    fp.push_back(br.read_ubitlong(5)? as i32);
    Ok(())
}

// PushThreePack5LeftDeltaOne
#[cold]
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
    // stolen from butterfly. those ids are result of encoding sequence of bools
    // into numeric representation like so (in a loop): id = ( id << 1 ) |
    // br.readBool().
    //
    // TODO: don't questionmark all the ops, instead combine op result with the
    // final "true" return.
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
        // NOTE: functions that are down below are marked as cold
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

thread_local! {
    // NOTE: 4096 is an arbitrary value that is large enough that that came out
    // of printing out count of fps collected per "run". (sort -nr can be handy)
    //
    // NOTE: swapping RefCell to UnsafeCell doesn't seem to make any difference
    // (with `let fps = unsafe { &mut *fps.get() }` inside of entities's parse)
    pub(crate) static FIELD_PATHS: RefCell<Vec<FieldPath>> = {
        const SIZE: usize = 4096;
        let mut v = Vec::with_capacity(SIZE);
        unsafe { v.set_len(SIZE) };
        RefCell::new(v)
    };
}

pub(crate) fn read_field_paths<'a>(
    br: &mut BitReader,
    fps: &'a mut [FieldPath],
) -> Result<&'a [FieldPath]> {
    let mut fp = FieldPath::default();
    let mut i: isize = -1;
    'epic_loop: loop {
        i += 1;
        let mut id = 0;
        // stolen from butterfly;
        // 17 is the depth of the huffman tree.
        for _ in 0..17 {
            // true is right, false is left
            id = (id << 1) | (br.read_bool()? as u32);
            if !lookup_exec_op(id, &mut fp, br)? {
                continue;
            }
            if fp.finished {
                return Ok(&fps[..i as usize]);
            }
            fps[i as usize] = fp.clone();
            continue 'epic_loop;
        }

        return Err(Error::ExhaustedMaxOpBits);
    }
}
