// NOTE: looking into public/dt_common.h might help to get more ideas about field value thing.

// NOTE: Clone derive is needed here because Entity in entities.rs needs to be
// clonable which means that all members of it also should be clonable.
#[derive(Clone)]
pub enum FieldValue {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Bool(bool),
    F32(f32),
    Vector([f32; 3]),
    Vector2D([f32; 2]),
    Vector4D([f32; 4]),
    QAngle([f32; 3]),
    // TODO: array backed string
    //
    // NOTE: for example smol_str does not fit well because its default size
    // when it's empty on 64 bit arch is 64 bit xd, but Box<str>'s size is 16.
    String(Box<str>),
}

#[derive(Debug, thiserror::Error)]
#[error("tried to convert a field value into something that it isn't")]
pub struct FieldValueTryIntoError(());

macro_rules! impl_try_into {
    ($($variant:ident => $ty:ty),+) => {
        $(
            impl TryInto<$ty> for &FieldValue {
                type Error = FieldValueTryIntoError;

                fn try_into(self) -> Result<$ty, Self::Error> {
                    match self {
                        FieldValue::$variant(value) => Ok(*value),
                        _ => Err(FieldValueTryIntoError(())),
                    }
                }
            }
        )+
    }
}

impl_try_into! {
    I8 => i8,
    I16 => i16,
    I32 => i32,
    I64 => i64,
    U8 => u8,
    U16 => u16,
    U32 => u32,
    U64 => u64,
    Bool => bool,
    F32 => f32,
    Vector2D => [f32; 2],
    Vector4D => [f32; 4]
}

impl TryInto<[f32; 3]> for &FieldValue {
    type Error = FieldValueTryIntoError;

    fn try_into(self) -> Result<[f32; 3], Self::Error> {
        match self {
            FieldValue::Vector(value) | FieldValue::QAngle(value) => Ok(*value),
            _ => Err(FieldValueTryIntoError(())),
        }
    }
}

impl TryInto<String> for &FieldValue {
    type Error = FieldValueTryIntoError;

    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            FieldValue::String(value) => Ok(value.to_string()),
            _ => Err(FieldValueTryIntoError(())),
        }
    }
}

macro_rules! impl_debug {
    ($($variant:ident),+) => {
        impl std::fmt::Debug for FieldValue {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(Self::$variant(value) => f.debug_tuple(stringify!($variant)).field(value).finish(),)+
                }
            }
        }
    };
}

impl_debug! {
    I8, I16, I32, I64,
    U8, U16, U32, U64,
    Bool, F32,
    Vector, Vector2D, Vector4D, QAngle,
    String
}

macro_rules! impl_display {
    ($($variant:ident),+) => {
        impl std::fmt::Display for FieldValue {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(Self::$variant(value) => write!(f, "{:?}", value),)+
                }
            }
        }
    };
}

impl_display! {
    I8, I16, I32, I64,
    U8, U16, U32, U64,
    Bool, F32,
    Vector, Vector2D, Vector4D, QAngle,
    String
}
