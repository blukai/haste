use mars::fxhash;

use crate::bitreader::BitReader;
use crate::fieldvalue::FieldValue;
use crate::flattenedserializers::FlattenedSerializerField;
use crate::quantizedfloat::{QuantizedFloat, QuantizedFloatError};

// NOTE: PropTypeFns (from csgo source code) is what you are looking for, it has all the encoders,
// decoders, proxies and all of the stuff.

// public/dt_common.h
//
// NOTE(blukai): all pieces of public code define DT_MAX_STRING_BITS to 9, but that does not appear
// to be enough for deadlock (the game):
// https://github.com/blukai/haste/issues/4
//
// deadlock's `CCitadelPlayerPawn` entity has a field `m_sHeroBuildSerialized` of type
// `CUtlString`, but this does not appear to be a valid utf8 string, but as the name suggests some
// serialized data.
// for more info on that see FieldValue::String's comment.
const DT_MAX_STRING_BITS: u32 = 9;
/// maximum length of a string that can be sent.
const DT_MAX_STRING_BUFFERSIZE: u32 = 1 << DT_MAX_STRING_BITS;

#[derive(Debug)]
pub(crate) struct FieldDecoderContext {
    pub(crate) tick_interval: f32,
    pub(crate) string_buf: Vec<u8>,
}

