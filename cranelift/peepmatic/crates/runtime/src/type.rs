//! Types.

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;

/// A bit width of a type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum BitWidth {
    /// Polymorphic over bit width, with the same width as the root of the
    /// optimization's LHS and RHS.
    Polymorphic = 0,

    /// A fixed bit width of 1.
    One = 1,

    /// A fixed bit width of 8.
    Eight = 8,

    /// A fixed bit width of 16.
    Sixteen = 16,

    /// A fixed bit width of 32.
    ThirtyTwo = 32,

    /// A fixed bit width of 64.
    SixtyFour = 64,

    /// A fixed bit width of 128.
    OneTwentyEight = 128,
}

/// The kind of type we are looking at: either an integer kind or boolean kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    /// Integer kind.
    Int,

    /// Boolean kind.
    Bool,

    /// CPU flags kind.
    CpuFlags,

    /// Void kind.
    Void,
}

/// A type a value or the result of an operation.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Type {
    /// This type's kind.
    pub kind: Kind,

    /// This type's bit width.
    pub bit_width: BitWidth,
}

impl TryFrom<u8> for BitWidth {
    type Error = &'static str;

    #[inline]
    fn try_from(x: u8) -> Result<Self, Self::Error> {
        Ok(match x {
            1 => Self::One,
            8 => Self::Eight,
            16 => Self::Sixteen,
            32 => Self::ThirtyTwo,
            64 => Self::SixtyFour,
            128 => Self::OneTwentyEight,
            _ => return Err("not a valid bit width"),
        })
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            Kind::CpuFlags => return write!(f, "cpu-flags"),
            Kind::Void => return write!(f, "void"),
            Kind::Int => write!(f, "i")?,
            Kind::Bool => write!(f, "b")?,
        }
        match self.bit_width {
            BitWidth::Polymorphic => write!(f, "NN"),
            otherwise => write!(f, "{}", otherwise as u8),
        }
    }
}

impl BitWidth {
    /// Is this a polymorphic bit width?
    pub fn is_polymorphic(&self) -> bool {
        matches!(self, BitWidth::Polymorphic)
    }

    /// Get this width in bits, unless this is a polymorphic bit width.
    pub fn fixed_width(&self) -> Option<u8> {
        if self.is_polymorphic() {
            None
        } else {
            Some(*self as u8)
        }
    }
}

macro_rules! type_ctors {
    ( $( $( #[$attr:meta] )* $name:ident ( $kind:ident , $width:ident ) ; )* ) => {
        $(
            $( #[$attr] )*
            pub const fn $name() -> Self {
                Type {
                    kind: Kind::$kind,
                    bit_width: BitWidth::$width,
                }
            }
        )*
    }
}

impl Type {
    type_ctors! {
        /// Get the `i1` type.
        i1(Int, One);
        /// Get the `i8` type.
        i8(Int, Eight);
        /// Get the `i16` type.
        i16(Int, Sixteen);
        /// Get the `i32` type.
        i32(Int, ThirtyTwo);
        /// Get the `i64` type.
        i64(Int, SixtyFour);
        /// Get the `i128` type.
        i128(Int, OneTwentyEight);
        /// Get the `b1` type.
        b1(Bool, One);
        /// Get the `b8` type.
        b8(Bool, Eight);
        /// Get the `b16` type.
        b16(Bool, Sixteen);
        /// Get the `b32` type.
        b32(Bool, ThirtyTwo);
        /// Get the `b64` type.
        b64(Bool, SixtyFour);
        /// Get the `b128` type.
        b128(Bool, OneTwentyEight);
        /// Get the CPU flags type.
        cpu_flags(CpuFlags, One);
        /// Get the void type.
        void(Void, One);
    }
}

#[cfg(feature = "construct")]
mod tok {
    use wast::custom_keyword;
    custom_keyword!(b1);
    custom_keyword!(b8);
    custom_keyword!(b16);
    custom_keyword!(b32);
    custom_keyword!(b64);
    custom_keyword!(b128);
    custom_keyword!(i1);
    custom_keyword!(i8);
    custom_keyword!(i16);
    custom_keyword!(i32);
    custom_keyword!(i64);
    custom_keyword!(i128);
}

#[cfg(feature = "construct")]
impl<'a> wast::parser::Parse<'a> for Type {
    fn parse(p: wast::parser::Parser<'a>) -> wast::parser::Result<Self> {
        if p.peek::<tok::b1>() {
            p.parse::<tok::b1>()?;
            return Ok(Type {
                kind: Kind::Bool,
                bit_width: BitWidth::One,
            });
        }
        if p.peek::<tok::b8>() {
            p.parse::<tok::b8>()?;
            return Ok(Type {
                kind: Kind::Bool,
                bit_width: BitWidth::Eight,
            });
        }
        if p.peek::<tok::b16>() {
            p.parse::<tok::b16>()?;
            return Ok(Type {
                kind: Kind::Bool,
                bit_width: BitWidth::Sixteen,
            });
        }
        if p.peek::<tok::b32>() {
            p.parse::<tok::b32>()?;
            return Ok(Type {
                kind: Kind::Bool,
                bit_width: BitWidth::ThirtyTwo,
            });
        }
        if p.peek::<tok::b64>() {
            p.parse::<tok::b64>()?;
            return Ok(Type {
                kind: Kind::Bool,
                bit_width: BitWidth::SixtyFour,
            });
        }
        if p.peek::<tok::b128>() {
            p.parse::<tok::b128>()?;
            return Ok(Type {
                kind: Kind::Bool,
                bit_width: BitWidth::OneTwentyEight,
            });
        }
        if p.peek::<tok::i1>() {
            p.parse::<tok::i1>()?;
            return Ok(Type {
                kind: Kind::Int,
                bit_width: BitWidth::One,
            });
        }
        if p.peek::<tok::i8>() {
            p.parse::<tok::i8>()?;
            return Ok(Type {
                kind: Kind::Int,
                bit_width: BitWidth::Eight,
            });
        }
        if p.peek::<tok::i16>() {
            p.parse::<tok::i16>()?;
            return Ok(Type {
                kind: Kind::Int,
                bit_width: BitWidth::Sixteen,
            });
        }
        if p.peek::<tok::i32>() {
            p.parse::<tok::i32>()?;
            return Ok(Type {
                kind: Kind::Int,
                bit_width: BitWidth::ThirtyTwo,
            });
        }
        if p.peek::<tok::i64>() {
            p.parse::<tok::i64>()?;
            return Ok(Type {
                kind: Kind::Int,
                bit_width: BitWidth::SixtyFour,
            });
        }
        if p.peek::<tok::i128>() {
            p.parse::<tok::i128>()?;
            return Ok(Type {
                kind: Kind::Int,
                bit_width: BitWidth::OneTwentyEight,
            });
        }
        Err(p.error("expected an ascribed type"))
    }
}
