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

// ----

impl std::fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I8(value) => f.write_fmt(format_args!("I8({:?})", value)),
            Self::I16(value) => f.write_fmt(format_args!("I16({:?})", value)),
            Self::I32(value) => f.write_fmt(format_args!("I32({:?})", value)),
            Self::I64(value) => f.write_fmt(format_args!("I64({:?})", value)),

            Self::U8(value) => f.write_fmt(format_args!("U8({:?})", value)),
            Self::U16(value) => f.write_fmt(format_args!("U16({:?})", value)),
            Self::U32(value) => f.write_fmt(format_args!("U32({:?})", value)),
            Self::U64(value) => f.write_fmt(format_args!("U64({:?})", value)),

            Self::Bool(value) => f.write_fmt(format_args!("Bool({:?})", value)),
            Self::F32(value) => f.write_fmt(format_args!("F32({:?})", value)),

            Self::Vector(value) => f.write_fmt(format_args!("Vector({:?})", value)),
            Self::Vector2D(value) => f.write_fmt(format_args!("Vector2D({:?})", value)),
            Self::Vector4D(value) => f.write_fmt(format_args!("Vector4D({:?})", value)),
            Self::QAngle(value) => f.write_fmt(format_args!("QAngle({:?})", value)),

            Self::String(value) => f.write_fmt(format_args!("String({:?})", value)),
        }
    }
}