impl Default for FieldDecoderContext {
    fn default() -> Self {
        Self {
            // NOTE(blukai): tick interval needs to be read from SvcServerInfo packet message. it
            // becomes available "later"; it is okay to initialize it to 0.0.
            tick_interval: 0.0,
            string_buf: Vec::with_capacity(DT_MAX_STRING_BUFFERSIZE as usize),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum FieldDecoderConstructionError {
    #[error(transparent)]
    QuantizedFloatError(#[from] QuantizedFloatError),
}

type DecodeFn = fn(&FieldDecoder, &mut FieldDecoderContext, &mut BitReader) -> FieldValue;

#[rustfmt::skip]
mod decoders {
    use super::*;

    use FieldDecoder as Fd;
    use FieldDecoderContext as Ctx;
    use BitReader as Br;
    use FieldValue::*;

    pub fn decode_placeholder(_fd: &Fd, _ctx: &mut Ctx, _br: &mut Br) -> FieldValue { unreachable!() }

    pub fn decode_i64_varint(_fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue { I64(br.read_varint64()) }
    pub fn decode_i64_fixed8(_fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue { I64(br.read_ubit64(8) as i8 as i64) }

    pub fn decode_u64_varint(_fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue { U64(br.read_uvarint64()) }
    pub fn decode_u64_fixed64(_fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue { U64(br.read_ubit64(64)) }
    pub fn decode_u64_fixed8(_fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue { U64(br.read_ubit64(8)) }

    pub fn decode_bool(_fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue { Bool(br.read_bool()) }

    pub fn decode_string(_fd: &Fd, ctx: &mut Ctx, br: &mut Br) -> FieldValue {
        ctx.string_buf.clear();
        let n = br.read_string_to_end(&mut ctx.string_buf, false);
        String(Box::from(&ctx.string_buf[..n]))
    }

    pub fn decode_f32_coord(_fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue { F32(br.read_bitcoord()) }
    pub fn decode_f32_no_scale(_fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue { F32(br.read_bitfloat()) }
    pub fn decode_f32_normal(_fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue { F32(br.read_bitnormal()) }
    pub fn decode_f32_tick_count(_fd: &Fd, ctx: &mut Ctx, br: &mut Br) -> FieldValue {
        F32(br.read_uvarint32() as f32 * ctx.tick_interval)
    }
    pub fn decode_f32_quantized(fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue {
        F32(fd.quantized_float.decode(br))
    }

    pub fn decode_qangle_pitch_yaw(fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue {
        QAngle([br.read_bitangle(fd.bit_count), br.read_bitangle(fd.bit_count), 0.0])
    }
    pub fn decode_qangle_precise(_fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue {
        let rx = br.read_bool();
        let ry = br.read_bool();
        let rz = br.read_bool();
        QAngle([
            if rx { br.read_bitangle(20) } else { 0.0 },
            if ry { br.read_bitangle(20) } else { 0.0 },
            if rz { br.read_bitangle(20) } else { 0.0 },
        ])
    }
    pub fn decode_qangle_no_bit_count(_fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue {
        QAngle(br.read_bitvec3coord())
    }
    pub fn decode_qangle_bit_count(fd: &Fd, _ctx: &mut Ctx, br: &mut Br) -> FieldValue {
        QAngle([br.read_bitangle(fd.bit_count), br.read_bitangle(fd.bit_count), br.read_bitangle(fd.bit_count)])
    }

    macro_rules! make_decode_vector_fn {
        ($name:ident, $variant:ident, |$fd:ident, $ctx:ident, $br: ident| $f:expr) => {
            pub fn $name($fd: &Fd, $ctx: &mut Ctx, $br: &mut Br) -> FieldValue {
                $variant(::core::array::from_fn(|_| $f))
            }
        };
    }

    make_decode_vector_fn!(decode_vector2_coord, Vector2, |_fd, _ctx, br| br.read_bitcoord());
    make_decode_vector_fn!(decode_vector2_no_scale, Vector2, |_fd, _ctx, br| br.read_bitfloat());
    make_decode_vector_fn!(decode_vector2_normal, Vector2, |_fd, _ctx, br| br.read_bitnormal());
    make_decode_vector_fn!(decode_vector2_tick_count, Vector2, |_fd, ctx, br| br.read_uvarint32() as f32 * ctx.tick_interval);
    make_decode_vector_fn!(decode_vector2_quantized, Vector2, |fd, _ctx, br| fd.quantized_float.decode(br));

    make_decode_vector_fn!(decode_vector3_coord, Vector3, |_fd, _ctx, br| br.read_bitcoord());
    make_decode_vector_fn!(decode_vector3_no_scale, Vector3, |_fd, _ctx, br| br.read_bitfloat());
    make_decode_vector_fn!(decode_vector3_normal, Vector3, |_fd, _ctx, br| br.read_bitnormal());
    make_decode_vector_fn!(decode_vector3_tick_count, Vector3, |_fd, ctx, br| br.read_uvarint32() as f32 * ctx.tick_interval);
    make_decode_vector_fn!(decode_vector3_quantized, Vector3, |fd, _ctx, br| fd.quantized_float.decode(br));

    make_decode_vector_fn!(decode_vector4_coord, Vector4, |_fd, _ctx, br| br.read_bitcoord());
    make_decode_vector_fn!(decode_vector4_no_scale, Vector4, |_fd, _ctx, br| br.read_bitfloat());
    make_decode_vector_fn!(decode_vector4_normal, Vector4, |_fd, _ctx, br| br.read_bitnormal());
    make_decode_vector_fn!(decode_vector4_tick_count, Vector4, |_fd, ctx, br| br.read_uvarint32() as f32 * ctx.tick_interval);
    make_decode_vector_fn!(decode_vector4_quantized, Vector4, |fd, _ctx, br| fd.quantized_float.decode(br));
}

use decoders::*;

enum F32DecoderKind {
    Coord,
    NoScale,
    Normal,
    TickCount,
    Quantized(QuantizedFloat),
}

impl F32DecoderKind {
    fn from_field(field: &FlattenedSerializerField) -> Result<Self, QuantizedFloatError> {
        if field.var_name.hash == fxhash::hash_bytes(b"m_flSimulationTime")
            || field.var_name.hash == fxhash::hash_bytes(b"m_flAnimTime")
        {
            return Ok(Self::TickCount);
        }

        if let Some(var_encoder) = field.var_encoder.as_ref() {
            match var_encoder.hash {
                hash if hash == fxhash::hash_bytes(b"coord") => {
                    return Ok(Self::Coord);
                }
                hash if hash == fxhash::hash_bytes(b"normal") => {
                    return Ok(Self::Normal);
                }
                _ => unimplemented!("{:?}", var_encoder),
            }
        }

        let bit_count = field.bit_count.unwrap_or_default();
        // NOTE: that would mean that something is seriously wrong - in that case yell at me
        // loudly.
        debug_assert!(bit_count >= 0 && bit_count <= 32);
        if bit_count == 0 || bit_count == 32 {
            return Ok(Self::NoScale);
        }

        QuantizedFloat::new(
            field.bit_count.unwrap_or_default(),
            field.encode_flags.unwrap_or_default(),
            field.low_value.unwrap_or_default(),
            field.high_value.unwrap_or_default(),
        )
        .map(Self::Quantized)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FieldDecoder {
    decode_fn: DecodeFn,
    bit_count: usize,
    quantized_float: QuantizedFloat,
}

impl Default for FieldDecoder {
    fn default() -> Self {
        Self::new(decode_placeholder)
    }
}

impl FieldDecoder {
    #[inline(always)]
    fn new(decode_fn: DecodeFn) -> Self {
        Self {
            decode_fn,
            bit_count: 0,
            quantized_float: const { unsafe { std::mem::zeroed() } },
        }
    }

    // ----

    #[inline(always)]
    pub(crate) fn new_i64(field: &FlattenedSerializerField) -> Self {
        Self::new(if field.var_encoder_heq(fxhash::hash_bytes(b"fixed8")) {
            decode_i64_fixed8
        } else {
            decode_i64_varint
        })
    }

    #[inline(always)]
    pub(crate) fn new_u64(field: &FlattenedSerializerField) -> Self {
        Self::new(if field.var_encoder_heq(fxhash::hash_bytes(b"fixed64")) {
            decode_u64_fixed64
        } else if field.var_encoder_heq(fxhash::hash_bytes(b"fixed8")) {
            decode_u64_fixed8
        } else {
            decode_u64_varint
        })
    }

    #[inline(always)]
    pub(crate) fn new_bool() -> Self {
        Self::new(decode_bool)
    }

    #[inline(always)]
    pub(crate) fn new_string() -> Self {
        Self::new(decode_string)
    }

    #[inline(always)]
    pub(crate) fn new_f32(
        field: &FlattenedSerializerField,
    ) -> Result<Self, FieldDecoderConstructionError> {
        let encoding = F32DecoderKind::from_field(field)
            .map_err(FieldDecoderConstructionError::QuantizedFloatError)?;
        Ok(match encoding {
            F32DecoderKind::Coord => Self::new(decode_f32_coord),
            F32DecoderKind::NoScale => Self::new(decode_f32_no_scale),
            F32DecoderKind::Normal => Self::new(decode_f32_normal),
            F32DecoderKind::TickCount => Self::new(decode_f32_tick_count),
            F32DecoderKind::Quantized(qf) => {
                let mut decoder = Self::new(decode_f32_quantized);
                decoder.quantized_float = qf;
                decoder
            }
        })
    }

    #[inline(always)]
    pub(crate) fn new_vector2(
        field: &FlattenedSerializerField,
    ) -> Result<Self, FieldDecoderConstructionError> {
        let encoding = F32DecoderKind::from_field(field)
            .map_err(FieldDecoderConstructionError::QuantizedFloatError)?;
        Ok(match encoding {
            F32DecoderKind::Coord => Self::new(decode_vector2_coord),
            F32DecoderKind::NoScale => Self::new(decode_vector2_no_scale),
            F32DecoderKind::Normal => Self::new(decode_vector2_normal),
            F32DecoderKind::TickCount => Self::new(decode_vector2_tick_count),
            F32DecoderKind::Quantized(qf) => {
                let mut decoder = Self::new(decode_vector2_quantized);
                decoder.quantized_float = qf;
                decoder
            }
        })
    }

    #[inline(always)]
    pub(crate) fn new_vector3(
        field: &FlattenedSerializerField,
    ) -> Result<Self, FieldDecoderConstructionError> {
        let encoding = F32DecoderKind::from_field(field)
            .map_err(FieldDecoderConstructionError::QuantizedFloatError)?;
        Ok(match encoding {
            F32DecoderKind::Coord => Self::new(decode_vector3_coord),
            F32DecoderKind::NoScale => Self::new(decode_vector3_no_scale),
            F32DecoderKind::Normal => Self::new(decode_vector3_normal),
            F32DecoderKind::TickCount => Self::new(decode_vector3_tick_count),
            F32DecoderKind::Quantized(qf) => {
                let mut decoder = Self::new(decode_vector3_quantized);
                decoder.quantized_float = qf;
                decoder
            }
        })
    }

    #[inline(always)]
    pub(crate) fn new_vector4(
        field: &FlattenedSerializerField,
    ) -> Result<Self, FieldDecoderConstructionError> {
        let encoding = F32DecoderKind::from_field(field)
            .map_err(FieldDecoderConstructionError::QuantizedFloatError)?;
        Ok(match encoding {
            F32DecoderKind::Coord => Self::new(decode_vector4_coord),
            F32DecoderKind::NoScale => Self::new(decode_vector4_no_scale),
            F32DecoderKind::Normal => Self::new(decode_vector4_normal),
            F32DecoderKind::TickCount => Self::new(decode_vector4_tick_count),
            F32DecoderKind::Quantized(qf) => {
                let mut decoder = Self::new(decode_vector4_quantized);
                decoder.quantized_float = qf;
                decoder
            }
        })
    }

    #[inline(always)]
    pub(crate) fn new_qangle(field: &FlattenedSerializerField) -> Self {
        let bit_count = field.bit_count.unwrap_or_default() as usize;

        if let Some(var_encoder) = field.var_encoder.as_ref() {
            match var_encoder.hash {
                hash if hash == fxhash::hash_bytes(b"qangle_pitch_yaw") => {
                    let mut decoder = Self::new(decode_qangle_pitch_yaw);
                    decoder.bit_count = bit_count;
                    return decoder;
                }
                hash if hash == fxhash::hash_bytes(b"qangle_precise") => {
                    return Self::new(decode_qangle_precise);
                }

                hash if hash == fxhash::hash_bytes(b"qangle") => {}
                // NOTE(blukai): naming of var encoders seem inconsistent. found this pascal cased
                // name in dota 2 replay from 2018.
                hash if hash == fxhash::hash_bytes(b"QAngle") => {}

                _ => unimplemented!("{:?}", var_encoder),
            }
        }

        if bit_count == 0 {
            return Self::new(decode_qangle_no_bit_count);
        }

        let mut decoder = Self::new(decode_qangle_bit_count);
        decoder.bit_count = bit_count;
        decoder
    }

    // ----

    #[inline(always)]
    pub(crate) fn decode(&self, ctx: &mut FieldDecoderContext, br: &mut BitReader) -> FieldValue {
        (self.decode_fn)(self, ctx, br)
    }
}
