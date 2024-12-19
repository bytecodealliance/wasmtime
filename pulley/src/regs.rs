//! Pulley registers.

use crate::U6;
use core::hash::Hash;
use core::marker::PhantomData;
use core::{fmt, ops::Range};

use cranelift_bitset::ScalarBitSet;

/// Trait for common register operations.
pub trait Reg: Sized + Copy + Eq + Ord + Hash + Into<AnyReg> + fmt::Debug + fmt::Display {
    /// Range of valid register indices.
    const RANGE: Range<u8>;

    /// Convert a register index to a register, without bounds checking.
    unsafe fn new_unchecked(index: u8) -> Self;

    /// Convert a register index to a register, with bounds checking.
    fn new(index: u8) -> Option<Self> {
        if Self::RANGE.contains(&index) {
            Some(unsafe { Self::new_unchecked(index) })
        } else {
            None
        }
    }

    /// Convert a register to its index.
    fn to_u8(self) -> u8;

    /// Convert a register to its index.
    fn index(self) -> usize {
        self.to_u8().into()
    }
}

macro_rules! impl_reg {
    ($reg_ty:ty, $any:ident, $range:expr) => {
        impl From<$reg_ty> for AnyReg {
            fn from(r: $reg_ty) -> Self {
                AnyReg::$any(r)
            }
        }

        impl fmt::Display for $reg_ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self, f)
            }
        }

        impl Reg for $reg_ty {
            const RANGE: Range<u8> = $range;

            unsafe fn new_unchecked(index: u8) -> Self {
                core::mem::transmute(index)
            }

            fn to_u8(self) -> u8 {
                self as u8
            }
        }
    };
}

/// An `x` register: integers.
#[repr(u8)]
#[derive(Debug,Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[allow(non_camel_case_types, missing_docs)]
#[rustfmt::skip]
pub enum XReg {
    x0,  x1,  x2,  x3,  x4,  x5,  x6,  x7,  x8,  x9,
    x10, x11, x12, x13, x14, x15, x16, x17, x18, x19,
    x20, x21, x22, x23, x24, x25, x26, x27, x28, x29,

    /// The special `sp` stack pointer register.
    sp,

    /// The special `spilltmp0` scratch register.
    spilltmp0,

}

impl XReg {
    /// Index of the first "special" register.
    pub const SPECIAL_START: u8 = XReg::sp as u8;

    /// Is this `x` register a special register?
    pub fn is_special(self) -> bool {
        matches!(self, Self::sp | Self::spilltmp0)
    }
}

#[test]
fn assert_special_start_is_right() {
    for i in 0..XReg::SPECIAL_START {
        assert!(!XReg::new(i).unwrap().is_special());
    }
    for i in XReg::SPECIAL_START.. {
        match XReg::new(i) {
            Some(r) => assert!(r.is_special()),
            None => break,
        }
    }
}

/// An `f` register: floats.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[allow(non_camel_case_types, missing_docs)]
#[rustfmt::skip]
pub enum FReg {
    f0,  f1,  f2,  f3,  f4,  f5,  f6,  f7,  f8,  f9,
    f10, f11, f12, f13, f14, f15, f16, f17, f18, f19,
    f20, f21, f22, f23, f24, f25, f26, f27, f28, f29,
    f30, f31,
}

/// A `v` register: vectors.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[allow(non_camel_case_types, missing_docs)]
#[rustfmt::skip]
pub enum VReg {
    v0,  v1,  v2,  v3,  v4,  v5,  v6,  v7,  v8,  v9,
    v10, v11, v12, v13, v14, v15, v16, v17, v18, v19,
    v20, v21, v22, v23, v24, v25, v26, v27, v28, v29,
    v30, v31,
}

impl_reg!(XReg, X, 0..32);
impl_reg!(FReg, F, 0..32);
impl_reg!(VReg, V, 0..32);

/// Any register, regardless of class.
///
/// Never appears inside an instruction -- instructions always name a particular
/// class of register -- but this is useful for testing and things like that.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum AnyReg {
    X(XReg),
    F(FReg),
    V(VReg),
}

impl fmt::Display for AnyReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl fmt::Debug for AnyReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AnyReg::X(r) => fmt::Debug::fmt(r, f),
            AnyReg::F(r) => fmt::Debug::fmt(r, f),
            AnyReg::V(r) => fmt::Debug::fmt(r, f),
        }
    }
}

/// Operands to a binary operation, packed into a 16-bit word (5 bits per register).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct BinaryOperands<D, S1 = D, S2 = D> {
    /// The destination register, packed in bits 0..5.
    pub dst: D,
    /// The first source register, packed in bits 5..10.
    pub src1: S1,
    /// The second source register, packed in bits 10..15.
    pub src2: S2,
}

impl<D, S1, S2> BinaryOperands<D, S1, S2> {
    /// Convenience constructor for applying `Into`
    pub fn new(dst: impl Into<D>, src1: impl Into<S1>, src2: impl Into<S2>) -> Self {
        Self {
            dst: dst.into(),
            src1: src1.into(),
            src2: src2.into(),
        }
    }
}

