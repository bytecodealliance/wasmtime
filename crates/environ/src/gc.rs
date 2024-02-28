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

/// Discriminant to check whether GC reference is an `i31ref` or not.
pub const I31_DISCRIMINANT: u64 = 1;

/// A mask that can be used to check for non-null and non-i31ref GC references
/// with a single bitwise-and operation.
pub const NON_NULL_NON_I31_MASK: u64 = !I31_DISCRIMINANT;

/// The kind of an object in a GC heap.
///
/// This is accessed from Wasm JIT code.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VMGcKind {
    /// An `externref` holding some kind of host data.
    ExternRef = 0b00 << 30,
    //
    // When we support more GC types, we will complete this type with:
    //
    // /// An `anyref` (or one of its subtypes, other than `i31ref`).
    // AnyRef = 0b01 << 30,
    //
    // /// An `anyref` that was wrapped into an `externref` via
    // /// `extern.convert_any`.
    // ExternOfAnyRef = 0b10 << 30,
    //
    // /// An `externref` that was wrapped into an `anyref` via
    // /// `any.convert_extern`.
    // AnyOfExternRef = 0b11 << 30,
}

impl VMGcKind {
    /// Mask this value with a `u32` to turn it into a valid `VMGcKind`.
    pub const MASK: u32 = 0b11 << 30;

    /// Mask this value with a `u32` that potentially contains a `VMGcKind` to
    /// get the bits that `VMGcKind` doesn't use.
    pub const UNUSED_MASK: u32 = !Self::MASK;

    /// Convert the given value into a `VMGcKind` by masking off the unused
    /// bits.
    pub const fn from_u32(val: u32) -> VMGcKind {
        let masked = val & Self::MASK;
        assert!(
            masked == Self::ExternRef as u32,
            "not all masked bit patterns are valid `VMGcKind`s yet"
        );
        Self::ExternRef
    }
}
