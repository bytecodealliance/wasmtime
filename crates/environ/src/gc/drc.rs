//! Layout of Wasm GC objects in the deferred reference-counting collector.

use super::*;
use core::cmp;

/// The size of the `VMDrcHeader` header for GC objects.
pub const HEADER_SIZE: u32 = 24;

/// The align of the `VMDrcHeader` header for GC objects.
pub const HEADER_ALIGN: u32 = 8;

/// The offset of the length field in a `VMDrcArrayHeader`.
pub const ARRAY_LENGTH_OFFSET: u32 = HEADER_SIZE;

/// The layout of Wasm GC objects in the deferred reference-counting collector.
#[derive(Default)]
pub struct DrcTypeLayouts;

impl GcTypeLayouts for DrcTypeLayouts {
    fn array_length_field_offset(&self) -> u32 {
        ARRAY_LENGTH_OFFSET
    }

    fn array_layout(&self, ty: &WasmArrayType) -> GcArrayLayout {
        common_array_layout(ty, HEADER_SIZE, HEADER_ALIGN, ARRAY_LENGTH_OFFSET)
    }

    fn struct_layout(&self, ty: &WasmStructType) -> GcStructLayout {
        // Sort the struct fields into the order in which we will lay them out.
        //
        // NB: we put all GC refs first so that we can simply store the number
        // of GC refs in any object in the `VMDrcHeader` and then uniformly
        // trace all structs types.
        let mut fields: Vec<_> = ty.fields.iter().enumerate().collect();
        fields.sort_by_key(|(i, f)| {
            let is_gc_ref = f.element_type.is_vmgcref_type_and_not_i31();
            let size = byte_size_of_wasm_ty_in_gc_heap(&f.element_type);
            (cmp::Reverse(is_gc_ref), cmp::Reverse(size), *i)
        });

        // Compute the offset of each field as well as the size and alignment of
        // the whole struct.
        let mut size = HEADER_SIZE;
        let mut align = HEADER_ALIGN;
        let mut fields: Vec<_> = fields
            .into_iter()
            .map(|(i, f)| {
                let field_size = byte_size_of_wasm_ty_in_gc_heap(&f.element_type);
                let offset = field(&mut size, &mut align, field_size);
                let is_gc_ref = f.element_type.is_vmgcref_type_and_not_i31();
                (i, GcStructLayoutField { offset, is_gc_ref })
            })
            .collect();
        if let Some((_i, f)) = fields.get(0) {
            if f.is_gc_ref {
                debug_assert_eq!(
                    f.offset, HEADER_SIZE,
                    "GC refs should come directly after the header, without any padding",
                );
            }
        }

        // Re-sort the fields into their definition (rather than layout) order
        // and throw away the definition index.
        fields.sort_by_key(|(i, _f)| *i);
        let fields: Vec<_> = fields.into_iter().map(|(_i, f)| f).collect();

        // Ensure that the final size is a multiple of the alignment, for
        // simplicity.
        let align_size_to = align;
        align_up(&mut size, &mut align, align_size_to);

        GcStructLayout {
            size,
            align,
            fields,
        }
    }
}
