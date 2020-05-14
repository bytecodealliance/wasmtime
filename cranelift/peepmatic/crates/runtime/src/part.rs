//! Parts of instructions.

use crate::cc::ConditionCode;
use crate::r#type::BitWidth;
use std::fmt::Debug;

/// A constant value.
///
/// Whether an integer is interpreted as signed or unsigned depends on the
/// operations applied to it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Constant {
    /// A boolean of the given width.
    Bool(bool, BitWidth),

    /// An integer constant of the given width,
    Int(u64, BitWidth),
}

/// A part of an instruction, or a whole instruction itself.
///
/// These are the different values that can be matched in an optimization's
/// left-hand side and then built up in its right-hand side.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Part<I>
where
    I: Copy + Debug + Eq,
{
    /// An instruction (or result of an instruction).
    Instruction(I),

    /// A constant value.
    Constant(Constant),

    /// A condition code.
    ConditionCode(ConditionCode),
}

impl<I> From<Constant> for Part<I>
where
    I: Copy + Debug + Eq,
{
    #[inline]
    fn from(c: Constant) -> Part<I> {
        Part::Constant(c)
    }
}

impl<I> From<ConditionCode> for Part<I>
where
    I: Copy + Debug + Eq,
{
    #[inline]
    fn from(c: ConditionCode) -> Part<I> {
        Part::ConditionCode(c)
    }
}

macro_rules! accessors {
    ( $( $variant:ident , $result:ty , $getter:ident , $is:ident , $unwrap:ident ; )* ) => {
        $(
            #[inline]
            #[allow(missing_docs)]
            pub fn $getter(&self) -> Option<$result> {
                match *self {
                    Self::$variant(x, ..) => Some(x),
                    _ => None
                }
            }

            #[inline]
            #[allow(missing_docs)]
            pub fn $is(&self) -> bool {
                self.$getter().is_some()
            }

            #[inline]
            #[allow(missing_docs)]
            pub fn $unwrap(&self) -> $result {
                self.$getter().expect(concat!("failed to unwrap `", stringify!($variant), "`"))
            }
        )*
    }
}

impl Constant {
    /// If this is any kind of integer constant, get it as a 64-bit unsigned
    /// integer.
    pub fn as_int(&self) -> Option<u64> {
        match *self {
            Constant::Bool(..) => None,
            Constant::Int(x, _) => Some(x),
        }
    }

    /// If this is any kind of boolean constant, get its value.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Constant::Bool(b, _) => Some(b),
            Constant::Int(..) => None,
        }
    }

    /// The number of bits required to represent this constant value's type.
    pub fn bit_width(&self, root_width: u8) -> u8 {
        match *self {
            Constant::Bool(_, w) | Constant::Int(_, w) => {
                if let Some(w) = w.fixed_width() {
                    w
                } else {
                    debug_assert!(w.is_polymorphic());
                    root_width
                }
            }
        }
    }
}

impl<I> Part<I>
where
    I: Copy + Debug + Eq,
{
    accessors! {
        Instruction, I, as_instruction, is_instruction, unwrap_instruction;
        Constant, Constant, as_constant, is_constant, unwrap_constant;
        ConditionCode, ConditionCode, as_condition_code, is_condition_code, unwrap_condition_code;
    }
}
