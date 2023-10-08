use crate::{
    bitbuf::{self, BitReader},
    fieldvalue::FieldValue,
    flattenedserializers::FlattenedSerializerField,
    fnv1a,
    quantizedfloat::{self, QuantizedFloat},
};
use dyn_clone::DynClone;
use std::{fmt::Debug, mem::MaybeUninit};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // crate
    #[error(transparent)]
    BitBuf(#[from] bitbuf::Error),
    #[error(transparent)]
    QuantizedFloat(#[from] quantizedfloat::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

// ----

pub trait FieldDecode: DynClone + Debug {
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue>;
}

dyn_clone::clone_trait_object!(FieldDecode);

// ----

#[derive(Debug, Clone, Default)]
pub struct U32Decoder {}

impl FieldDecode for U32Decoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        Ok(br.read_uvarint32()?.into())
    }
}

// ----

#[derive(Debug, Clone, Default)]
struct InternalU64Decoder {}

impl FieldDecode for InternalU64Decoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        Ok(br.read_uvarint64()?.into())
    }
}

#[derive(Debug, Clone, Default)]
struct InternalU64Fixed64Decoder {}

impl FieldDecode for InternalU64Fixed64Decoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        let mut buf: [u8; 8] = unsafe { MaybeUninit::uninit().assume_init() };
        br.read_bytes(&mut buf)?;
        Ok(u64::from_le_bytes(buf).into())
    }
}

#[derive(Debug, Clone)]
pub struct U64Decoder {
    decoder: Box<dyn FieldDecode>,
}

impl U64Decoder {
    pub fn new(field: &FlattenedSerializerField) -> Self {
        if field.is_var_encoder_hash_eq(fnv1a::hash_u8(b"fixed64")) {
            Self {
                decoder: Box::<InternalU64Fixed64Decoder>::default(),
            }
        } else {
            Self {
                decoder: Box::<InternalU64Decoder>::default(),
            }
        }
    }
}

impl FieldDecode for U64Decoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        self.decoder.decode(br)
    }
}

// ----

#[derive(Debug, Clone, Default)]
pub struct I32Decoder {}

impl FieldDecode for I32Decoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        Ok(br.read_varint32()?.into())
    }
}

// ----

#[derive(Debug, Clone, Default)]
pub struct I64Decoder {}

impl FieldDecode for I64Decoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        Ok(br.read_varint64()?.into())
    }
}

// ----

#[derive(Debug, Clone, Default)]
pub struct BoolDecoder {}

impl FieldDecode for BoolDecoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        Ok(br.read_bool()?.into())
    }
}

// ----

#[derive(Debug, Clone, Default)]
pub struct StringDecoder {}

impl FieldDecode for StringDecoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        let mut buf: [u8; 1024] = unsafe { MaybeUninit::uninit().assume_init() };
        let n = br.read_string(&mut buf, false)?;
        Ok(Box::<str>::from(unsafe { std::str::from_utf8_unchecked(&buf[..n]) }).into())
    }
}

// ----

#[derive(Debug, Clone)]
struct InternalQAnglePitchYawDecoder {
    bit_count: usize,
}

impl FieldDecode for InternalQAnglePitchYawDecoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        let vec3 = [
            br.read_bitangle(self.bit_count)?,
            br.read_bitangle(self.bit_count)?,
            0.0,
        ];
        Ok(vec3.into())
    }
}

#[derive(Debug, Clone, Default)]
struct InternalQAngleNoBitCountDecoder {}

impl FieldDecode for InternalQAngleNoBitCountDecoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        let rx = br.read_bool()?;
        let ry = br.read_bool()?;
        let rz = br.read_bool()?;
        let vec3 = [
            if rx { br.read_bitcoord()? } else { 0.0 },
            if ry { br.read_bitcoord()? } else { 0.0 },
            if rz { br.read_bitcoord()? } else { 0.0 },
        ];
        Ok(vec3.into())
    }
}

#[derive(Debug, Clone)]
struct InternalQAngleBitCountDecoder {
    bit_count: usize,
}

impl FieldDecode for InternalQAngleBitCountDecoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        let vec3 = [
            br.read_bitangle(self.bit_count)?,
            br.read_bitangle(self.bit_count)?,
            br.read_bitangle(self.bit_count)?,
        ];
        Ok(vec3.into())
    }
}

#[derive(Debug, Clone)]
pub struct QAngleDecoder {
    decoder: Box<dyn FieldDecode>,
}

impl QAngleDecoder {
    pub fn new(field: &FlattenedSerializerField) -> Self {
        let bit_count = field.bit_count.unwrap_or_default() as usize;

        if field.is_var_encoder_hash_eq(fnv1a::hash_u8(b"qangle_pitch_yaw")) {
            return Self {
                decoder: Box::new(InternalQAnglePitchYawDecoder { bit_count }),
            };
        }

        if bit_count == 0 {
            return Self {
                decoder: Box::<InternalQAngleNoBitCountDecoder>::default(),
            };
        }

        Self {
            decoder: Box::new(InternalQAngleBitCountDecoder { bit_count }),
        }
    }
}

impl FieldDecode for QAngleDecoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        self.decoder.decode(br)
    }
}

// ----

trait InternalF32Decode: DynClone + Debug {
    fn decode(&self, br: &mut BitReader) -> Result<f32>;
}

dyn_clone::clone_trait_object!(InternalF32Decode);

// ----

#[derive(Debug, Clone)]
struct InternalQuantizedFloatDecoder {
    quantized_float: QuantizedFloat,
}