impl<D: Reg, S1: Reg, S2: Reg> BinaryOperands<D, S1, S2> {
    /// Convert to dense 16 bit encoding.
    pub fn to_bits(self) -> u16 {
        let dst = self.dst.to_u8();
        let src1 = self.src1.to_u8();
        let src2 = self.src2.to_u8();
        (dst as u16) | ((src1 as u16) << 5) | ((src2 as u16) << 10)
    }

    /// Convert from dense 16 bit encoding. The topmost bit is ignored.
    pub fn from_bits(bits: u16) -> Self {
        Self {
            dst: D::new((bits & 0b11111) as u8).unwrap(),
            src1: S1::new(((bits >> 5) & 0b11111) as u8).unwrap(),
            src2: S2::new(((bits >> 10) & 0b11111) as u8).unwrap(),
        }
    }
}

impl<D: Reg, S1: Reg> BinaryOperands<D, S1, U6> {
    /// Convert to dense 16 bit encoding.
    pub fn to_bits(self) -> u16 {
        let dst = self.dst.to_u8();
        let src1 = self.src1.to_u8();
        let src2 = u8::from(self.src2);
        (dst as u16) | ((src1 as u16) << 5) | ((src2 as u16) << 10)
    }

    /// Convert from dense 16 bit encoding. The topmost bit is ignored.
    pub fn from_bits(bits: u16) -> Self {
        Self {
            dst: D::new((bits & 0b11111) as u8).unwrap(),
            src1: S1::new(((bits >> 5) & 0b11111) as u8).unwrap(),
            src2: U6::new(((bits >> 10) & 0b111111) as u8).unwrap(),
        }
    }
}

/// A set of registers, packed into a 32-bit bitset.
pub struct RegSet<R> {
    bitset: ScalarBitSet<u32>,
    phantom: PhantomData<R>,
}

impl<R: Reg> RegSet<R> {
    /// Create a `RegSet` from a `ScalarBitSet`.
    pub fn from_bitset(bitset: ScalarBitSet<u32>) -> Self {
        Self {
            bitset,
            phantom: PhantomData,
        }
    }

    /// Convert a `RegSet` into a `ScalarBitSet`.
    pub fn to_bitset(self) -> ScalarBitSet<u32> {
        self.bitset
    }
}

impl<R: Reg> From<ScalarBitSet<u32>> for RegSet<R> {
    fn from(bitset: ScalarBitSet<u32>) -> Self {
        Self {
            bitset,
            phantom: PhantomData,
        }
    }
}

impl<R: Reg> Into<ScalarBitSet<u32>> for RegSet<R> {
    fn into(self) -> ScalarBitSet<u32> {
        self.bitset
    }
}

impl<R: Reg> IntoIterator for RegSet<R> {
    type Item = R;
    type IntoIter = core::iter::FilterMap<cranelift_bitset::scalar::Iter<u32>, fn(u8) -> Option<R>>;

    fn into_iter(self) -> Self::IntoIter {
        self.bitset.into_iter().filter_map(R::new)
    }
}

impl<R: Reg> FromIterator<R> for RegSet<R> {
    fn from_iter<I: IntoIterator<Item = R>>(iter: I) -> Self {
        let mut set = ScalarBitSet::new();
        for reg in iter {
            set.insert(reg.to_u8());
        }
        RegSet::from(set)
    }
}

impl<R: Reg> Default for RegSet<R> {
    fn default() -> Self {
        Self {
            bitset: Default::default(),
            phantom: Default::default(),
        }
    }
}

impl<R: Reg> Copy for RegSet<R> {}
impl<R: Reg> Clone for RegSet<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: Reg> PartialEq for RegSet<R> {
    fn eq(&self, other: &Self) -> bool {
        self.bitset == other.bitset
    }
}
impl<R: Reg> Eq for RegSet<R> {}

impl<R: Reg> fmt::Debug for RegSet<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.into_iter()).finish()
    }
}

#[cfg(feature = "arbitrary")]
impl<'a, R: Reg> arbitrary::Arbitrary<'a> for RegSet<R> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        ScalarBitSet::arbitrary(u).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_x_regs() {
        assert!(XReg::sp.is_special());
        assert!(XReg::spilltmp0.is_special());
    }

    #[test]
    fn not_special_x_regs() {
        for i in 0..27 {
            assert!(!XReg::new(i).unwrap().is_special());
        }
    }

    #[test]
    fn binary_operands() {
        let mut i = 0;
        for src2 in XReg::RANGE {
            for src1 in XReg::RANGE {
                for dst in XReg::RANGE {
                    let operands = BinaryOperands {
                        dst: XReg::new(dst).unwrap(),
                        src1: XReg::new(src1).unwrap(),
                        src2: XReg::new(src2).unwrap(),
                    };
                    assert_eq!(operands.to_bits(), i);
                    assert_eq!(BinaryOperands::<XReg>::from_bits(i), operands);
                    assert_eq!(BinaryOperands::<XReg>::from_bits(0x8000 | i), operands);
                    i += 1;
                }
            }
        }
    }
}
