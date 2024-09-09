use crate::bitreader::BitReader;

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
}

impl Default for FieldPath {
    #[inline(always)]
    fn default() -> Self {
        Self {
            data: [255, 0, 0, 0, 0, 0, 0],
            last: 0,
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
    fn plus_one(&mut self, _br: &mut BitReader) {
        self.inc_last(1);
    }

    // PlusTwo
    #[inline(always)]
    fn plus_two(&mut self, _br: &mut BitReader) {
        self.inc_last(2);
    }

    // PlusThree
    #[inline(always)]
    fn plus_three(&mut self, _br: &mut BitReader) {
        self.inc_last(3);
    }

    // PlusFour
    #[inline(always)]
    fn plus_four(&mut self, _br: &mut BitReader) {
        self.inc_last(4);
    }

    // PlusN
    #[inline(always)]
    fn plus_n(&mut self, br: &mut BitReader) {
        self.inc_last(br.read_ubitvarfp() as i32 + 5);
    }

    // PushOneLeftDeltaZeroRightZero
    #[inline(always)]
    fn push_one_left_delta_zero_right_zero(&mut self, _br: &mut BitReader) {
        self.push(0);
    }

    // PushOneLeftDeltaZeroRightNonZero
    #[inline(always)]
    fn push_one_left_delta_zero_right_non_zero(&mut self, br: &mut BitReader) {
        self.push(br.read_ubitvarfp() as i32);
    }

    // PushOneLeftDeltaOneRightZero
    #[inline(always)]
    fn push_one_left_delta_one_right_zero(&mut self, _br: &mut BitReader) {
        self.inc_last(1);
        self.push(0);
    }

    // PushOneLeftDeltaOneRightNonZero
    #[inline(always)]
    fn push_one_left_delta_one_right_non_zero(&mut self, br: &mut BitReader) {
        self.inc_last(1);
        self.push(br.read_ubitvarfp() as i32);
    }

    // PushOneLeftDeltaNRightZero
    #[inline(always)]
    fn push_one_left_delta_n_right_zero(&mut self, br: &mut BitReader) {
        self.inc_last(br.read_ubitvarfp() as i32);
        self.push(0);
    }

    // PushOneLeftDeltaNRightNonZero
    #[inline(always)]
    fn push_one_left_delta_n_right_non_zero(&mut self, br: &mut BitReader) {
        self.inc_last(br.read_ubitvarfp() as i32 + 2);
        self.push(br.read_ubitvarfp() as i32 + 1);
    }

    // PushOneLeftDeltaNRightNonZeroPack6Bits
    #[inline(always)]
    fn push_one_left_delta_n_right_non_zero_pack6_bits(&mut self, br: &mut BitReader) {
        self.inc_last(br.read_ubit64(3) as i32 + 2);
        self.push(br.read_ubit64(3) as i32 + 1);
    }

    // PushOneLeftDeltaNRightNonZeroPack8Bits
    #[inline(always)]
    fn push_one_left_delta_n_right_non_zero_pack8_bits(&mut self, br: &mut BitReader) {
        self.inc_last(br.read_ubit64(4) as i32 + 2);
        self.push(br.read_ubit64(4) as i32 + 1);
    }

    // PushTwoLeftDeltaZero
    #[inline(always)]
    fn push_two_left_delta_zero(&mut self, br: &mut BitReader) {
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
    }

    // PushTwoLeftDeltaOne
    #[inline(always)]
    fn push_two_left_delta_one(&mut self, br: &mut BitReader) {
        self.inc_last(1);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
    }

    // PushTwoLeftDeltaN
    #[inline(always)]
    fn push_two_left_delta_n(&mut self, br: &mut BitReader) {
        self.inc_last(br.read_ubitvar() as i32 + 2);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
    }

    // PushTwoPack5LeftDeltaZero
    #[inline(always)]
    fn push_two_pack5_left_delta_zero(&mut self, br: &mut BitReader) {
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
    }

    // PushTwoPack5LeftDeltaOne
    #[inline(always)]
    fn push_two_pack5_left_delta_one(&mut self, br: &mut BitReader) {
        self.inc_last(1);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
    }

    // PushTwoPack5LeftDeltaN
    #[inline(always)]
    fn push_two_pack5_left_delta_n(&mut self, br: &mut BitReader) {
        self.inc_last(br.read_ubitvar() as i32 + 2);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
    }

    // PushThreeLeftDeltaZero
    #[inline(always)]
    fn push_three_left_delta_zero(&mut self, br: &mut BitReader) {
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
    }

    // PushThreeLeftDeltaOne
    #[inline(always)]
    fn push_three_left_delta_one(&mut self, br: &mut BitReader) {
        self.inc_last(1);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
    }

    // PushThreeLeftDeltaN
    #[inline(always)]
    fn push_three_left_delta_n(&mut self, br: &mut BitReader) {
        self.inc_last(br.read_ubitvar() as i32 + 2);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
        self.push(br.read_ubitvarfp() as i32);
    }

    // PushThreePack5LeftDeltaZero
    #[inline(always)]
    fn push_three_pack5_left_delta_zero(&mut self, br: &mut BitReader) {
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
    }

    // PushThreePack5LeftDeltaOne
    #[inline(always)]
    fn push_three_pack5_left_delta_one(&mut self, br: &mut BitReader) {
        self.inc_last(1);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
    }

    // PushThreePack5LeftDeltaN
    #[inline(always)]
    fn push_three_pack5_left_delta_n(&mut self, br: &mut BitReader) {
        self.inc_last(br.read_ubitvar() as i32 + 2);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
        self.push(br.read_ubit64(5) as i32);
    }

    // PushN
    #[inline(always)]
    fn push_n(&mut self, br: &mut BitReader) {
        let n = br.read_ubitvar() as usize;
        self.inc_last(br.read_ubitvar() as i32);
        for _ in 0..n {
            self.push(br.read_ubitvarfp() as i32);
        }
    }

    // PushNAndNonTopographical
    #[inline(always)]
    fn push_n_and_non_topographical(&mut self, br: &mut BitReader) {
        for i in 0..=self.last {
            if br.read_bool() {
                self.inc_at(i, br.read_varint32() + 1);
            }
        }
        let n = br.read_ubitvar() as usize;
        for _ in 0..n {
            self.push(br.read_ubitvarfp() as i32);
        }
    }

    // PopOnePlusOne
    #[inline(always)]
    fn pop_one_plus_one(&mut self, _br: &mut BitReader) {
        self.pop(1);
        self.inc_last(1);
    }

    // PopOnePlusN
    #[inline(always)]
    fn pop_one_plus_n(&mut self, br: &mut BitReader) {
        self.pop(1);
        self.inc_last(br.read_ubitvarfp() as i32 + 1);
    }

    // PopAllButOnePlusOne
    #[inline(always)]
    fn pop_all_but_one_plus_one(&mut self, _br: &mut BitReader) {
        self.pop(self.last);
        self.inc_last(1);
    }

    // PopAllButOnePlusN
    #[inline(always)]
    fn pop_all_but_one_plus_n(&mut self, br: &mut BitReader) {
        self.pop(self.last);
        self.inc_last(br.read_ubitvarfp() as i32 + 1);
    }

    // PopAllButOnePlusNPack3Bits
    #[inline(always)]
    fn pop_all_but_one_plus_n_pack3_bits(&mut self, br: &mut BitReader) {
        self.pop(self.last);
        self.inc_last(br.read_ubit64(3) as i32 + 1);
    }

    // PopAllButOnePlusNPack6Bits
    #[inline(always)]
    fn pop_all_but_one_plus_n_pack6_bits(&mut self, br: &mut BitReader) {
        self.pop(self.last);
        self.inc_last(br.read_ubit64(6) as i32 + 1);
    }

    // PopNPlusOne
    #[inline(always)]
    fn pop_n_plus_one(&mut self, br: &mut BitReader) {
        self.pop(br.read_ubitvarfp() as usize);
        self.inc_last(1);
    }

    // PopNPlusN
    #[inline(always)]
    fn pop_n_plus_n(&mut self, br: &mut BitReader) {
        self.pop(br.read_ubitvarfp() as usize);
        self.inc_last(br.read_varint32());
    }

    // PopNAndNonTopographical
    #[inline(always)]
    fn pop_n_and_non_topographical(&mut self, br: &mut BitReader) {
        self.pop(br.read_ubitvarfp() as usize);
        for i in 0..=self.last {
            if br.read_bool() {
                self.inc_at(i, br.read_varint32());
            }
        }
    }

    // NonTopoComplex
    #[inline(always)]
    fn non_topo_complex(&mut self, br: &mut BitReader) {
        for i in 0..=self.last {
            if br.read_bool() {
                self.inc_at(i, br.read_varint32());
            }
        }
    }

    // NonTopoPenultimatePluseOne
    #[inline(always)]
    fn non_topo_penultimate_pluse_one(&mut self, _br: &mut BitReader) {
        self.inc_at(self.last - 1, 1);
    }

    // NonTopoComplexPack4Bits
    #[inline(always)]
    fn non_topo_complex_pack4_bits(&mut self, br: &mut BitReader) {
        for i in 0..=self.last {
            if br.read_bool() {
                self.inc_at(i, br.read_ubit64(4) as i32 - 7);
            }
        }
    }

    // FieldPathEncodeFinish
    #[inline(always)]
    fn field_path_encode_finish(&mut self, _br: &mut BitReader) {}

    // ----

    #[inline(always)]
    fn exec_op(&mut self, id: u32, br: &mut BitReader) -> bool {
        // stolen from butterfly. those ids are result of encoding sequence of bools into numeric
        // representation like so (in a loop):
        // id = ( id << 1 ) | br.readBool().
        //
        // TODO: too many branch misses happen here.
        // $ perf record -e branch-misses ./target/release/emptybench fixtures/7116662198_1379602574.dem
        // $ perf report
        //
        // TODO: try to optimize by walking hufflam tree. aparantely some compression algos use it
        // (or similar techniques) because it allows to reduce branch misses by providing
        // hierarchical structure that allows making decisions based on variable values.

        match id {
            0b_00000000000000000 => self.plus_one(br),
            0b_00000000000000010 => self.field_path_encode_finish(br),
            0b_00000000000001110 => self.plus_two(br),
            0b_00000000000001111 => self.push_one_left_delta_n_right_non_zero_pack6_bits(br),
            0b_00000000000011000 => self.push_one_left_delta_one_right_non_zero(br),
            0b_00000000000011010 => self.plus_n(br),
            0b_00000000000110010 => self.plus_three(br),
            0b_00000000000110011 => self.pop_all_but_one_plus_one(br),
            0b_00000000011011001 => self.push_one_left_delta_n_right_non_zero(br),
            0b_00000000011011010 => self.push_one_left_delta_one_right_zero(br),
            0b_00000000011011100 => self.push_one_left_delta_n_right_zero(br),
            0b_00000000011011110 => self.pop_all_but_one_plus_n_pack6_bits(br),
            0b_00000000011011111 => self.plus_four(br),
            0b_00000000110110000 => self.pop_all_but_one_plus_n(br),
            0b_00000000110110110 => self.push_one_left_delta_n_right_non_zero_pack8_bits(br),
            0b_00000000110110111 => self.non_topo_penultimate_pluse_one(br),
            0b_00000000110111010 => self.pop_all_but_one_plus_n_pack3_bits(br),
            0b_00000000110111011 => self.push_n_and_non_topographical(br),
            0b_00000001101100010 => self.non_topo_complex_pack4_bits(br),
            0b_00000011011000111 => self.non_topo_complex(br),
            0b_00000110110001101 => self.push_one_left_delta_zero_right_zero(br),
            0b_00110110001100001 => self.pop_one_plus_one(br),
            0b_00110110001100101 => self.push_one_left_delta_zero_right_non_zero(br),
            0b_01101100011000000 => self.pop_n_and_non_topographical(br),
            0b_01101100011000001 => self.pop_n_plus_n(br),
            0b_01101100011000100 => self.push_n(br),
            0b_01101100011000101 => self.push_three_pack5_left_delta_n(br),
            0b_01101100011000110 => self.pop_n_plus_one(br),
            0b_01101100011000111 => self.pop_one_plus_n(br),
            0b_01101100011001000 => self.push_two_left_delta_zero(br),
            0b_11011000110010010 => self.push_three_left_delta_zero(br),
            0b_11011000110010011 => self.push_two_pack5_left_delta_zero(br),
            0b_11011000110011000 => self.push_two_left_delta_n(br),
            0b_11011000110011001 => self.push_three_pack5_left_delta_one(br),
            0b_11011000110011010 => self.push_three_left_delta_n(br),
            0b_11011000110011011 => self.push_two_pack5_left_delta_n(br),
            0b_11011000110011100 => self.push_two_left_delta_one(br),
            0b_11011000110011101 => self.push_three_pack5_left_delta_zero(br),
            0b_11011000110011110 => self.push_three_left_delta_one(br),
            0b_11011000110011111 => self.push_two_pack5_left_delta_one(br),
            // NOTE: false indicates that no op was found for the provided id
            _ => return false,
        };

        true
    }
}

// ----

// NOTE: majority of field path reads are shorter then 32 (but some are beyond thousand).
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

            // FieldPathEncodeFinish; don't store finished bool on fieldpath because it adds extra
            // weight to the struct plus it's useless everywhere else but here.
            if id == 0b_10 {
                return Ok(&mut fps[..i as usize]);
            }

            if fp.exec_op(id, br) {
                fps[i as usize] = fp.clone();
                i += 1;
                continue 'outer_loop;
            }
        }

        return Err(Error::ExhaustedMaxOpBits);
    }
}
