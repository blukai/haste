use crate::allocstring::AllocString;
use std::alloc::Allocator;

// NOTE: Clone derive is needed here because Entity in entities.rs needs to be
// clonable which means that all members of it also should be clonable.
#[derive(Clone)]
pub enum FieldValue<A: Allocator + Clone> {
    U32(u32), // U32 will also handle uint8 and uint16
    U64(u64),
    I32(u32), // I32 will also handle int8 and int16
    I64(u64),
    F32(f32),
    Bool(bool),
    String(AllocString<A>),
    Vec3(Box<[f32; 3], A>),
    Vec2(Box<[f32; 2], A>),
    Vec4(Box<[f32; 4], A>),
}

impl<A: Allocator + Clone> From<u32> for FieldValue<A> {
    fn from(value: u32) -> Self {
        FieldValue::U32(value)
    }
}

impl<A: Allocator + Clone> From<u64> for FieldValue<A> {
    fn from(value: u64) -> Self {
        FieldValue::U64(value)
    }
}

impl<A: Allocator + Clone> From<i32> for FieldValue<A> {
    fn from(value: i32) -> Self {
        FieldValue::I32(value as u32)
    }
}

impl<A: Allocator + Clone> From<i64> for FieldValue<A> {
    fn from(value: i64) -> Self {
        FieldValue::I64(value as u64)
    }
}

impl<A: Allocator + Clone> From<f32> for FieldValue<A> {
    fn from(value: f32) -> Self {
        FieldValue::F32(value)
    }
}

impl<A: Allocator + Clone> From<bool> for FieldValue<A> {
    fn from(value: bool) -> Self {
        FieldValue::Bool(value)
    }
}

impl<A: Allocator + Clone> From<Vec<u8, A>> for FieldValue<A> {
    fn from(value: Vec<u8, A>) -> Self {
        FieldValue::String(AllocString::from(value))
    }
}

impl<A: Allocator + Clone> From<Box<[f32; 3], A>> for FieldValue<A> {
    fn from(value: Box<[f32; 3], A>) -> Self {
        FieldValue::Vec3(value)
    }
}

impl<A: Allocator + Clone> From<Box<[f32; 2], A>> for FieldValue<A> {
    fn from(value: Box<[f32; 2], A>) -> Self {
        FieldValue::Vec2(value)
    }
}

impl<A: Allocator + Clone> From<Box<[f32; 4], A>> for FieldValue<A> {
    fn from(value: Box<[f32; 4], A>) -> Self {
        FieldValue::Vec4(value)
    }
}

// ----

impl<A: Allocator + Clone> ToString for FieldValue<A> {
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

impl<A: Allocator + Clone> std::fmt::Debug for FieldValue<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}
