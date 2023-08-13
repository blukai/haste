use crate::{
    bitbuf::{self, BitReader},
    fieldvalue::FieldValue,
    flattenedserializers::FlattenedSerializerField,
    fnv1a::hash,
    quantizedfloat::{self, QuantizedFloat},
};
use std::alloc::Allocator;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // crate
    #[error(transparent)]
    BitBuf(#[from] bitbuf::Error),
    #[error(transparent)]
    QuantizedFloat(#[from] quantizedfloat::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

const TICK_INTERVAL: f32 = 1.0 / 30.0;

pub type FieldDecoder<A> =
    fn(field: &FlattenedSerializerField<A>, br: &mut BitReader, alloc: A) -> Result<FieldValue<A>>;

pub fn decode_u32<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    Ok(br.read_uvarint32()?.into())
}

#[inline(always)]
fn internal_decode_u64_fixed64<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
) -> Result<u64> {
    let mut bytes = [0u8; 8];
    br.read_bytes(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

pub fn decode_u64<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    if field.is_var_encoder_hash_eq(hash(b"fixed64")) {
        return Ok(internal_decode_u64_fixed64(field, br)?.into());
    }

    Ok(br.read_uvarint64()?.into())
}

pub fn decode_i32<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    Ok(br.read_varint32()?.into())
}

pub fn decode_i64<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    Ok(br.read_varint64()?.into())
}

#[inline(always)]
fn internal_decode_quantized_float<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
) -> Result<f32> {
    let qf = QuantizedFloat::new(
        field.bit_count.unwrap_or_default(),
        field.encode_flags.unwrap_or_default(),
        field.low_value.unwrap_or_default(),
        field.high_value.unwrap_or_default(),
    )?;
    Ok(qf.decode(br)?)
}

pub fn decode_quantized_float<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    Ok(internal_decode_quantized_float(field, br)?.into())
}

#[inline(always)]
fn internal_decode_f32_simulation_time<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
) -> Result<f32> {
    Ok(br
        .read_uvarint32()
        .map(|value| value as f32 * TICK_INTERVAL)?)
}

#[inline(always)]
fn internal_decode_f32_coord<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
) -> Result<f32> {
    Ok(br.read_bitcoord()?)
}

#[inline(always)]
fn internal_decode_f32_noscale(br: &mut BitReader) -> Result<f32> {
    Ok(br.read_bitfloat()?)
}

#[inline(always)]
fn internal_decode_f32<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
) -> Result<f32> {
    if field.var_name_hash == hash(b"m_flSimulationTime")
        || field.var_name_hash == hash(b"m_flAnimTime")
    {
        return internal_decode_f32_simulation_time(field, br);
    }

    if field.is_var_encoder_hash_eq(hash(b"coord")) {
        return internal_decode_f32_coord(field, br);
    }

    let bit_count = field.bit_count.unwrap_or_default();
    // why would it be greater than 32? :thinking:
    if bit_count == 0 || bit_count >= 32 {
        return internal_decode_f32_noscale(br);
    }

    internal_decode_quantized_float(field, br)
}

pub fn decode_f32<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    Ok(internal_decode_f32(field, br)?.into())
}

pub fn decode_bool<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    Ok(br.read_bool()?.into())
}

#[inline(always)]
fn internal_decode_qangle_pitch_yaw<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<Box<[f32; 3], A>> {
    let mut vec3 = Box::new_in([0.0f32; 3], alloc);
    let bit_count = field.bit_count.unwrap_or_default() as usize;
    vec3[0] = br.read_bitangle(bit_count)?;
    vec3[1] = br.read_bitangle(bit_count)?;
    Ok(vec3)
}

#[inline(always)]
fn internal_decode_qangle_no_bit_count<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<Box<[f32; 3], A>> {
    let mut vec3 = Box::new_in([0.0f32; 3], alloc);

    let rx = br.read_bool()?;
    let ry = br.read_bool()?;
    let rz = br.read_bool()?;

    if rx {
        vec3[0] = br.read_bitcoord()?;
    }
    if ry {
        vec3[1] = br.read_bitcoord()?;
    }
    if rz {
        vec3[2] = br.read_bitcoord()?;
    }

    Ok(vec3)
}

pub fn decode_qangle<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<FieldValue<A>> {
    let bit_count = field.bit_count.unwrap_or_default();

    if field.is_var_encoder_hash_eq(hash(b"qangle_pitch_yaw")) {
        return Ok(internal_decode_qangle_pitch_yaw(field, br, alloc)?.into());
    }

    if bit_count == 0 {
        return Ok(internal_decode_qangle_no_bit_count(field, br, alloc)?.into());
    }

    unimplemented!("other qangle decoder")
}

pub fn decode_vec3<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<FieldValue<A>> {
    let mut vec3 = Box::new_in([0.0f32; 3], alloc);
    vec3[0] = internal_decode_f32(field, br)?;
    vec3[1] = internal_decode_f32(field, br)?;
    vec3[2] = internal_decode_f32(field, br)?;
    Ok(vec3.into())
}

pub fn decode_vec2<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<FieldValue<A>> {
    let mut vec2 = Box::new_in([0.0f32; 2], alloc);
    vec2[0] = internal_decode_f32(field, br)?;
    vec2[1] = internal_decode_f32(field, br)?;
    Ok(vec2.into())
}

pub fn decode_vec4<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<FieldValue<A>> {
    let mut vec4 = Box::new_in([0.0f32; 4], alloc);
    vec4[0] = internal_decode_f32(field, br)?;
    vec4[1] = internal_decode_f32(field, br)?;
    vec4[2] = internal_decode_f32(field, br)?;
    vec4[3] = internal_decode_f32(field, br)?;
    Ok(vec4.into())
}

pub fn decode_string<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<FieldValue<A>> {
    let mut buf = [0u8; 1024];
    let num_chars = br.read_string(&mut buf, false)?;
    Ok(buf[..num_chars].to_vec_in(alloc).into())
}
