
//! Common types for the Cretonne code generator.

use std::default::Default;
use std::fmt::{self, Display, Formatter, Write};

// ====--------------------------------------------------------------------------------------====//
//
// Value types
//
// ====--------------------------------------------------------------------------------------====//

/// The type of an SSA value.
///
/// The `VOID` type is only used for instructions that produce no value. It can't be part of a SIMD
/// vector.
///
/// Basic integer types: `I8`, `I16`, `I32`, and `I64`. These types are sign-agnostic.
///
/// Basic floating point types: `F32` and `F64`. IEEE single and double precision.
///
/// Boolean types: `B1`, `B8`, `B16`, `B32`, and `B64`. These all encode 'true' or 'false'. The
/// larger types use redundant bits.
///
/// SIMD vector types have power-of-two lanes, up to 256. Lanes can be any int/float/bool type.
///
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Type(u8);

/// No type. Used for functions without a return value. Can't be loaded or stored. Can't be part of
/// a SIMD vector.
pub const VOID: Type = Type(0);

/// Integer type with 8 bits.
pub const I8: Type = Type(1);

/// Integer type with 16 bits.
pub const I16: Type = Type(2);

/// Integer type with 32 bits.
pub const I32: Type = Type(3);

/// Integer type with 64 bits.
pub const I64: Type = Type(4);

/// IEEE single precision floating point type.
pub const F32: Type = Type(5);

/// IEEE double precision floating point type.
pub const F64: Type = Type(6);

/// Boolean type. Can't be loaded or stored, but can be used to form SIMD vectors.
pub const B1: Type = Type(7);

/// Boolean type using 8 bits to represent true/false.
pub const B8: Type = Type(8);

/// Boolean type using 16 bits to represent true/false.
pub const B16: Type = Type(9);

/// Boolean type using 32 bits to represent true/false.
pub const B32: Type = Type(10);

/// Boolean type using 64 bits to represent true/false.
pub const B64: Type = Type(11);

impl Type {
    /// Get the lane type of this SIMD vector type.
    ///
    /// A scalar type is the same as a SIMD vector type with one lane, so it returns itself.
    pub fn lane_type(self) -> Type {
        Type(self.0 & 0x0f)
    }

    /// Get the number of bits in a lane.
    pub fn lane_bits(self) -> u8 {
        match self.lane_type() {
            B1 => 1,
            B8 | I8 => 8,
            B16 | I16 => 16,
            B32 | I32 | F32 => 32,
            B64 | I64 | F64 => 64,
            _ => 0,
        }
    }

    /// Get a type with the same number of lanes as this type, but with the lanes replaces by
    /// booleans of the same size.
    pub fn as_bool(self) -> Type {
        // Replace the low 4 bits with the boolean version, preserve the high 4 bits.
        let lane = match self.lane_type() {
            B8 | I8 => B8,
            B16 | I16 => B16,
            B32 | I32 | F32 => B32,
            B64 | I64 | F64 => B64,
            _ => B1,
        };
        Type(lane.0 | (self.0 & 0xf0))
    }

    /// Is this the VOID type?
    pub fn is_void(self) -> bool {
        self == VOID
    }

    /// Is this a scalar boolean type?
    pub fn is_bool(self) -> bool {
        match self {
            B1 | B8 | B16 | B32 | B64 => true,
            _ => false,
        }
    }

    /// Is this a scalar integer type?
    pub fn is_int(self) -> bool {
        match self {
            I8 | I16 | I32 | I64 => true,
            _ => false,
        }
    }

    /// Is this a scalar floating point type?
    pub fn is_float(self) -> bool {
        match self {
            F32 | F64 => true,
            _ => false,
        }
    }

    /// Get log2 of the number of lanes in this SIMD vector type.
    ///
    /// All SIMD types have a lane count that is a power of two and no larger than 256, so this
    /// will be a number in the range 0-8.
    ///
    /// A scalar type is the same as a SIMD vector type with one lane, so it return 0.
    pub fn log2_lane_count(self) -> u8 {
        self.0 >> 4
    }

    /// Is this a scalar type? (That is, not a SIMD vector type).
    ///
    /// A scalar type is the same as a SIMD vector type with one lane.
    pub fn is_scalar(self) -> bool {
        self.log2_lane_count() == 0
    }

