// from public/dt_common.h
//
// typedef enum
// {
// 	DPT_Int=0,
// 	DPT_Float,
// 	DPT_Vector,
// 	DPT_VectorXY, // Only encodes the XY of a vector, ignores Z
// 	DPT_String,
// 	DPT_Array,	// An array of the base types (can't be of datatables).
// 	DPT_DataTable,
// #if 0 // We can't ship this since it changes the size of DTVariant to be 20 bytes instead of 16 and that breaks MODs!!!
// 	DPT_Quaternion,
// #endif
// 	DPT_Int64,
// 	DPT_NUMSendPropTypes
// } SendPropType;

// NOTE: Clone derive is needed here because Entity in entities.rs needs to be
// clonable which means that all members of it also should be clonable.
#[derive(Clone)]
pub enum FieldValue {
    U32(u32), // U32 will also handle uint8 and uint16
    U64(u64),
    I32(u32), // I32 will also handle int8 and int16
    I64(u64),
    F32(f32),
    Bool(bool),
    // TODO: array backed string
    String(Box<str>),
    Vec3([f32; 3]),
    Vec2([f32; 2]),
    Vec4([f32; 4]),
}

impl From<u32> for FieldValue {
    fn from(value: u32) -> Self {
        FieldValue::U32(value)
    }
}

impl From<u64> for FieldValue {
    fn from(value: u64) -> Self {
        FieldValue::U64(value)
    }
}

impl From<i32> for FieldValue {
    fn from(value: i32) -> Self {
        FieldValue::I32(value as u32)
    }
}

impl From<i64> for FieldValue {
    fn from(value: i64) -> Self {
        FieldValue::I64(value as u64)
    }
}

impl From<f32> for FieldValue {
    fn from(value: f32) -> Self {
        FieldValue::F32(value)
    }
}

impl From<bool> for FieldValue {
    fn from(value: bool) -> Self {
        FieldValue::Bool(value)
    }
}

impl From<Box<str>> for FieldValue {
    fn from(value: Box<str>) -> Self {
        FieldValue::String(value)
    }
}

impl From<[f32; 3]> for FieldValue {
    fn from(value: [f32; 3]) -> Self {
        FieldValue::Vec3(value)
    }
}

impl From<[f32; 2]> for FieldValue {
    fn from(value: [f32; 2]) -> Self {
        FieldValue::Vec2(value)
    }
}

impl From<[f32; 4]> for FieldValue {
    fn from(value: [f32; 4]) -> Self {
        FieldValue::Vec4(value)
    }
}

// ----

impl ToString for FieldValue {
    fn to_string(&self) -> String {
        match self {
            Self::U32(value) | Self::I32(value) => format!("{:?}", value),
            Self::U64(value) | Self::I64(value) => format!("{:?}", value),
            Self::F32(value) => format!("{:?}", value),
            Self::Bool(value) => format!("{:?}", value),
            Self::String(value) => format!("{:?}", value),
            Self::Vec3(value) => format!("{:?}", value),
            Self::Vec2(value) => format!("{:?}", value),
            Self::Vec4(value) => format!("{:?}", value),
        }
    }
}

impl std::fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}