impl InternalQuantizedFloatDecoder {
    pub fn new(field: &FlattenedSerializerField) -> Result<Self> {
        Ok(Self {
            quantized_float: QuantizedFloat::new(
                field.bit_count.unwrap_or_default(),
                field.encode_flags.unwrap_or_default(),
                field.low_value.unwrap_or_default(),
                field.high_value.unwrap_or_default(),
            )?,
        })
    }
}

impl InternalF32Decode for InternalQuantizedFloatDecoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<f32> {
        Ok(self.quantized_float.decode(br)?)
    }
}

#[derive(Debug, Clone)]
pub struct QuantizedFloatDecoder {
    decoder: Box<dyn InternalF32Decode>,
}

impl QuantizedFloatDecoder {
    pub fn new(field: &FlattenedSerializerField) -> Result<Self> {
        Ok(Self {
            decoder: Box::new(InternalQuantizedFloatDecoder::new(field)?),
        })
    }
}

impl FieldDecode for QuantizedFloatDecoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        Ok(self.decoder.decode(br)?.into())
    }
}

// ----

#[derive(Debug, Clone, Default)]
struct InternalF32SimulationTimeDecoder {}

impl InternalF32Decode for InternalF32SimulationTimeDecoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<f32> {
        const TICK_INTERVAL: f32 = 1.0 / 30.0;
        Ok(br
            .read_uvarint32()
            .map(|value| value as f32 * TICK_INTERVAL)?)
    }
}

#[derive(Debug, Clone, Default)]
struct InternalF32CoordDecoder {}

impl InternalF32Decode for InternalF32CoordDecoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<f32> {
        Ok(br.read_bitcoord()?)
    }
}

#[derive(Debug, Clone, Default)]
struct InternalF32NoScaleDecoder {}

impl InternalF32Decode for InternalF32NoScaleDecoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<f32> {
        Ok(br.read_bitfloat()?)
    }
}

#[derive(Debug, Clone)]
pub struct InternalF32Decoder {
    decoder: Box<dyn InternalF32Decode>,
}

impl InternalF32Decoder {
    pub fn new(field: &FlattenedSerializerField) -> Result<Self> {
        if field.var_name_hash == fnv1a::hash_u8(b"m_flSimulationTime")
            || field.var_name_hash == fnv1a::hash_u8(b"m_flAnimTime")
        {
            return Ok(Self {
                decoder: Box::<InternalF32SimulationTimeDecoder>::default(),
            });
        }

        if field.is_var_encoder_hash_eq(fnv1a::hash_u8(b"coord")) {
            return Ok(Self {
                decoder: Box::<InternalF32CoordDecoder>::default(),
            });
        }

        let bit_count = field.bit_count.unwrap_or_default();
        // why would it be greater than 32? :thinking:
        if bit_count == 0 || bit_count >= 32 {
            return Ok(Self {
                decoder: Box::<InternalF32NoScaleDecoder>::default(),
            });
        }

        Ok(Self {
            decoder: Box::new(InternalQuantizedFloatDecoder::new(field)?),
        })
    }
}

impl InternalF32Decode for InternalF32Decoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<f32> {
        self.decoder.decode(br)
    }
}

#[derive(Debug, Clone)]
pub struct F32Decoder {
    decoder: Box<dyn InternalF32Decode>,
}

impl F32Decoder {
    pub fn new(field: &FlattenedSerializerField) -> Result<Self> {
        Ok(Self {
            decoder: Box::new(InternalF32Decoder::new(field)?),
        })
    }
}

impl FieldDecode for F32Decoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        Ok(self.decoder.decode(br)?.into())
    }
}

// ----

#[derive(Debug, Clone)]
pub struct Vec2Decoder {
    decoder: Box<dyn InternalF32Decode>,
}

impl Vec2Decoder {
    pub fn new(field: &FlattenedSerializerField) -> Result<Self> {
        Ok(Self {
            decoder: Box::new(InternalF32Decoder::new(field)?),
        })
    }
}

impl FieldDecode for Vec2Decoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        let vec2 = [self.decoder.decode(br)?, self.decoder.decode(br)?];
        Ok(vec2.into())
    }
}

// ----

#[derive(Debug, Clone)]
pub struct Vec3Decoder {
    decoder: Box<dyn InternalF32Decode>,
}

impl Vec3Decoder {
    pub fn new(field: &FlattenedSerializerField) -> Result<Self> {
        Ok(Self {
            decoder: Box::new(InternalF32Decoder::new(field)?),
        })
    }
}

impl FieldDecode for Vec3Decoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        let vec3 = [
            self.decoder.decode(br)?,
            self.decoder.decode(br)?,
            self.decoder.decode(br)?,
        ];
        Ok(vec3.into())
    }
}

// ----

#[derive(Debug, Clone)]
pub struct Vec4Decoder {
    decoder: Box<dyn InternalF32Decode>,
}

impl Vec4Decoder {
    pub fn new(field: &FlattenedSerializerField) -> Result<Self> {
        Ok(Self {
            decoder: Box::new(InternalF32Decoder::new(field)?),
        })
    }
}

impl FieldDecode for Vec4Decoder {
    #[inline]
    fn decode(&self, br: &mut BitReader) -> Result<FieldValue> {
        let vec4 = [
            self.decoder.decode(br)?,
            self.decoder.decode(br)?,
            self.decoder.decode(br)?,
            self.decoder.decode(br)?,
        ];
        Ok(vec4.into())
    }
}