    /// Get the number of lanes in this SIMD vector type.
    ///
    /// A scalar type is the same as a SIMD vector type with one lane, so it returns 1.
    pub fn lane_count(self) -> u16 {
        1 << self.log2_lane_count()
    }

    /// Get the total number of bits used to represent this type.
    pub fn bits(self) -> u16 {
        self.lane_bits() as u16 * self.lane_count()
    }

    /// Get a SIMD vector type with `n` times more lanes than this one.
    ///
    /// If this is a scalar type, this produces a SIMD type with this as a lane type and `n` lanes.
    ///
    /// If this is already a SIMD vector type, this produces a SIMD vector type with `n *
    /// self.lane_count()` lanes.
    pub fn by(self, n: u16) -> Option<Type> {
        if self.lane_bits() == 0 || !n.is_power_of_two() {
            return None;
        }
        let log2_lanes: u32 = n.trailing_zeros();
        let new_type = self.0 as u32 + (log2_lanes << 4);
        if new_type < 0x90 {
            Some(Type(new_type as u8))
        } else {
            None
        }
    }

    /// Get a SIMD vector with half the number of lanes.
    pub fn half_vector(self) -> Option<Type> {
        if self.is_scalar() {
            None
        } else {
            Some(Type(self.0 - 0x10))
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_void() {
            write!(f, "void")
        } else if self.is_bool() {
            write!(f, "b{}", self.lane_bits())
        } else if self.is_int() {
            write!(f, "i{}", self.lane_bits())
        } else if self.is_float() {
            write!(f, "f{}", self.lane_bits())
        } else if !self.is_scalar() {
            write!(f, "{}x{}", self.lane_type(), self.lane_count())
        } else {
            panic!("Invalid Type(0x{:x})", self.0)
        }
    }
}

impl Default for Type {
    fn default() -> Type {
        VOID
    }
}

// ====--------------------------------------------------------------------------------------====//
//
// Function signatures
//
// ====--------------------------------------------------------------------------------------====//

/// The name of a function can be any UTF-8 string.
///
/// Function names are mostly a testing and debugging tool. In partucular, `.cton` files use
/// function names to identify functions.
pub type FunctionName = String;

/// Function argument extension options.
///
/// On some architectures, small integer function arguments are extended to the width of a
/// general-purpose register.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ArgumentExtension {
    /// No extension, high bits are indeterminate.
    None,
    /// Unsigned extension: high bits in register are 0.
    Uext,
    /// Signed extension: high bits in register replicate sign bit.
    Sext,
}

/// Function argument or return value type.
///
/// This describes the value type being passed to or from a function along with flags that affect
/// how the argument is passed.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ArgumentType {
    pub value_type: Type,
    pub extension: ArgumentExtension,
    /// Place this argument in a register if possible.
    pub inreg: bool,
}

/// Function signature.
///
/// The function signature describes the types of arguments and return values along with other
/// details that are needed to call a function correctly.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Signature {
    pub argument_types: Vec<ArgumentType>,
    pub return_types: Vec<ArgumentType>,
}

impl ArgumentType {
    pub fn new(vt: Type) -> ArgumentType {
        ArgumentType {
            value_type: vt,
            extension: ArgumentExtension::None,
            inreg: false,
        }
    }
}

impl Display for ArgumentType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        try!(write!(f, "{}", self.value_type));
        match self.extension {
            ArgumentExtension::None => {}
            ArgumentExtension::Uext => try!(write!(f, " uext")),
            ArgumentExtension::Sext => try!(write!(f, " sext")),
        }
        if self.inreg {
            try!(write!(f, " inreg"));
        }
        Ok(())
    }
}

impl Signature {
    pub fn new() -> Signature {
        Signature {
            argument_types: Vec::new(),
            return_types: Vec::new(),
        }
    }
}

fn write_list(f: &mut Formatter, args: &Vec<ArgumentType>) -> fmt::Result {
    match args.split_first() {
        None => {}
        Some((first, rest)) => {
            try!(write!(f, "{}", first));
            for arg in rest {
                try!(write!(f, ", {}", arg));
            }
        }
    }
    Ok(())
}

