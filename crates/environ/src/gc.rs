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
/// have been convered into an `anyref` via `any.convert_extern`. Note that in
/// the latter case, because `any.convert_extern $foo` produces a value that is
/// not an instance of `eqref`, `VMGcKind::AnyOfExternRef & VMGcKind::EqRef !=
/// VMGcKind::EqRef`.
///
/// Furthermore, this type only uses the highest 6 bits of its `u32`
/// representation, allowing the lower 26 bytes to be bitpacked with other stuff
/// as users see fit.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[rustfmt::skip]
#[allow(missing_docs)]
pub enum VMGcKind {
    ExternRef      = 0b010000 << 26,
    ExternOfAnyRef = 0b011000 << 26,
    AnyRef         = 0b100000 << 26,
    AnyOfExternRef = 0b100100 << 26,
    EqRef          = 0b101000 << 26,
    ArrayRef       = 0b101001 << 26,
    StructRef      = 0b101010 << 26,
}

impl VMGcKind {
    /// Mask this value with a `u32` to get just the bits that `VMGcKind` uses.
    pub const MASK: u32 = 0b111111 << 26;

    /// Mask this value with a `u32` that potentially contains a `VMGcKind` to
    /// get the bits that `VMGcKind` doesn't use.
    pub const UNUSED_MASK: u32 = !Self::MASK;

    /// Convert the given value into a `VMGcKind` by masking off the unused
    /// bottom bits.
    pub fn from_high_bits_of_u32(val: u32) -> VMGcKind {
        let masked = val & Self::MASK;
        match masked {
            x if x == Self::ExternRef as u32 => Self::ExternRef,
            x if x == Self::ExternOfAnyRef as u32 => Self::ExternOfAnyRef,
            x if x == Self::AnyRef as u32 => Self::AnyRef,
            x if x == Self::AnyOfExternRef as u32 => Self::AnyOfExternRef,
            x if x == Self::EqRef as u32 => Self::EqRef,
            x if x == Self::ArrayRef as u32 => Self::ArrayRef,
            x if x == Self::StructRef as u32 => Self::StructRef,
            _ => panic!("invalid `VMGcKind`: {masked:#032b}"),
        }
    }

    /// Does this kind match the other kind?
    ///
    /// That is, is this kind a subtype of the other kind?
    pub fn matches(self, other: Self) -> bool {
        (self as u32) & (other as u32) == (other as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::VMGcKind::*;
    use crate::prelude::*;

    #[test]
    fn kind_matches() {
        let all = [
            ExternRef,
            ExternOfAnyRef,
            AnyRef,
            AnyOfExternRef,
            EqRef,
            ArrayRef,
            StructRef,
        ];

        for (sup, subs) in [
            (ExternRef, vec![ExternOfAnyRef]),
            (ExternOfAnyRef, vec![]),
            (AnyRef, vec![AnyOfExternRef, EqRef, ArrayRef, StructRef]),
            (AnyOfExternRef, vec![]),
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
