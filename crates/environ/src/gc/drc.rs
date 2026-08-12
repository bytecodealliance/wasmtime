//! Layout of Wasm GC objects in the deferred reference-counting collector.

use super::*;

/// The size of the `VMDrcHeader` header for GC objects.
pub const HEADER_SIZE: u32 = 24;

/// The align of the `VMDrcHeader` header for GC objects.
pub const HEADER_ALIGN: u32 = 8;

/// The offset of the length field in a `VMDrcArrayHeader`.
pub const ARRAY_LENGTH_OFFSET: u32 = HEADER_SIZE;

/// The offset of the tag-instance-index field in an exception header.
pub const EXCEPTION_TAG_INSTANCE_OFFSET: u32 = HEADER_SIZE;

/// The offset of the tag-defined-index field in an exception header.
pub const EXCEPTION_TAG_DEFINED_OFFSET: u32 = HEADER_SIZE + 4;

pub use super::DRC_HEADER_IN_OVER_APPROX_LIST_BIT as HEADER_IN_OVER_APPROX_LIST_BIT;
pub use super::DRC_HEADER_MARK_BIT as HEADER_MARK_BIT;
pub use super::DRC_MIN_OVER_APPROX_STACK_ROOTS_GC_THRESHOLD as MIN_OVER_APPROX_STACK_ROOTS_GC_THRESHOLD;

/// The layout of Wasm GC objects in the deferred reference-counting collector.
#[derive(Default)]
pub struct DrcTypeLayouts;

impl GcTypeLayouts for DrcTypeLayouts {
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

    fn struct_layout(&self, ty: &WasmStructType) -> Result<GcStructLayout, OutOfMemory> {
        common_struct_layout(ty, HEADER_SIZE, HEADER_ALIGN)
    }

    fn exn_layout(&self, ty: &WasmExnType) -> Result<GcStructLayout, OutOfMemory> {
        common_exn_layout(ty, HEADER_SIZE, HEADER_ALIGN)
    }
}
