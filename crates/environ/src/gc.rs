//! Target- and pointer-width-agnostic definitions of GC-related types and
//! constants.
//!
//! These definitions are suitable for use both during compilation and at
//! runtime.
//!
//! Note: We don't bother gating these on `cfg(feature = "gc")` because that
//! makes downstream uses pretty annoying, and the primary thing we want to gate
//! on our various `gc` cargo features is the actual garbage collection
//! functions and their associated impact on binary size anyways.

#[cfg(feature = "gc-drc")]
pub mod drc;

#[cfg(feature = "gc-null")]
pub mod null;

use crate::prelude::*;
use crate::{
    WasmArrayType, WasmCompositeInnerType, WasmCompositeType, WasmStorageType, WasmStructType,
    WasmValType,
};
use core::alloc::Layout;

/// Discriminant to check whether GC reference is an `i31ref` or not.
pub const I31_DISCRIMINANT: u64 = 1;

/// A mask that can be used to check for non-null and non-i31ref GC references
/// with a single bitwise-and operation.
pub const NON_NULL_NON_I31_MASK: u64 = !I31_DISCRIMINANT;

/// The size of the `VMGcHeader` in bytes.
pub const VM_GC_HEADER_SIZE: u32 = 8;

/// The minimum alignment of the `VMGcHeader` in bytes.
pub const VM_GC_HEADER_ALIGN: u32 = 8;

/// The offset of the `VMGcKind` field in the `VMGcHeader`.
pub const VM_GC_HEADER_KIND_OFFSET: u32 = 0;

/// The offset of the `VMSharedTypeIndex` field in the `VMGcHeader`.
pub const VM_GC_HEADER_TYPE_INDEX_OFFSET: u32 = 4;

