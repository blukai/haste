use crate::{
    bitreader::BitReader,
    fieldvalue::FieldValue,
    flattenedserializers::FlattenedSerializerField,
    fxhash,
    quantizedfloat::{self, QuantizedFloat},
};
use std::{fmt::Debug, mem::MaybeUninit};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // crate
    #[error(transparent)]
    QuantizedFloat(#[from] quantizedfloat::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

// NOTE: PropTypeFns (from csgo source code) is what you are looking for, it has all the encoders,
// decoders, proxies and all of the stuff.

#[derive(Debug)]
pub struct FieldDecoderContext {
    pub tick_interval: f32,
}

pub type FieldDecoder = fn(
    field: &FlattenedSerializerField,
    ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue>;

type InternalFieldDecoder<T> = fn(
    field: &FlattenedSerializerField,
    ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<T>;

// placeholder (used during multi-phase initialization. never called)
// -----------

#[cold]
pub fn decode_invalid(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    _br: &mut BitReader,
) -> Result<FieldValue> {
    unreachable!()
}

// simple primitives
// -----------------

pub fn decode_i32(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    Ok(FieldValue::I32(br.read_varint32()))
}

pub fn decode_i64(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    Ok(FieldValue::I64(br.read_varint64()))
}

pub fn decode_u32(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    Ok(FieldValue::U32(br.read_uvarint32()))
}

pub fn decode_bool(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    Ok(FieldValue::Bool(br.read_bool()))
}

pub fn decode_string(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    // NOTE(blukai): the stuff is safe cause we can make sure here that no uninit memory is being read.
    #[allow(invalid_value)]
    let mut buf: [u8; 1024] = unsafe { MaybeUninit::uninit().assume_init() };
    let n = br.read_string(&mut buf, false);
    // TODO(blukai): should string conversion be actually checked? why not?
    Ok(FieldValue::String(Box::<str>::from(unsafe {
        std::str::from_utf8_unchecked(&buf[..n])
    })))
}

// u64
// ---

fn decode_u64(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    Ok(FieldValue::U64(br.read_uvarint64()))
}

fn decode_u64_fixed(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    let mut buf = [0u8; 8];
    br.read_bytes(&mut buf);
    Ok(FieldValue::U64(u64::from_le_bytes(buf)))
}

pub fn determine_u64_decoder(field: &FlattenedSerializerField) -> FieldDecoder {
    if field.var_encoder_heq(fxhash::hash_bytes(b"fixed64")) {
        decode_u64_fixed
    } else {
        decode_u64
    }
}

// f32 (internal)
// ---

#[inline(always)]
fn internal_decode_f32_simulation_time(
    _field: &FlattenedSerializerField,
    ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<f32> {
    Ok(br.read_uvarint32() as f32 * ctx.tick_interval)
}

#[inline(always)]
fn internal_decode_f32_coord(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<f32> {
    Ok(br.read_bitcoord())
}

#[inline(always)]
fn internal_decode_f32_normal(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<f32> {
    Ok(br.read_bitnormal())
}

#[inline(always)]
fn internal_decode_f32_noscale(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<f32> {
    Ok(br.read_bitfloat())
}

#[inline(always)]
fn internal_decode_f32_quantized(
    field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<f32> {
    // TODO(blukai): would it be beneficail to "cache" quantized float decoders (within the
    // FieldDecoderContext)?
    let qf = QuantizedFloat::new(
        field.bit_count.unwrap_or_default(),
        field.encode_flags.unwrap_or_default(),
        field.low_value.unwrap_or_default(),
        field.high_value.unwrap_or_default(),
    )?;
    Ok(qf.decode(br))
}

fn internal_determine_f32_decoder(field: &FlattenedSerializerField) -> InternalFieldDecoder<f32> {
    if field.var_name.hash == fxhash::hash_bytes(b"m_flSimulationTime")
        || field.var_name.hash == fxhash::hash_bytes(b"m_flAnimTime")
    {
        return internal_decode_f32_simulation_time;
    }

    if let Some(var_encoder) = field.var_encoder.as_ref() {
        match var_encoder.hash {
            hash if hash == fxhash::hash_bytes(b"coord") => {
                return internal_decode_f32_coord;
            }
            hash if hash == fxhash::hash_bytes(b"normal") => {
                return internal_decode_f32_normal;
            }
            _ => unimplemented!("{:?}", var_encoder),
        }
    }

    let bit_count = field.bit_count.unwrap_or_default();
    // NOTE: that would mean that something is seriously wrong - in that case yell at me
    // loudly.
    debug_assert!(bit_count >= 0 && bit_count <= 32);
    if bit_count == 0 || bit_count == 32 {
        return internal_decode_f32_noscale;
    }

    return internal_decode_f32_quantized;
}

// f32
// ---

fn decode_f32_simulation_time(
    field: &FlattenedSerializerField,
    ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    internal_decode_f32_simulation_time(field, ctx, br).map(FieldValue::F32)
}

fn decode_f32_coord(
    field: &FlattenedSerializerField,
    ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    internal_decode_f32_coord(field, ctx, br).map(FieldValue::F32)
}

fn decode_f32_normal(
    field: &FlattenedSerializerField,
    ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    internal_decode_f32_normal(field, ctx, br).map(FieldValue::F32)
}

fn decode_f32_noscale(
    field: &FlattenedSerializerField,
    ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    internal_decode_f32_noscale(field, ctx, br).map(FieldValue::F32)
}

fn decode_f32_quantized(
    field: &FlattenedSerializerField,
    ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    internal_decode_f32_quantized(field, ctx, br).map(FieldValue::F32)
}

// NOTE(blukai): this is a 1:1 duplicate of internal_determine_f32_decoder.
pub fn determine_f32_decoder(field: &FlattenedSerializerField) -> FieldDecoder {
    if field.var_name.hash == fxhash::hash_bytes(b"m_flSimulationTime")
        || field.var_name.hash == fxhash::hash_bytes(b"m_flAnimTime")
    {
        return decode_f32_simulation_time;
    }

    if let Some(var_encoder) = field.var_encoder.as_ref() {
        match var_encoder.hash {
            hash if hash == fxhash::hash_bytes(b"coord") => {
                return decode_f32_coord;
            }
            hash if hash == fxhash::hash_bytes(b"normal") => {
                return decode_f32_normal;
            }
            _ => unimplemented!("{:?}", var_encoder),
        }
    }

    let bit_count = field.bit_count.unwrap_or_default();
    // NOTE: that would mean that something is seriously wrong - in that case yell at me
    // loudly.
    debug_assert!(bit_count >= 0 && bit_count <= 32);
    if bit_count == 0 || bit_count == 32 {
        return decode_f32_noscale;
    }

    return decode_f32_quantized;
}

// qangle
// ------

fn decode_qangle_pitch_yaw(
    field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    let bit_count = field.bit_count.unwrap_or_default() as usize;
    debug_assert!(bit_count > 0 && bit_count <= 32);
    let vec3 = [
        br.read_bitangle(bit_count),
        br.read_bitangle(bit_count),
        0.0,
    ];
    Ok(FieldValue::QAngle(vec3))
}

fn decode_qangle_precise(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    let mut vec3 = [0f32; 3];

    let rx = br.read_bool();
    let ry = br.read_bool();
    let rz = br.read_bool();

    if rx {
        vec3[0] = br.read_bitangle(20);
    }
    if ry {
        vec3[1] = br.read_bitangle(20);
    }
    if rz {
        vec3[2] = br.read_bitangle(20);
    }

    Ok(FieldValue::QAngle(vec3))
}

fn decode_qangle_no_bit_count(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    Ok(FieldValue::QAngle(br.read_bitvec3coord()))
}

fn decode_qangle_bit_count(
    field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    let bit_count = field.bit_count.unwrap_or_default() as usize;
    debug_assert!(bit_count > 0 && bit_count <= 32);
    let vec3 = [
        br.read_bitangle(bit_count),
        br.read_bitangle(bit_count),
        br.read_bitangle(bit_count),
    ];
    Ok(FieldValue::QAngle(vec3))
}

pub fn determine_qangle_decoder(field: &FlattenedSerializerField) -> FieldDecoder {
    if let Some(var_encoder) = field.var_encoder.as_ref() {
        match var_encoder.hash {
            hash if hash == fxhash::hash_bytes(b"qangle_pitch_yaw") => {
                return decode_qangle_pitch_yaw;
            }
            hash if hash == fxhash::hash_bytes(b"qangle_precise") => {
                return decode_qangle_precise;
            }

            hash if hash == fxhash::hash_bytes(b"qangle") => {}
            // NOTE(blukai): naming of var encoders seem inconsistent. found this pascal cased
            // name in dota 2 replay from 2018.
            hash if hash == fxhash::hash_bytes(b"QAngle") => {}

            _ => unimplemented!("{:?}", var_encoder),
        }
    }

    let bit_count = field.bit_count.unwrap_or_default() as usize;
    if bit_count == 0 {
        return decode_qangle_no_bit_count;
    }

    return decode_qangle_bit_count;
}

// vector
// ------

fn decode_vector(
    field: &FlattenedSerializerField,
    ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    // TODO(blukai): would it make sense to cache vector's f32 decoder?
    let decode_f32 = internal_determine_f32_decoder(field);
    let vec3 = [
        decode_f32(field, ctx, br)?,
        decode_f32(field, ctx, br)?,
        decode_f32(field, ctx, br)?,
    ];
    Ok(FieldValue::Vector(vec3))
}

fn decode_vector_normal(
    _field: &FlattenedSerializerField,
    _ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    Ok(FieldValue::Vector(br.read_bitvec3normal()))
}

pub fn determine_vector_decoder(field: &FlattenedSerializerField) -> FieldDecoder {
    if field.var_encoder_heq(fxhash::hash_bytes(b"normal")) {
        decode_vector_normal
    } else {
        decode_vector
    }
}

// vector2d
// --------

pub fn decode_vector2d(
    field: &FlattenedSerializerField,
    ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    // TODO(blukai): would it make sense to cache vector2d's f32 decoder?
    let decode_f32 = internal_determine_f32_decoder(field);
    let vec2 = [decode_f32(field, ctx, br)?, decode_f32(field, ctx, br)?];
    Ok(FieldValue::Vector2D(vec2))
}

// vector4d
// --------

pub fn decode_vector4d(
    field: &FlattenedSerializerField,
    ctx: &FieldDecoderContext,
    br: &mut BitReader,
) -> Result<FieldValue> {
    // TODO(blukai): would it make sense to cache vector4d's f32 decoder?
    let decode_f32 = internal_determine_f32_decoder(field);
    let vec4 = [
        decode_f32(field, ctx, br)?,
        decode_f32(field, ctx, br)?,
        decode_f32(field, ctx, br)?,
        decode_f32(field, ctx, br)?,
    ];
    Ok(FieldValue::Vector4D(vec4))
}
