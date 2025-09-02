// NOTE: looking into public/dt_common.h might help to get more ideas about field value thing.
//
// NOTE: don't bother creating variants for ints that are smaller then 64 bits. that will not make
// enum consume less space; that will not make decoding any faster (the exception could be i8/u8
// cause those won't branch, but that ain't worth it really).
//
// NOTE: Clone derive is needed here because Entity in entities.rs needs to be
// clonable which means that all members of it also should be clonable.
#[derive(Clone)]
pub enum FieldValue {
    I64(i64),
    U64(u64),
    F32(f32),
    Bool(bool),
    Vector3([f32; 3]),
    Vector2([f32; 2]),
    Vector4([f32; 4]),
    QAngle([f32; 3]),
    /// NOTE: not all strings are valid utf8 strings.
    ///
    /// deadlock has `CCitadelPlayerPawn` entity which has a `m_sHeroBuildSerialized` field of type
    /// `CUtlString` which, as the name suggests, contains serialized data which canno't be
    /// successfully converted to str/String.
    ///
    /// in c/cpp there's no enforcement on validition of strings. but in rust there is, see
    /// https://doc.rust-lang.org/stable/std/string/struct.String.html#utf-8.
    ///
    /// it doesn't not seem like thre's a reliable way to know which strings are treated as actual
    /// strings and which ones are represent non-utf8 byte sequences to encode other data.
    ///
    /// also note that there's no data type in demo files that is dedicated specifically for
    /// transfering bytes.
    ///
    /// use String::from_utf8_lossy in your code to convert this to an actual string, and handle
    /// conversion errors.
    String(Box<[u8]>),
}

// TODO(blukai): when you'll be unfucking errors - rename this one to FieldValueInvalidConversion
// or something..
#[derive(Debug, thiserror::Error)]
#[error("incompatible types or out of range integer type conversion attempted")]
pub enum FieldValueConversionError {
    #[error("incompatible types or out of range integer type conversion attempted")]
    IncompatibleTypeOrOutOfRangeInteger,
    #[error("utf-8 conversion error: {0}")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
}

macro_rules! impl_try_into_numeric {
    ($($variant:ident => $ty:ty),+) => {
        $(
            impl TryInto<$ty> for FieldValue {
                type Error = FieldValueConversionError;

                fn try_into(self) -> Result<$ty, Self::Error> {
                    match self {
                        FieldValue::$variant(value) => value.try_into().map_err(|_| FieldValueConversionError::IncompatibleTypeOrOutOfRangeInteger),
                        _ => Err(FieldValueConversionError::IncompatibleTypeOrOutOfRangeInteger),
                    }
                }
            }
        )+
    }
}

impl_try_into_numeric! {
    I64 => i8,
    I64 => i16,
    I64 => i32,
    I64 => i64,
    U64 => u8,
    U64 => u16,
    U64 => u32,
    U64 => u64,
    F32 => f32
}

macro_rules! impl_try_into_inner {
    ($($variant:ident => $ty:ty),+) => {
        $(
            impl TryInto<$ty> for FieldValue {
                type Error = FieldValueConversionError;

                fn try_into(self) -> Result<$ty, Self::Error> {
                    match self {
                        FieldValue::$variant(value) => Ok(value),
                        _ => Err(FieldValueConversionError::IncompatibleTypeOrOutOfRangeInteger),
                    }
                }
            }
        )+
    }
}

impl_try_into_inner! {
    Bool => bool,
    Vector2 => [f32; 2],
    Vector4 => [f32; 4]
}

// and some specials...

impl TryInto<[f32; 3]> for FieldValue {
    type Error = FieldValueConversionError;

    fn try_into(self) -> Result<[f32; 3], Self::Error> {
        match self {
            FieldValue::Vector3(value) | FieldValue::QAngle(value) => Ok(value),
            _ => Err(FieldValueConversionError::IncompatibleTypeOrOutOfRangeInteger),
        }
    }
}

impl TryInto<String> for FieldValue {
    type Error = FieldValueConversionError;

    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            FieldValue::String(value) => String::from_utf8(value.into_vec())
                .map_err(FieldValueConversionError::FromUtf8Error),
            _ => Err(FieldValueConversionError::IncompatibleTypeOrOutOfRangeInteger),
        }
    }
}

// debug and display...

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
    I64,
    U64,
    Bool,
    F32,
    Vector3,
    Vector2,
    Vector4,
    QAngle,
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
    I64,
    U64,
    Bool,
    F32,
    Vector3,
    Vector2,
    Vector4,
    QAngle,
    String
}