impl Display for Signature {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        try!(write!(f, "("));
        try!(write_list(f, &self.argument_types));
        try!(write!(f, ")"));
        if !self.return_types.is_empty() {
            try!(write!(f, " -> "));
            try!(write_list(f, &self.return_types));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_scalars() {
        assert_eq!(VOID, VOID.lane_type());
        assert_eq!(0, VOID.bits());
        assert_eq!(B1, B1.lane_type());
        assert_eq!(B8, B8.lane_type());
        assert_eq!(B16, B16.lane_type());
        assert_eq!(B32, B32.lane_type());
        assert_eq!(B64, B64.lane_type());
        assert_eq!(I8, I8.lane_type());
        assert_eq!(I16, I16.lane_type());
        assert_eq!(I32, I32.lane_type());
        assert_eq!(I64, I64.lane_type());
        assert_eq!(F32, F32.lane_type());
        assert_eq!(F64, F64.lane_type());

        assert_eq!(VOID.lane_bits(), 0);
        assert_eq!(B1.lane_bits(), 1);
        assert_eq!(B8.lane_bits(), 8);
        assert_eq!(B16.lane_bits(), 16);
        assert_eq!(B32.lane_bits(), 32);
        assert_eq!(B64.lane_bits(), 64);
        assert_eq!(I8.lane_bits(), 8);
        assert_eq!(I16.lane_bits(), 16);
        assert_eq!(I32.lane_bits(), 32);
        assert_eq!(I64.lane_bits(), 64);
        assert_eq!(F32.lane_bits(), 32);
        assert_eq!(F64.lane_bits(), 64);
    }

    #[test]
    fn vectors() {
        let big = F64.by(256).unwrap();
        assert_eq!(big.lane_bits(), 64);
        assert_eq!(big.lane_count(), 256);
        assert_eq!(big.bits(), 64 * 256);

        assert_eq!(big.half_vector().unwrap().to_string(), "f64x128");
        assert_eq!(B1.by(2).unwrap().half_vector().unwrap().to_string(), "b1");
        assert_eq!(I32.half_vector(), None);
        assert_eq!(VOID.half_vector(), None);
    }

    #[test]
    fn format_scalars() {
        assert_eq!(VOID.to_string(), "void");
        assert_eq!(B1.to_string(), "b1");
        assert_eq!(B8.to_string(), "b8");
        assert_eq!(B16.to_string(), "b16");
        assert_eq!(B32.to_string(), "b32");
        assert_eq!(B64.to_string(), "b64");
        assert_eq!(I8.to_string(), "i8");
        assert_eq!(I16.to_string(), "i16");
        assert_eq!(I32.to_string(), "i32");
        assert_eq!(I64.to_string(), "i64");
        assert_eq!(F32.to_string(), "f32");
        assert_eq!(F64.to_string(), "f64");
    }

    #[test]
    fn format_vectors() {
        assert_eq!(B1.by(8).unwrap().to_string(), "b1x8");
        assert_eq!(B8.by(1).unwrap().to_string(), "b8");
        assert_eq!(B16.by(256).unwrap().to_string(), "b16x256");
        assert_eq!(B32.by(4).unwrap().by(2).unwrap().to_string(), "b32x8");
        assert_eq!(B64.by(8).unwrap().to_string(), "b64x8");
        assert_eq!(I8.by(64).unwrap().to_string(), "i8x64");
        assert_eq!(F64.by(2).unwrap().to_string(), "f64x2");
        assert_eq!(I8.by(3), None);
        assert_eq!(I8.by(512), None);
        assert_eq!(VOID.by(4), None);
    }

    #[test]
    fn argument_type() {
        let mut t = ArgumentType::new(I32);
        assert_eq!(t.to_string(), "i32");
        t.extension = ArgumentExtension::Uext;
        assert_eq!(t.to_string(), "i32 uext");
        t.inreg = true;
        assert_eq!(t.to_string(), "i32 uext inreg");
    }

    #[test]
    fn signatures() {
        let mut sig = Signature::new();
        assert_eq!(sig.to_string(), "()");
        sig.argument_types.push(ArgumentType::new(I32));
        assert_eq!(sig.to_string(), "(i32)");
        sig.return_types.push(ArgumentType::new(F32));
        assert_eq!(sig.to_string(), "(i32) -> f32");
        sig.argument_types.push(ArgumentType::new(I32.by(4).unwrap()));
        assert_eq!(sig.to_string(), "(i32, i32x4) -> f32");
        sig.return_types.push(ArgumentType::new(B8));
        assert_eq!(sig.to_string(), "(i32, i32x4) -> f32, b8");
    }
}
