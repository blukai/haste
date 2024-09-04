use crate::bitreader::BitReader;
use std::cell::UnsafeCell;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // mod
    #[error("exhausted max operation bits")]
    ExhaustedMaxOpBits,
}

pub type Result<T> = std::result::Result<T, Error>;

// TODO: try clarity's field path format (encode all components into u64); see
// src/main/java/skadistats/clarity/model/s2/S2LongFieldPathFormat.java for
// implementation and
// https://github.com/skadistats/clarity/commit/212eaddf7dc8b716c22faaec37952236f521a804#commitcomment-86037653
// for a technical exmplanation.

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

    // ----

    #[inline(always)]
    pub(crate) unsafe fn get_unchecked(&self, index: usize) -> usize {
        *self.data.get_unchecked(index) as usize
    }

    // ----
    // public api

    // NOTE: using this method can hurt performance when used in critical code
    // paths. use the unsafe [`Self::get_unchecked`] instead.
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

    // ----

    // PlusOne
    #[inline(always)]
    fn plus_one(&mut self, _br: &mut BitReader) -> Result<()> {
        self.inc_last(1);
        Ok(())
    }

    // PlusTwo
    #[inline(always)]
    fn plus_two(&mut self, _br: &mut BitReader) -> Result<()> {
        self.inc_last(2);
        Ok(())
    }

    // PlusThree
    #[inline(always)]
    fn plus_three(&mut self, _br: &mut BitReader) -> Result<()> {
        self.inc_last(3);
        Ok(())
    }

    // PlusFour
    #[inline(always)]
    fn plus_four(&mut self, _br: &mut BitReader) -> Result<()> {
        self.inc_last(4);
        Ok(())
    }

    // PlusN
    #[inline(always)]
    fn plus_n(&mut self, br: &mut BitReader) -> Result<()> {
        self.inc_last(br.read_ubitvarfp() as i32 + 5);
        Ok(())
    }

    // PushOneLeftDeltaZeroRightZero
    #[inline(always)]
    fn push_one_left_delta_zero_right_zero(&mut self, _br: &mut BitReader) -> Result<()> {
        self.push(0);
        Ok(())
    }

    // PushOneLeftDeltaZeroRightNonZero
    #[inline(always)]
    fn push_one_left_delta_zero_right_non_zero(&mut self, br: &mut BitReader) -> Result<()> {
        self.push(br.read_ubitvarfp() as i32);
        Ok(())
    }

    // PushOneLeftDeltaOneRightZero
    #[inline(always)]
    fn push_one_left_delta_one_right_zero(&mut self, _br: &mut BitReader) -> Result<()> {
        self.inc_last(1);
        self.push(0);
        Ok(())
    }

    // PushOneLeftDeltaOneRightNonZero
    #[inline(always)]
    fn push_one_left_delta_one_right_non_zero(&mut self, br: &mut BitReader) -> Result<()> {
        self.inc_last(1);
        self.push(br.read_ubitvarfp() as i32);
        Ok(())
    }

    // PushOneLeftDeltaNRightZero
    #[inline(always)]
    fn push_one_left_delta_n_right_zero(&mut self, br: &mut BitReader) -> Result<()> {
        self.inc_last(br.read_ubitvarfp() as i32);
        self.push(0);
        Ok(())
    }

    // PushOneLeftDeltaNRightNonZero
    #[inline(always)]
    fn push_one_left_delta_n_right_non_zero(&mut self, br: &mut BitReader) -> Result<()> {
        self.inc_last(br.read_ubitvarfp() as i32 + 2);
        self.push(br.read_ubitvarfp() as i32 + 1);
        Ok(())
    }

    // PushOneLeftDeltaNRightNonZeroPack6Bits
    #[inline(always)]
    fn push_one_left_delta_n_right_non_zero_pack6_bits(
        &mut self,
        br: &mut BitReader,
    ) -> Result<()> {
        self.inc_last(br.read_ubit64(3) as i32 + 2);
        self.push(br.read_ubit64(3) as i32 + 1);
        Ok(())
    }

    // PushOneLeftDeltaNRightNonZeroPack8Bits
    #[inline(always)]
    fn push_one_left_delta_n_right_non_zero_pack8_bits(
        &mut self,
        br: &mut BitReader,
    ) -> Result<()> {
        self.inc_last(br.read_ubit64(4) as i32 + 2);
        self.push(br.read_ubit64(4) as i32 + 1);
        Ok(())
    }

    // PushTwoLeftDeltaZero
    #[inline(always)]
    fn push_two_left_delta_zero(&mut self, br: &mut BitReader) -> Result<()> {
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
        Ok(())
    }

    // PushTwoLeftDeltaOne
    #[inline(always)]
    fn push_two_left_delta_one(&mut self, br: &mut BitReader) -> Result<()> {
        self.inc_last(1);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
        Ok(())
    }

    // PushTwoLeftDeltaN
    #[inline(always)]
    fn push_two_left_delta_n(&mut self, br: &mut BitReader) -> Result<()> {
        self.inc_last(br.read_ubitvar() as i32 + 2);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
        Ok(())
    }

    // PushTwoPack5LeftDeltaZero
    #[inline(always)]
    fn push_two_pack5_left_delta_zero(&mut self, br: &mut BitReader) -> Result<()> {
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
        Ok(())
    }

    // PushTwoPack5LeftDeltaOne
    #[inline(always)]
    fn push_two_pack5_left_delta_one(&mut self, br: &mut BitReader) -> Result<()> {
        self.inc_last(1);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
        Ok(())
    }

    // PushTwoPack5LeftDeltaN
    #[inline(always)]
    fn push_two_pack5_left_delta_n(&mut self, br: &mut BitReader) -> Result<()> {
        self.inc_last(br.read_ubitvar() as i32 + 2);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
        Ok(())
    }

    // PushThreeLeftDeltaZero
    #[inline(always)]
    fn push_three_left_delta_zero(&mut self, br: &mut BitReader) -> Result<()> {
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
        Ok(())
    }

    // PushThreeLeftDeltaOne
    #[inline(always)]
    fn push_three_left_delta_one(&mut self, br: &mut BitReader) -> Result<()> {
        self.inc_last(1);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
        Ok(())
    }

    // PushThreeLeftDeltaN
    #[inline(always)]
    fn push_three_left_delta_n(&mut self, br: &mut BitReader) -> Result<()> {
        self.inc_last(br.read_ubitvar() as i32 + 2);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
        Ok(())
    }

    // PushThreePack5LeftDeltaZero
    #[inline(always)]
    fn push_three_pack5_left_delta_zero(&mut self, br: &mut BitReader) -> Result<()> {
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
        Ok(())
    }

    // PushThreePack5LeftDeltaOne
    #[inline(always)]
    fn push_three_pack5_left_delta_one(&mut self, br: &mut BitReader) -> Result<()> {
        self.inc_last(1);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
        Ok(())
    }

    // PushThreePack5LeftDeltaN
    #[inline(always)]
    fn push_three_pack5_left_delta_n(&mut self, br: &mut BitReader) -> Result<()> {
        self.inc_last(br.read_ubitvar() as i32 + 2);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
        Ok(())
    }

    // PushN
    #[inline(always)]
    fn push_n(&mut self, br: &mut BitReader) -> Result<()> {
        let n = br.read_ubitvar() as usize;
        self.inc_last(br.read_ubitvar() as i32);
        for _ in 0..n {
            self.push(br.read_ubitvarfp() as i32);
        }
        Ok(())
    }

    // PushNAndNonTopographical
    #[inline(always)]
    fn push_n_and_non_topographical(&mut self, br: &mut BitReader) -> Result<()> {
        for i in 0..=self.last {
            if br.read_bool() {
                self.inc_at(i, br.read_varint32() + 1);
            }
        }
        let n = br.read_ubitvar() as usize;
        for _ in 0..n {
            self.push(br.read_ubitvarfp() as i32);
        }
        Ok(())
    }

    // PopOnePlusOne
    #[inline(always)]
    fn pop_one_plus_one(&mut self, _br: &mut BitReader) -> Result<()> {
        self.pop(1);
        self.inc_last(1);
        Ok(())
    }

    // PopOnePlusN
    #[inline(always)]
    fn pop_one_plus_n(&mut self, br: &mut BitReader) -> Result<()> {
        self.pop(1);
        self.inc_last(br.read_ubitvarfp() as i32 + 1);
        Ok(())
    }

    // PopAllButOnePlusOne
    #[inline(always)]
    fn pop_all_but_one_plus_one(&mut self, _br: &mut BitReader) -> Result<()> {
        self.pop(self.last);
        self.inc_last(1);
        Ok(())
    }

    // PopAllButOnePlusN
    #[inline(always)]
    fn pop_all_but_one_plus_n(&mut self, br: &mut BitReader) -> Result<()> {
        self.pop(self.last);
        self.inc_last(br.read_ubitvarfp() as i32 + 1);
        Ok(())
    }

    // PopAllButOnePlusNPack3Bits
    #[inline(always)]
    fn pop_all_but_one_plus_n_pack3_bits(&mut self, br: &mut BitReader) -> Result<()> {
        self.pop(self.last);
        self.inc_last(br.read_ubit64(3) as i32 + 1);
        Ok(())
    }

    // PopAllButOnePlusNPack6Bits
    #[inline(always)]
    fn pop_all_but_one_plus_n_pack6_bits(&mut self, br: &mut BitReader) -> Result<()> {
        self.pop(self.last);
        self.inc_last(br.read_ubit64(6) as i32 + 1);
        Ok(())
    }

    // PopNPlusOne
    #[inline(always)]
    fn pop_n_plus_one(&mut self, br: &mut BitReader) -> Result<()> {
        self.pop(br.read_ubitvarfp() as usize);
        self.inc_last(1);
        Ok(())
    }

    // PopNPlusN
    #[inline(always)]
    fn pop_n_plus_n(&mut self, br: &mut BitReader) -> Result<()> {
        self.pop(br.read_ubitvarfp() as usize);
        self.inc_last(br.read_varint32());
        Ok(())
    }

    // PopNAndNonTopographical
    #[inline(always)]
    fn pop_n_and_non_topographical(&mut self, br: &mut BitReader) -> Result<()> {
        self.pop(br.read_ubitvarfp() as usize);
        for i in 0..=self.last {
            if br.read_bool() {
                self.inc_at(i, br.read_varint32());
            }
        }
        Ok(())
    }

    // NonTopoComplex
    #[inline(always)]
    fn non_topo_complex(&mut self, br: &mut BitReader) -> Result<()> {
        for i in 0..=self.last {
            if br.read_bool() {
                self.inc_at(i, br.read_varint32());
            }
        }
        Ok(())
    }

    // NonTopoPenultimatePluseOne
    #[inline(always)]
    fn non_topo_penultimate_pluse_one(&mut self, _br: &mut BitReader) -> Result<()> {
        self.inc_at(self.last - 1, 1);
        Ok(())
    }

    // NonTopoComplexPack4Bits
    #[inline(always)]
    fn non_topo_complex_pack4_bits(&mut self, br: &mut BitReader) -> Result<()> {
        for i in 0..=self.last {
            if br.read_bool() {
                self.inc_at(i, br.read_ubit64(4) as i32 - 7);
            }
        }
        Ok(())
    }

    // FieldPathEncodeFinish
    #[inline(always)]
    fn field_path_encode_finish(&mut self, _br: &mut BitReader) -> Result<()> {
        self.finished = true;
        Ok(())
    }

    // ----

    #[inline(always)]
    fn exec_op(&mut self, id: u32, br: &mut BitReader) -> Result<bool> {
        // stolen from butterfly. those ids are result of encoding sequence of
        // bools into numeric representation like so (in a loop): id = ( id << 1
        // ) | br.readBool().
        //
        // TODO: don't questionmark all the ops, instead combine op result with
        // the final "true" return.
        //
        // TODO: too many branch misses happen here. $ perf record -e
        // branch-misses ./target/release/emptybench
        // fixtures/7116662198_1379602574.dem $ perf report
        //
        // TODO: try to optimize by walking hufflam tree. aparantely some
        // compression algos use it (or similar techniques) because it allows to
        // reduce branch misses by providing hierarchical structure that allows
        // making decisions based on variable values.
        match id {
            0 => self.plus_one(br)?,
            2 => self.field_path_encode_finish(br)?,
            14 => self.plus_two(br)?,
            15 => self.push_one_left_delta_n_right_non_zero_pack6_bits(br)?,
            24 => self.push_one_left_delta_one_right_non_zero(br)?,
            26 => self.plus_n(br)?,
            50 => self.plus_three(br)?,
            51 => self.pop_all_but_one_plus_one(br)?,
            217 => self.push_one_left_delta_n_right_non_zero(br)?,
            218 => self.push_one_left_delta_one_right_zero(br)?,
            220 => self.push_one_left_delta_n_right_zero(br)?,
            222 => self.pop_all_but_one_plus_n_pack6_bits(br)?,
            223 => self.plus_four(br)?,
            432 => self.pop_all_but_one_plus_n(br)?,
            438 => self.push_one_left_delta_n_right_non_zero_pack8_bits(br)?,
            439 => self.non_topo_penultimate_pluse_one(br)?,
            442 => self.pop_all_but_one_plus_n_pack3_bits(br)?,
            443 => self.push_n_and_non_topographical(br)?,
            866 => self.non_topo_complex_pack4_bits(br)?,
            1735 => self.non_topo_complex(br)?,
            3469 => self.push_one_left_delta_zero_right_zero(br)?,
            27745 => self.pop_one_plus_one(br)?,
            27749 => self.push_one_left_delta_zero_right_non_zero(br)?,
            55488 => self.pop_n_and_non_topographical(br)?,
            55489 => self.pop_n_plus_n(br)?,
            55492 => self.push_n(br)?,
            55493 => self.push_three_pack5_left_delta_n(br)?,
            55494 => self.pop_n_plus_one(br)?,
            55495 => self.pop_one_plus_n(br)?,
            55496 => self.push_two_left_delta_zero(br)?,
            110994 => self.push_three_left_delta_zero(br)?,
            110995 => self.push_two_pack5_left_delta_zero(br)?,
            111000 => self.push_two_left_delta_n(br)?,
            111001 => self.push_three_pack5_left_delta_one(br)?,
            111002 => self.push_three_left_delta_n(br)?,
            111003 => self.push_two_pack5_left_delta_n(br)?,
            111004 => self.push_two_left_delta_one(br)?,
            111005 => self.push_three_pack5_left_delta_zero(br)?,
            111006 => self.push_three_left_delta_one(br)?,
            111007 => self.push_two_pack5_left_delta_one(br)?,
            _ => return Ok(false),
        };

        Ok(true)
    }
}

