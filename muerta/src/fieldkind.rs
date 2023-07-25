// FieldKind represents only non-primitive kinds
// NOTE: Clone is derived because FlattenedSerializerField needs to be clonable.
#[derive(Clone)]
pub enum FieldKind {
    // TODO: rename into FixedArray into FixedPrimitiveArray, etc.?
    FixedArray { size: usize },
    DynamicArray, // or Vec on rust's terms, Dynamic* just fits nicer..
    FixedTable { size: usize },
    DynamicTable,
    // Pointer,
}