/// Get the byte size of the given Wasm type when it is stored inside the GC
/// heap.
pub fn byte_size_of_wasm_ty_in_gc_heap(ty: &WasmStorageType) -> u32 {
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

/// Align `offset` up to `bytes`, updating `max_align` if `align` is the
/// new maximum alignment, and returning the aligned offset.
#[cfg(any(feature = "gc-drc", feature = "gc-null"))]
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
#[cfg(any(feature = "gc-drc", feature = "gc-null"))]
fn field(size: &mut u32, align: &mut u32, bytes: u32) -> u32 {
    let offset = align_up(size, align, bytes);
    *size += bytes;
    offset
}

/// Common code to define a GC array's layout, given the size and alignment of
/// the collector's GC header and its expected offset of the array length field.
#[cfg(any(feature = "gc-drc", feature = "gc-null"))]
fn common_array_layout(
    ty: &WasmArrayType,
    header_size: u32,
    header_align: u32,
    expected_array_length_offset: u32,
) -> GcArrayLayout {
    assert!(header_size >= crate::VM_GC_HEADER_SIZE);
    assert!(header_align >= crate::VM_GC_HEADER_ALIGN);

    let mut size = header_size;
    let mut align = header_align;

    let length_field_offset = field(&mut size, &mut align, 4);
    assert_eq!(length_field_offset, expected_array_length_offset);

    let elem_size = byte_size_of_wasm_ty_in_gc_heap(&ty.0.element_type);
    let elems_offset = align_up(&mut size, &mut align, elem_size);
    assert_eq!(elems_offset, size);

    GcArrayLayout {
        base_size: size,
        align,
        elem_size,
    }
}

/// Common code to define a GC struct's layout, given the size and alignment of
/// the collector's GC header and its expected offset of the array length field.
#[cfg(any(feature = "gc-drc", feature = "gc-null"))]
fn common_struct_layout(
    ty: &WasmStructType,
    header_size: u32,
    header_align: u32,
) -> GcStructLayout {
    assert!(header_size >= crate::VM_GC_HEADER_SIZE);
    assert!(header_align >= crate::VM_GC_HEADER_ALIGN);

    // Process each field, aligning it to its natural alignment.
    //
    // We don't try and do any fancy field reordering to minimize padding
    // (yet?) because (a) the toolchain probably already did that and (b)
    // we're just doing the simple thing first. We can come back and improve
    // things here if we find that (a) isn't actually holding true in
    // practice.
    let mut size = header_size;
    let mut align = header_align;

    let fields = ty
        .fields
        .iter()
        .map(|f| {
            let field_size = byte_size_of_wasm_ty_in_gc_heap(&f.element_type);
            field(&mut size, &mut align, field_size)
        })
        .collect();

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

/// A trait for getting the layout of a Wasm GC struct or array inside a
/// particular collector.
pub trait GcTypeLayouts {
    /// The offset of an array's length field.
    ///
    /// This must be the same for all arrays in the heap, regardless of their
    /// element type.
    fn array_length_field_offset(&self) -> u32;

    /// Get this collector's layout for the given composite type.
    ///
    /// Returns `None` if the type is a function type, as functions are not
    /// managed by the GC.
    fn gc_layout(&self, ty: &WasmCompositeType) -> Option<GcLayout> {
        assert!(!ty.shared);
        match &ty.inner {
            WasmCompositeInnerType::Array(ty) => Some(self.array_layout(ty).into()),
            WasmCompositeInnerType::Struct(ty) => Some(self.struct_layout(ty).into()),
            WasmCompositeInnerType::Func(_) => None,
        }
    }

    /// Get this collector's layout for the given array type.
    fn array_layout(&self, ty: &WasmArrayType) -> GcArrayLayout;

    /// Get this collector's layout for the given struct type.
    fn struct_layout(&self, ty: &WasmStructType) -> GcStructLayout;
}

/// The layout of a GC-managed object.
#[derive(Clone, Debug)]
pub enum GcLayout {
    /// The layout of a GC-managed array object.
    Array(GcArrayLayout),

    /// The layout of a GC-managed struct object.
    Struct(GcStructLayout),
}

impl From<GcArrayLayout> for GcLayout {
    fn from(layout: GcArrayLayout) -> Self {
        Self::Array(layout)
    }
}

impl From<GcStructLayout> for GcLayout {
    fn from(layout: GcStructLayout) -> Self {
        Self::Struct(layout)
    }
}

impl GcLayout {
    /// Get the underlying `GcStructLayout`, or panic.
    #[track_caller]
    pub fn unwrap_struct(&self) -> &GcStructLayout {
        match self {
            Self::Struct(s) => s,
            _ => panic!("GcLayout::unwrap_struct on non-struct GC layout"),
        }
    }

    /// Get the underlying `GcArrayLayout`, or panic.
    #[track_caller]
    pub fn unwrap_array(&self) -> &GcArrayLayout {
        match self {
            Self::Array(a) => a,
            _ => panic!("GcLayout::unwrap_array on non-array GC layout"),
        }
    }
}

/// The layout of a GC-managed array.
///
/// This layout is only valid for use with the GC runtime that created it. It is
/// not valid to use one GC runtime's layout with another GC runtime, doing so
/// is memory safe but will lead to general incorrectness like panics and wrong
/// results.
///
/// All offsets are from the start of the object; that is, the size of the GC
/// header (for example) is included in the offset.
///
/// All arrays are composed of the generic `VMGcHeader`, followed by
/// collector-specific fields, followed by the contiguous array elements
/// themselves. The array elements must be aligned to the element type's natural
/// alignment.
#[derive(Clone, Debug)]
pub struct GcArrayLayout {
    /// The size of this array object, without any elements.
    ///
    /// The array's elements, if any, must begin at exactly this offset.
    pub base_size: u32,

    /// The alignment of this array.
    pub align: u32,

    /// The size and natural alignment of each element in this array.
    pub elem_size: u32,
}

impl GcArrayLayout {
    /// Get the total size of this array for a given length of elements.
    #[inline]
    pub fn size_for_len(&self, len: u32) -> u32 {
        self.elem_offset(len)
    }

    /// Get the offset of the `i`th element in an array with this layout.
    #[inline]
    pub fn elem_offset(&self, i: u32) -> u32 {
        self.base_size + i * self.elem_size
    }

    /// Get a `core::alloc::Layout` for an array of this type with the given
    /// length.
    pub fn layout(&self, len: u32) -> Layout {
        let size = self.size_for_len(len);
        let size = usize::try_from(size).unwrap();
        let align = usize::try_from(self.align).unwrap();
        Layout::from_size_align(size, align).unwrap()
    }
}

/// The layout for a GC-managed struct type.
///
/// This layout is only valid for use with the GC runtime that created it. It is
/// not valid to use one GC runtime's layout with another GC runtime, doing so
/// is memory safe but will lead to general incorrectness like panics and wrong
/// results.
///
/// All offsets are from the start of the object; that is, the size of the GC
/// header (for example) is included in the offset.
#[derive(Clone, Debug)]
pub struct GcStructLayout {
    /// The size (in bytes) of this struct.
    pub size: u32,

    /// The alignment (in bytes) of this struct.
    pub align: u32,

    /// The fields of this struct. The `i`th entry is the `i`th struct field's
    /// offset (in bytes) in the struct.
    pub fields: Vec<u32>,
}

impl GcStructLayout {
    /// Get a `core::alloc::Layout` for a struct of this type.
    pub fn layout(&self) -> Layout {
        let size = usize::try_from(self.size).unwrap();
        let align = usize::try_from(self.align).unwrap();
        Layout::from_size_align(size, align).unwrap()
    }
}

/// The kind of an object in a GC heap.
///
/// Note that this type is accessed from Wasm JIT code.
///
/// `VMGcKind` is a bitset where to test if `a` is a subtype of an
/// "abstract-ish" type `b`, we can simply use a single bitwise-and operation:
///
/// ```ignore
/// a <: b   iff   a & b == b
/// ```
///
/// For example, because `VMGcKind::AnyRef` has the high bit set, every kind
/// representing some subtype of `anyref` also has its high bit set.
///
/// We say "abstract-ish" type because in addition to the abstract heap types
/// (other than `i31`) we also have variants for `externref`s that have been
/// converted into an `anyref` via `extern.convert_any` and `externref`s that
/// have been converted into an `anyref` via `any.convert_extern`. Note that in
/// the latter case, because `any.convert_extern $foo` produces a value that is
/// not an instance of `eqref`, `VMGcKind::AnyOfExternRef & VMGcKind::EqRef !=
/// VMGcKind::EqRef`.
///
/// Furthermore, this type only uses the highest 6 bits of its `u32`
/// representation, allowing the lower 27 bytes to be bitpacked with other stuff
/// as users see fit.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[rustfmt::skip]
#[allow(missing_docs, reason = "self-describing variants")]
pub enum VMGcKind {
    ExternRef      = 0b01000 << 27,
    AnyRef         = 0b10000 << 27,
    EqRef          = 0b10100 << 27,
    ArrayRef       = 0b10101 << 27,
    StructRef      = 0b10110 << 27,
}

impl VMGcKind {
    /// Mask this value with a `u32` to get just the bits that `VMGcKind` uses.
    pub const MASK: u32 = 0b11111 << 27;

    /// Mask this value with a `u32` that potentially contains a `VMGcKind` to
    /// get the bits that `VMGcKind` doesn't use.
    pub const UNUSED_MASK: u32 = !Self::MASK;

    /// Does the given value fit in the unused bits of a `VMGcKind`?
    #[inline]
    pub fn value_fits_in_unused_bits(value: u32) -> bool {
        (value & Self::UNUSED_MASK) == value
    }

    /// Convert the given value into a `VMGcKind` by masking off the unused
    /// bottom bits.
    #[inline]
    pub fn from_high_bits_of_u32(val: u32) -> VMGcKind {
        let masked = val & Self::MASK;
        match masked {
            x if x == Self::ExternRef.as_u32() => Self::ExternRef,
            x if x == Self::AnyRef.as_u32() => Self::AnyRef,
            x if x == Self::EqRef.as_u32() => Self::EqRef,
            x if x == Self::ArrayRef.as_u32() => Self::ArrayRef,
            x if x == Self::StructRef.as_u32() => Self::StructRef,
            _ => panic!("invalid `VMGcKind`: {masked:#032b}"),
        }
    }

    /// Does this kind match the other kind?
    ///
    /// That is, is this kind a subtype of the other kind?
    #[inline]
    pub fn matches(self, other: Self) -> bool {
        (self.as_u32() & other.as_u32()) == other.as_u32()
    }

    /// Get this `VMGcKind` as a raw `u32`.
    #[inline]
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

#[cfg(test)]
mod tests {
    use super::VMGcKind::*;
    use crate::prelude::*;

    #[test]
    fn kind_matches() {
        let all = [ExternRef, AnyRef, EqRef, ArrayRef, StructRef];

        for (sup, subs) in [
            (ExternRef, vec![]),
            (AnyRef, vec![EqRef, ArrayRef, StructRef]),
            (EqRef, vec![ArrayRef, StructRef]),
            (ArrayRef, vec![]),
            (StructRef, vec![]),
        ] {
            assert!(sup.matches(sup));
            for sub in &subs {
                assert!(sub.matches(sup));
            }
            for kind in all.iter().filter(|k| **k != sup && !subs.contains(k)) {
                assert!(!kind.matches(sup));
            }
        }
    }
}
