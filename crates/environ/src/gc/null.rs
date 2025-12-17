//! Layout of Wasm GC objects in the  null garbage collector.

use super::*;

/// The size of the `VMNullHeader` header for GC objects.
pub const HEADER_SIZE: u32 = 8;

/// The align of the `VMNullHeader` header for GC objects.
pub const HEADER_ALIGN: u32 = 8;

/// The offset of the length field in a `VMNullArrayHeader`.
pub const ARRAY_LENGTH_OFFSET: u32 = HEADER_SIZE;

/// The offset of the tag-instance-index field in an exception header.
pub const EXCEPTION_TAG_INSTANCE_OFFSET: u32 = HEADER_SIZE;

/// The offset of the tag-defined-index field in an exception header.
pub const EXCEPTION_TAG_DEFINED_OFFSET: u32 = HEADER_SIZE + 4;

/// The layout of Wasm GC objects in the null collector.
#[derive(Default)]
pub struct NullTypeLayouts;

impl GcTypeLayouts for NullTypeLayouts {
    fn array_length_field_offset(&self) -> u32 {
        ARRAY_LENGTH_OFFSET
    }

    fn exception_tag_instance_offset(&self) -> u32 {
        EXCEPTION_TAG_INSTANCE_OFFSET
    }

    fn exception_tag_defined_offset(&self) -> u32 {
        EXCEPTION_TAG_DEFINED_OFFSET
    }

    fn array_layout(&self, ty: &WasmArrayType) -> GcArrayLayout {
        common_array_layout(ty, HEADER_SIZE, HEADER_ALIGN, ARRAY_LENGTH_OFFSET)
    }

    fn struct_layout(&self, ty: &WasmStructType) -> GcStructLayout {
        common_struct_layout(ty, HEADER_SIZE, HEADER_ALIGN)
    }

    fn exn_layout(&self, ty: &WasmExnType) -> GcStructLayout {
        common_exn_layout(ty, HEADER_SIZE, HEADER_ALIGN)
    }
}
