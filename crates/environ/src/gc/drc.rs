//! Layout of Wasm GC objects in the deferred reference-counting collector.

use super::*;
use wasmtime_types::{WasmStorageType, WasmValType};

/// The size of the `VMDrcHeader` header for GC objects.
pub const HEADER_SIZE: u32 = 16;

/// The align of the `VMDrcHeader` header for GC objects.
pub const HEADER_ALIGN: u32 = 8;

/// The offset of the length field in a `VMDrcArrayHeader`.
pub const ARRAY_LENGTH_OFFSET: u32 = HEADER_SIZE;

/// Align `offset` up to `bytes`, updating `max_align` if `align` is the
/// new maximum alignment, and returning the aligned offset.
fn align_up(offset: &mut u32, max_align: &mut u32, align: u32) -> u32 {
    debug_assert!(max_align.is_power_of_two());
    debug_assert!(align.is_power_of_two());
    *offset = offset.checked_add(align - 1).unwrap() & !(align - 1);
    *max_align = core::cmp::max(*max_align, align);
    *offset
}

/// Define a new field of size and alignment `bytes`, updating the object's
/// total `size` and `align` as necessary. The offset of the new field is
/// returned.
fn field(size: &mut u32, align: &mut u32, bytes: u32) -> u32 {
    let offset = align_up(size, align, bytes);
    *size += bytes;
    offset
}

fn size_of_wasm_ty(ty: &WasmStorageType) -> u32 {
    match ty {
        WasmStorageType::I8 => 1,
        WasmStorageType::I16 => 2,
        WasmStorageType::Val(ty) => match ty {
            WasmValType::I32 | WasmValType::F32 | WasmValType::Ref(_) => 4,
            WasmValType::I64 | WasmValType::F64 => 8,
            WasmValType::V128 => 16,
        },
    }
}

/// The layout of Wasm GC objects in the deferred reference-counting collector.
#[derive(Default)]
pub struct DrcTypeLayouts;

impl GcTypeLayouts for DrcTypeLayouts {
    fn array_layout(&self, ty: &WasmArrayType) -> GcArrayLayout {
        let mut size = HEADER_SIZE;
        let mut align = HEADER_ALIGN;

        let length_field_offset = field(&mut size, &mut align, 4);
        debug_assert_eq!(length_field_offset, ARRAY_LENGTH_OFFSET);

        let elem_size = size_of_wasm_ty(&ty.0.element_type);
        let elems_offset = align_up(&mut size, &mut align, elem_size);

        GcArrayLayout {
            size,
            align,
            length_field_offset,
            elems_offset,
            elem_size,
        }
    }

    fn struct_layout(&self, ty: &WasmStructType) -> GcStructLayout {
        // Process each field, aligning it to its natural alignment.
        //
        // We don't try and do any fancy field reordering to minimize padding
        // (yet?) because (a) the toolchain probably already did that and (b)
        // we're just doing the simple thing first. We can come back and improve
        // things here if we find that (a) isn't actually holding true in
        // practice.
        let mut size = HEADER_SIZE;
        let mut align = HEADER_ALIGN;

        let fields = ty
            .fields
            .iter()
            .map(|f| {
                let field_size = size_of_wasm_ty(&f.element_type);
                field(&mut size, &mut align, field_size)
            })
            .collect();

        // Ensure that the final size is a multiple of the alignment, for
        // simplicity.
        align_up(&mut size, &mut 16, align);

        GcStructLayout {
            size,
            align,
            fields,
        }
    }
}
