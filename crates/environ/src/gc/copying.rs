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

/// The layout of Wasm GC objects in the copying collector.
#[derive(Default)]
pub struct CopyingTypeLayouts;

/// The inline trace info encoded in a `VMCopyingHeader`'s reserved bits.
#[derive(Clone, Copy, Debug)]
pub enum InlineTraceInfo {
    /// The trace info is not encoded inline. Look it up from `TraceInfos`.
    OutOfLine,

    /// Inline trace info for an array type.
    Array {
        /// Whether the array's elements are GC references that need tracing.
        elems_are_gc_refs: bool,
    },

    /// Inline trace info for a struct, exception, or externref type.
    Struct {
        /// A bitmap where the `i`th bit is set iff the `i`th `u32` in the
        /// object's data (after the header) is a `VMGcRef` that needs
        /// tracing. Only the lower 23 bits are meaningful.
        gc_ref_bitmap: u32,
    },
}

impl InlineTraceInfo {
    const IS_INLINE_BIT: u32 = 1 << 1;
    const IS_ARRAY_BIT: u32 = 1 << 2;
    const ELEMS_ARE_GC_REFS_BIT: u32 = 1 << 3;
    const STRUCT_BITMAP_SHIFT: u32 = 3;
    const STRUCT_BITMAP_BITS: u32 = 23;

    /// Inline trace info for an object with no GC-reference edges (e.g.
    /// externrefs). This is a `Struct` with an empty bitmap.
    pub const NO_GC_EDGES: InlineTraceInfo = InlineTraceInfo::Struct { gc_ref_bitmap: 0 };

    /// Create inline trace info for the given GC layout.
    pub fn new(layout: &GcLayout) -> Self {
        match layout {
            GcLayout::Array(a) => Self::array(a),
            GcLayout::Struct(s) => Self::r#struct(s),
        }
    }

    /// Create inline trace info for an array type.
    ///
    /// Arrays can always be represented inline (only one bit of payload is
    /// needed), so this never returns `OutOfLine`.
    pub fn array(layout: &GcArrayLayout) -> Self {
        InlineTraceInfo::Array {
            elems_are_gc_refs: layout.elems_are_gc_refs,
        }
    }

    /// Create inline trace info for a struct or exception type.
    ///
    /// Returns `OutOfLine` only when there is a GC-reference field whose
    /// offset cannot be represented in the 23-bit bitmap. Non-GC-reference
    /// fields are ignored regardless of offset.
    pub fn r#struct(layout: &GcStructLayout) -> Self {
        let mut bitmap: u32 = 0;
        for f in layout.fields.iter() {
            if !f.is_gc_ref {
                continue;
            }
            let Some(data_offset) = f.offset.checked_sub(HEADER_SIZE) else {
                return Self::OutOfLine;
            };
            if data_offset % 4 != 0 {
                return Self::OutOfLine;
            }
            let slot = data_offset / 4;
            if slot >= Self::STRUCT_BITMAP_BITS {
                return Self::OutOfLine;
            }
            bitmap |= 1u32 << slot;
        }
        InlineTraceInfo::Struct {
            gc_ref_bitmap: bitmap,
        }
    }

    /// Encode this inline trace info as its bit-packed representation for
    /// storage in a `VMGcHeader`'s reserved bits.
    pub fn encode(&self) -> u32 {
        match self {
            Self::OutOfLine => 0,
            Self::Array { elems_are_gc_refs } => {
                Self::IS_INLINE_BIT
                    | Self::IS_ARRAY_BIT
                    | if *elems_are_gc_refs {
                        Self::ELEMS_ARE_GC_REFS_BIT
                    } else {
                        0
                    }
            }
            Self::Struct { gc_ref_bitmap } => {
                Self::IS_INLINE_BIT | (*gc_ref_bitmap << Self::STRUCT_BITMAP_SHIFT)
            }
        }
    }

    /// Decode inline trace info from the reserved bits of a `VMGcHeader`.
    pub fn decode(reserved: u32) -> Self {
        if reserved & Self::IS_INLINE_BIT == 0 {
            return Self::OutOfLine;
        }
        if reserved & Self::IS_ARRAY_BIT != 0 {
            Self::Array {
                elems_are_gc_refs: reserved & Self::ELEMS_ARE_GC_REFS_BIT != 0,
            }
        } else {
            Self::Struct {
                gc_ref_bitmap: (reserved >> Self::STRUCT_BITMAP_SHIFT)
                    & ((1u32 << Self::STRUCT_BITMAP_BITS) - 1),
            }
        }
    }
}

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

    fn struct_layout(&self, ty: &WasmStructType) -> Result<GcStructLayout, OutOfMemory> {
        let mut layout = common_struct_layout(ty, HEADER_SIZE, ALIGN)?;
        // Ensure there is always space for the forwarding reference, even if
        // the struct has no fields.
        if layout.size < MIN_OBJECT_SIZE {
            layout.size = MIN_OBJECT_SIZE;
        }
        layout.size = layout.size.next_multiple_of(ALIGN);
        debug_assert!(layout.align <= ALIGN);
        layout.align = ALIGN;
        debug_assert!(layout.size >= MIN_OBJECT_SIZE);
        Ok(layout)
    }

    fn exn_layout(&self, ty: &WasmExnType) -> Result<GcStructLayout, OutOfMemory> {
        let mut layout = common_exn_layout(ty, HEADER_SIZE, ALIGN)?;
        layout.size = layout.size.next_multiple_of(ALIGN);
        debug_assert!(layout.align <= ALIGN);
        layout.align = ALIGN;
        debug_assert!(layout.size >= MIN_OBJECT_SIZE);
        Ok(layout)
    }
}