// ----

thread_local! {
    // NOTE: 4096 is an arbitrary value that is large enough that that came out
    // of printing out count of fps collected per "run". (sort -nr can be handy)
    //
    // NOTE: swapping RefCell to UnsafeCell doesn't seem to make any difference
    // (with `let fps = unsafe { &mut *fps.get() }` inside of entities's parse)
    pub(crate) static FIELD_PATHS: UnsafeCell<Vec<FieldPath>> = {
        const SIZE: usize = 4096;
        let mut v = Vec::with_capacity(SIZE);
        unsafe { v.set_len(SIZE) };
        UnsafeCell::new(v)
    };
}

// NOTE: majority of field path reads are shorter then 32 (but some are beyond
// thousand).
//
// println the len, and then pipe into sort -n | uniq -c.
pub(crate) fn read_field_paths<'a>(
    br: &mut BitReader,
    fps: &'a mut [FieldPath],
) -> Result<&'a mut [FieldPath]> {
    let mut fp = FieldPath::default();
    let mut i: usize = 0;
    'outer_loop: loop {
        let mut id = 0;
        // stolen from butterfly;
        // 17 is the depth of the huffman tree.
        for _ in 0..17 {
            // true is right, false is left
            id = (id << 1) | (br.read_bool() as u32);
            if !fp.exec_op(id, br)? {
                continue;
            }
            if fp.finished {
                return Ok(&mut fps[..i as usize]);
            }
            fps[i as usize] = fp.clone();
            i += 1;
            continue 'outer_loop;
        }
        return Err(Error::ExhaustedMaxOpBits);
    }
}
