//! Layout of Wasm GC objects in the copying garbage collector.

use super::*;
use core::mem;

/// The size of the `VMCopyingHeader` header for GC objects.
///
/// Note: This is 16 (not 12) because `VMGcHeader` has `align(8)`, so the
/// `repr(C)` struct has 4 bytes of trailing padding after the `object_size`
/// field.
pub const HEADER_SIZE: u32 = 16;

/// The alignment of all GC objects in the copying collector.
///
/// All objects and all layouts must be a multiple of this alignment.
pub const ALIGN: u32 = 16;

/// The offset of the length field in a `VMCopyingArrayHeader`.
pub const ARRAY_LENGTH_OFFSET: u32 = HEADER_SIZE;

/// The offset of the tag-instance-index field in an exception header.
pub const EXCEPTION_TAG_INSTANCE_OFFSET: u32 = HEADER_SIZE;

/// The offset of the tag-defined-index field in an exception header.
pub const EXCEPTION_TAG_DEFINED_OFFSET: u32 = HEADER_SIZE + 4;

/// The bit within a `VMCopyingHeader`'s reserved bits that represents whether,
/// during a collection, the object has already been copied into the new
/// semi-space.
pub const HEADER_COPIED_BIT: u32 = 1 << 0;

/// The offset within this GC object, which must have the `HEADER_COPIED_BIT`
/// set and must reside within the old semi-space, where the new copy of this
/// object is located within the new semi-space.
pub const FORWARDING_REF_OFFSET: u32 = HEADER_SIZE;

/// The minimum object size: every object must have enough room for the
/// forwarding reference that the copying collector writes during collection.
pub const MIN_OBJECT_SIZE: u32 = FORWARDING_REF_OFFSET + mem::size_of::<u32>() as u32;

/// Round `size` up to a multiple of `ALIGN`.
fn align_up(size: u32) -> u32 {
    (size + ALIGN - 1) & !(ALIGN - 1)
}

/// The layout of Wasm GC objects in the copying collector.
#[derive(Default)]
pub struct CopyingTypeLayouts;

impl GcTypeLayouts for CopyingTypeLayouts {
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
        let mut layout = common_array_layout(ty, HEADER_SIZE, ALIGN, ARRAY_LENGTH_OFFSET);
        debug_assert!(layout.align <= ALIGN);
        layout.align = ALIGN;
        debug_assert!(layout.base_size >= MIN_OBJECT_SIZE);
        layout
    }

    fn struct_layout(&self, ty: &WasmStructType) -> GcStructLayout {
        let mut layout = common_struct_layout(ty, HEADER_SIZE, ALIGN);
        // Ensure there is always space for the forwarding reference, even if
        // the struct has no fields.
        if layout.size < MIN_OBJECT_SIZE {
            layout.size = MIN_OBJECT_SIZE;
        }
        layout.size = align_up(layout.size);
        debug_assert!(layout.align <= ALIGN);
        layout.align = ALIGN;
        debug_assert!(layout.size >= MIN_OBJECT_SIZE);
        layout
    }

    fn exn_layout(&self, ty: &WasmExnType) -> GcStructLayout {
        let mut layout = common_exn_layout(ty, HEADER_SIZE, ALIGN);
        layout.size = align_up(layout.size);
        debug_assert!(layout.align <= ALIGN);
        layout.align = ALIGN;
        debug_assert!(layout.size >= MIN_OBJECT_SIZE);
        layout
    }
}
