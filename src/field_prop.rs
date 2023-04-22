use crate::{
    bitreader::BitReader, error::Result, flattened_serializers::FlattenedSerializerField,
    quantized_float::CNetworkedQuantizedFloat,
};
use compact_str::CompactString;
use std::{alloc::Allocator, fmt::Debug};

// csgo: engine/dt_encode.cpp
//
// in csgo decodedrs invoke proxies (proxy is not required, but for example it
// is defined for m_flSimulationTime in game/client/c_baseentity.cpp):
// RecvPropInt( RECVINFO(m_flSimulationTime), 0, RecvProxy_SimulationTime )

#[derive(Debug)]
pub enum FieldProp {
    F32(f32),
    U32(u32),
    U64(u64),
    I32(i32),
    I64(i64),
    Bool(bool),
    Vector([f32; 3]),
    String(CompactString),
}

// findings:
// CSVCMsg_ServerInfo msg has tick_interval prop, according to my tests it's
// equal to 0.033333335;
// all listed above projects do 1.0 / 30.0:
// butterfly (src/butterfly/private/property_decoder.cpp)
// manta (field_decoder.go)
// clarity (src/main/java/skadistats/clarity/io/decoder/FloatSimulationTimeDecoder.java)
pub fn decode_simulation_time<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    _f: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    const TICK_INTERVAL: f32 = 1.0 / 30.0;
    Ok(FieldProp::F32(br.read_varu32()? as f32 * TICK_INTERVAL))
}

pub fn decode_varu32<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    _f: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    br.read_varu32().map(FieldProp::U32)
}

pub fn decode_varu64<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    _f: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    br.read_varu64().map(FieldProp::U64)
}

pub fn decode_vari32<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    _f: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    br.read_vari32().map(FieldProp::I32)
}

pub fn decode_vari64<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    _f: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    br.read_vari64().map(FieldProp::I64)
}

pub fn decode_bool<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    _f: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    br.read_bool().map(FieldProp::Bool)
}

#[inline(always)]
fn internal_decode_quantized_float<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    f: &FlattenedSerializerField<A>,
) -> Result<f32> {
    // TODO: cache decoders
    let qf = CNetworkedQuantizedFloat::new(
        f.bit_count.unwrap_or_default(),
        f.low_value.unwrap_or_default(),
        f.high_value.unwrap_or_default(),
        f.encode_flags.unwrap_or_default(),
    );
    qf.decode(br)
}

pub fn decode_quantized_float<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    f: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    internal_decode_quantized_float(br, f).map(FieldProp::F32)
}

#[inline(always)]
fn internal_decode_qangle_no_bit_count(br: &mut BitReader) -> Result<[f32; 3]> {
    let (b0, b1, b2) = (br.read_bool()?, br.read_bool()?, br.read_bool()?);
    let mut vector: [f32; 3] = [0.0; 3];
    if b0 {
        vector[0] = br.read_coord()?;
    }
    if b1 {
        vector[1] = br.read_coord()?;
    }
    if b2 {
        vector[2] = br.read_coord()?;
    }
    Ok(vector)
}

// stolen from clarity
pub fn decode_qangle<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    f: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    let bc = f.bit_count.unwrap_or_default();
    if bc == 0 {
        internal_decode_qangle_no_bit_count(br).map(FieldProp::Vector)
    } else {
        unimplemented!()
    }
}

pub fn decode_float32_coord<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    _: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    br.read_coord().map(FieldProp::F32)
}

#[inline(always)]
fn internal_decode_float32_noscale(br: &mut BitReader) -> Result<f32> {
    Ok(f32::from_bits(br.read(32)?))
}

#[inline(always)]
fn internal_decode_float32<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    f: &FlattenedSerializerField<A>,
) -> Result<f32> {
    let bc = f.bit_count.unwrap_or_default();
    if bc == 0 || bc >= 32 {
        internal_decode_float32_noscale(br)
    } else {
        // for example in csgo in game/server/player.cpp
        // SendPropFloat		( SENDINFO(m_flFriction),		8,	SPROP_ROUNDDOWN,	0.0f,	4.0f),
        internal_decode_quantized_float(br, f)
    }
}

pub fn decode_float32<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    f: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    internal_decode_float32(br, f).map(FieldProp::F32)
}

pub fn decode_vector<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    f: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    Ok(FieldProp::Vector([
        internal_decode_float32(br, f)?,
        internal_decode_float32(br, f)?,
        internal_decode_float32(br, f)?,
    ]))
}

pub fn decode_string<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    _: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    let mut buf = [0u8; 1024];
    Ok(FieldProp::String(CompactString::from_utf8(
        br.read_string(&mut buf[..])?,
    )?))
}

pub fn decode_fixed64<A: Allocator + Clone + Debug>(
    br: &mut BitReader,
    _: &FlattenedSerializerField<A>,
) -> Result<FieldProp> {
    Ok(FieldProp::U64(
        (br.read(32)? as u64) | ((br.read(32)? as u64) << 32),
    ))
}
