//! The [Value] trait describes what operations can be performed on interpreter values. The
//! interpreter usually executes using [DataValue]s so an implementation is provided here. The fact
//! that [Value] is a trait, however, allows interpretation of Cranelift IR on other kinds of
//! values.
use core::convert::TryFrom;
use core::fmt::{self, Display, Formatter};
use cranelift_codegen::data_value::{DataValue, DataValueCastFailure};
use cranelift_codegen::ir::immediates::{Ieee32, Ieee64};
use cranelift_codegen::ir::{types, Type};
use thiserror::Error;

pub type ValueResult<T> = Result<T, ValueError>;

pub trait Value: Clone + From<DataValue> {
    // Identity.
    fn ty(&self) -> Type;
    fn int(n: i128, ty: Type) -> ValueResult<Self>;
    fn into_int(self) -> ValueResult<i128>;
    fn float(n: u64, ty: Type) -> ValueResult<Self>;
    fn into_float(self) -> ValueResult<f64>;
    fn is_nan(&self) -> ValueResult<bool>;
    fn bool(b: bool, ty: Type) -> ValueResult<Self>;
    fn into_bool(self) -> ValueResult<bool>;
    fn vector(v: [u8; 16], ty: Type) -> ValueResult<Self>;
    fn into_array(&self) -> ValueResult<[u8; 16]>;
    fn convert(self, kind: ValueConversionKind) -> ValueResult<Self>;
    fn concat(self, other: Self) -> ValueResult<Self>;

    // Comparison.
    fn eq(&self, other: &Self) -> ValueResult<bool>;
    fn gt(&self, other: &Self) -> ValueResult<bool>;
    fn ge(&self, other: &Self) -> ValueResult<bool> {
        Ok(self.eq(other)? || self.gt(other)?)
    }
    fn lt(&self, other: &Self) -> ValueResult<bool> {
        other.gt(self)
    }
    fn le(&self, other: &Self) -> ValueResult<bool> {
        Ok(other.eq(self)? || other.gt(self)?)
    }
    fn uno(&self, other: &Self) -> ValueResult<bool>;
    fn overflow(&self, other: &Self) -> ValueResult<bool>;

    // Arithmetic.
    fn add(self, other: Self) -> ValueResult<Self>;
    fn sub(self, other: Self) -> ValueResult<Self>;
    fn mul(self, other: Self) -> ValueResult<Self>;
    fn div(self, other: Self) -> ValueResult<Self>;
    fn rem(self, other: Self) -> ValueResult<Self>;

    // Saturating arithmetic.
    fn add_sat(self, other: Self) -> ValueResult<Self>;
    fn sub_sat(self, other: Self) -> ValueResult<Self>;

    // Bitwise.
    fn shl(self, other: Self) -> ValueResult<Self>;
    fn ushr(self, other: Self) -> ValueResult<Self>;
    fn ishr(self, other: Self) -> ValueResult<Self>;
    fn rotl(self, other: Self) -> ValueResult<Self>;
    fn rotr(self, other: Self) -> ValueResult<Self>;
    fn and(self, other: Self) -> ValueResult<Self>;
    fn or(self, other: Self) -> ValueResult<Self>;
    fn xor(self, other: Self) -> ValueResult<Self>;
    fn not(self) -> ValueResult<Self>;

    // Bit counting.
    fn count_ones(self) -> ValueResult<Self>;
    fn leading_ones(self) -> ValueResult<Self>;
    fn leading_zeros(self) -> ValueResult<Self>;
    fn trailing_zeros(self) -> ValueResult<Self>;
    fn reverse_bits(self) -> ValueResult<Self>;
}

#[derive(Error, Debug, PartialEq)]
pub enum ValueError {
    #[error("unable to convert type {1} into class {0}")]
    InvalidType(ValueTypeClass, Type),
    #[error("unable to convert value into type {0}")]
    InvalidValue(Type),
    #[error("unable to convert to primitive integer")]
    InvalidInteger(#[from] std::num::TryFromIntError),
    #[error("unable to cast data value")]
    InvalidDataValueCast(#[from] DataValueCastFailure),
    #[error("performed a division by zero")]
    IntegerDivisionByZero,
    #[error("performed a operation that overflowed this integer type")]
    IntegerOverflow,
}

#[derive(Debug, PartialEq)]
pub enum ValueTypeClass {
    Integer,
    Boolean,
    Float,
    Vector,
}

impl Display for ValueTypeClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ValueTypeClass::Integer => write!(f, "integer"),
            ValueTypeClass::Boolean => write!(f, "boolean"),
            ValueTypeClass::Float => write!(f, "float"),
            ValueTypeClass::Vector => write!(f, "vector"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueConversionKind {
    /// Throw a [ValueError] if an exact conversion to [Type] is not possible; e.g. in `i32` to
    /// `i16`, convert `0x00001234` to `0x1234`.
    Exact(Type),
    /// Truncate the value to fit into the specified [Type]; e.g. in `i16` to `i8`, `0x1234` becomes
    /// `0x34`.
    Truncate(Type),
    ///  Similar to Truncate, but extracts from the top of the value; e.g. in a `i32` to `u8`,
    /// `0x12345678` becomes `0x12`.
    ExtractUpper(Type),
    /// Convert to a larger integer type, extending the sign bit; e.g. in `i8` to `i16`, `0xff`
    /// becomes `0xffff`.
    SignExtend(Type),
    /// Convert to a larger integer type, extending with zeroes; e.g. in `i8` to `i16`, `0xff`
    /// becomes `0x00ff`.
    ZeroExtend(Type),
    /// Convert a signed integer to its unsigned value of the same size; e.g. in `i8` to `u8`,
    /// `0xff` (`-1`) becomes `0xff` (`255`).
    ToUnsigned,
    /// Convert an unsigned integer to its signed value of the same size; e.g. in `u8` to `i8`,
    /// `0xff` (`255`) becomes `0xff` (`-1`).
    ToSigned,
    /// Convert a floating point number by rounding to the nearest possible value with ties to even.
    /// See `fdemote`, e.g.
    RoundNearestEven(Type),
    /// Converts an integer into a boolean, zero integers are converted into a
    /// `false`, while other integers are converted into `true`. Booleans are passed through.
    ToBoolean,
}

/// Helper for creating match expressions over [DataValue].
macro_rules! unary_match {
    ( $op:ident($arg1:expr); [ $( $data_value_ty:ident ),* ]; [ $( $return_value_ty:ident ),* ] ) => {
        match $arg1 {
            $( DataValue::$data_value_ty(a) => {
                Ok(DataValue::$data_value_ty($return_value_ty::try_from(a.$op()).unwrap()))
            } )*
            _ => unimplemented!()
        }
    };
    ( $op:ident($arg1:expr); [ $( $data_value_ty:ident ),* ] ) => {
        match $arg1 {
            $( DataValue::$data_value_ty(a) => { Ok(DataValue::$data_value_ty(a.$op())) } )*
            _ => unimplemented!()
        }
    };
    ( $op:tt($arg1:expr); [ $( $data_value_ty:ident ),* ] ) => {
        match $arg1 {
            $( DataValue::$data_value_ty(a) => { Ok(DataValue::$data_value_ty($op a)) } )*
            _ => unimplemented!()
        }
    };
}
macro_rules! binary_match {
    ( $op:ident($arg1:expr, $arg2:expr); [ $( $data_value_ty:ident ),* ] ) => {
        match ($arg1, $arg2) {
            $( (DataValue::$data_value_ty(a), DataValue::$data_value_ty(b)) => { Ok(DataValue::$data_value_ty(a.$op(*b))) } )*
            _ => unimplemented!()
        }
    };
    ( $op:tt($arg1:expr, $arg2:expr); [ $( $data_value_ty:ident ),* ] ) => {
        match ($arg1, $arg2) {
            $( (DataValue::$data_value_ty(a), DataValue::$data_value_ty(b)) => { Ok(DataValue::$data_value_ty(a $op b)) } )*
            _ => unimplemented!()
        }
    };
    ( $op:tt($arg1:expr, $arg2:expr); unsigned integers ) => {
        match ($arg1, $arg2) {
            (DataValue::I8(a), DataValue::I8(b)) => { Ok(DataValue::I8((u8::try_from(*a)? $op u8::try_from(*b)?) as i8)) }
            (DataValue::I16(a), DataValue::I16(b)) => { Ok(DataValue::I16((u16::try_from(*a)? $op u16::try_from(*b)?) as i16)) }
            (DataValue::I32(a), DataValue::I32(b)) => { Ok(DataValue::I32((u32::try_from(*a)? $op u32::try_from(*b)?) as i32)) }
            (DataValue::I64(a), DataValue::I64(b)) => { Ok(DataValue::I64((u64::try_from(*a)? $op u64::try_from(*b)?) as i64)) }
            _ => { Err(ValueError::InvalidType(ValueTypeClass::Integer, if !($arg1).ty().is_int() { ($arg1).ty() } else { ($arg2).ty() })) }
        }
    };
}
macro_rules! comparison_match {
    ( $op:path[$arg1:expr, $arg2:expr]; [ $( $data_value_ty:ident ),* ] ) => {
        match ($arg1, $arg2) {
            $( (DataValue::$data_value_ty(a), DataValue::$data_value_ty(b)) => { Ok($op(a, b)) } )*
            _ => unimplemented!("comparison: {:?}, {:?}", $arg1, $arg2)
        }
    };
}

impl Value for DataValue {
    fn ty(&self) -> Type {
        self.ty()
    }

    fn int(n: i128, ty: Type) -> ValueResult<Self> {
        if ty.is_int() && !ty.is_vector() {
            DataValue::from_integer(n, ty).map_err(|_| ValueError::InvalidValue(ty))
        } else {
            Err(ValueError::InvalidType(ValueTypeClass::Integer, ty))
        }
    }

    fn into_int(self) -> ValueResult<i128> {
        match self {
            DataValue::I8(n) => Ok(n as i128),
            DataValue::I16(n) => Ok(n as i128),
            DataValue::I32(n) => Ok(n as i128),
            DataValue::I64(n) => Ok(n as i128),
            DataValue::I128(n) => Ok(n),
            DataValue::U8(n) => Ok(n as i128),
            DataValue::U16(n) => Ok(n as i128),
            DataValue::U32(n) => Ok(n as i128),
            DataValue::U64(n) => Ok(n as i128),
            DataValue::U128(n) => Ok(n as i128),
            _ => Err(ValueError::InvalidType(ValueTypeClass::Integer, self.ty())),
        }
    }

    fn float(bits: u64, ty: Type) -> ValueResult<Self> {
        match ty {
            types::F32 => Ok(DataValue::F32(Ieee32::with_bits(u32::try_from(bits)?))),
            types::F64 => Ok(DataValue::F64(Ieee64::with_bits(bits))),
            _ => Err(ValueError::InvalidType(ValueTypeClass::Float, ty)),
        }
    }

    fn into_float(self) -> ValueResult<f64> {
        unimplemented!()
    }

    fn is_nan(&self) -> ValueResult<bool> {
        match self {
            DataValue::F32(f) => Ok(f.is_nan()),
            DataValue::F64(f) => Ok(f.is_nan()),
            _ => Err(ValueError::InvalidType(ValueTypeClass::Float, self.ty())),
        }
    }

    fn bool(b: bool, ty: Type) -> ValueResult<Self> {
        assert!(ty.is_bool());
        Ok(DataValue::B(b))
    }

    fn into_bool(self) -> ValueResult<bool> {
        match self {
            DataValue::B(b) => Ok(b),
            _ => Err(ValueError::InvalidType(ValueTypeClass::Boolean, self.ty())),
        }
    }

    fn vector(v: [u8; 16], ty: Type) -> ValueResult<Self> {
        assert!(ty.is_vector() && ty.bytes() == 16);
        Ok(DataValue::V128(v))
    }

    fn into_array(&self) -> ValueResult<[u8; 16]> {
        match *self {
            DataValue::V128(v) => Ok(v),
            _ => Err(ValueError::InvalidType(ValueTypeClass::Vector, self.ty())),
        }
    }

    fn convert(self, kind: ValueConversionKind) -> ValueResult<Self> {
        Ok(match kind {
            ValueConversionKind::Exact(ty) => match (self, ty) {
                // TODO a lot to do here: from bmask to ireduce to raw_bitcast...
                (DataValue::I64(n), types::I32) => DataValue::I32(i32::try_from(n)?),
                (DataValue::I64(n), types::I64) => DataValue::I64(n),
                (DataValue::B(b), t) if t.is_bool() => DataValue::B(b),
                (dv, _) => unimplemented!("conversion: {} -> {:?}", dv.ty(), kind),
            },
            ValueConversionKind::Truncate(ty) => {
                assert!(
                    ty.is_int(),
                    "unimplemented conversion: {} -> {:?}",
                    self.ty(),
                    kind
                );

                let mask = (1 << (ty.bytes() * 8)) - 1i128;
                let truncated = self.into_int()? & mask;
                Self::from_integer(truncated, ty)?
            }
            ValueConversionKind::ExtractUpper(ty) => {
                assert!(
                    ty.is_int(),
                    "unimplemented conversion: {} -> {:?}",
                    self.ty(),
                    kind
                );

                let shift_amt = (self.ty().bytes() * 8) - (ty.bytes() * 8);
                let mask = (1 << (ty.bytes() * 8)) - 1i128;
                let shifted_mask = mask << shift_amt;

                let extracted = (self.into_int()? & shifted_mask) >> shift_amt;
                Self::from_integer(extracted, ty)?
            }
            ValueConversionKind::SignExtend(ty) => match (self.ty(), ty) {
                (types::I8, types::I16) => unimplemented!(),
                (types::I8, types::I32) => unimplemented!(),
                (types::I8, types::I64) => unimplemented!(),
                (types::I16, types::I32) => unimplemented!(),
                (types::I16, types::I64) => unimplemented!(),
                (types::I32, types::I64) => unimplemented!(),
                _ => unimplemented!("conversion: {} -> {:?}", self.ty(), kind),
            },
            ValueConversionKind::ZeroExtend(ty) => match (self, ty) {
                (DataValue::I8(n), types::I16) => DataValue::I16(n as u8 as i16),
                (DataValue::I8(n), types::I32) => DataValue::I32(n as u8 as i32),
                (DataValue::I8(n), types::I64) => DataValue::I64(n as u8 as i64),
                (DataValue::I16(n), types::I32) => DataValue::I32(n as u16 as i32),
                (DataValue::I16(n), types::I64) => DataValue::I64(n as u16 as i64),
                (DataValue::U32(n), types::I64) => DataValue::U64(n as u64),
                (DataValue::I32(n), types::I64) => DataValue::I64(n as u32 as i64),
                (DataValue::I64(n), types::I128) => DataValue::I128(n as u64 as i128),
                (from, to) if from.ty() == to => from,
                (dv, _) => unimplemented!("conversion: {} -> {:?}", dv.ty(), kind),
            },
            ValueConversionKind::ToUnsigned => match self {
                DataValue::I8(n) => DataValue::U8(n as u8),
                DataValue::I16(n) => DataValue::U16(n as u16),
                DataValue::I32(n) => DataValue::U32(n as u32),
                DataValue::I64(n) => DataValue::U64(n as u64),
                _ => unimplemented!("conversion: {} -> {:?}", self.ty(), kind),
            },
            ValueConversionKind::ToSigned => match self {
                DataValue::U8(n) => DataValue::I8(n as i8),
                DataValue::U16(n) => DataValue::I16(n as i16),
                DataValue::U32(n) => DataValue::I32(n as i32),
                DataValue::U64(n) => DataValue::I64(n as i64),
                _ => unimplemented!("conversion: {} -> {:?}", self.ty(), kind),
            },
            ValueConversionKind::RoundNearestEven(ty) => match (self.ty(), ty) {
                (types::F64, types::F32) => unimplemented!(),
                _ => unimplemented!("conversion: {} -> {:?}", self.ty(), kind),
            },
            ValueConversionKind::ToBoolean => match self.ty() {
                ty if ty.is_bool() => DataValue::B(self.into_bool()?),
                ty if ty.is_int() => DataValue::B(self.into_int()? != 0),
                ty => unimplemented!("conversion: {} -> {:?}", ty, kind),
            },
        })
    }

    fn concat(self, other: Self) -> ValueResult<Self> {
        match (self, other) {
            (DataValue::I64(lhs), DataValue::I64(rhs)) => Ok(DataValue::I128(
                (((lhs as u64) as u128) | (((rhs as u64) as u128) << 64)) as i128,
            )),
            (lhs, rhs) => unimplemented!("concat: {} -> {}", lhs.ty(), rhs.ty()),
        }
    }

    fn eq(&self, other: &Self) -> ValueResult<bool> {
        comparison_match!(PartialEq::eq[&self, &other]; [I8, I16, I32, I64, U8, U16, U32, U64, F32, F64])
    }

    fn gt(&self, other: &Self) -> ValueResult<bool> {
        comparison_match!(PartialOrd::gt[&self, &other]; [I8, I16, I32, I64, U8, U16, U32, U64, F32, F64])
    }

    fn uno(&self, other: &Self) -> ValueResult<bool> {
        Ok(self.is_nan()? || other.is_nan()?)
    }

    fn overflow(&self, other: &Self) -> ValueResult<bool> {
        Ok(match (self, other) {
            (DataValue::I8(a), DataValue::I8(b)) => a.checked_sub(*b).is_none(),
            (DataValue::I16(a), DataValue::I16(b)) => a.checked_sub(*b).is_none(),
            (DataValue::I32(a), DataValue::I32(b)) => a.checked_sub(*b).is_none(),
            (DataValue::I64(a), DataValue::I64(b)) => a.checked_sub(*b).is_none(),
            _ => unimplemented!(),
        })
    }

    fn add(self, other: Self) -> ValueResult<Self> {
        // TODO: floats must handle NaNs, +/-0
        binary_match!(wrapping_add(&self, &other); [I8, I16, I32, I64, I128, U8, U16, U32, U64, U128])
    }

    fn sub(self, other: Self) -> ValueResult<Self> {
        binary_match!(wrapping_sub(&self, &other); [I8, I16, I32, I64, I128]) // TODO: floats must handle NaNs, +/-0
    }

    fn mul(self, other: Self) -> ValueResult<Self> {
        binary_match!(wrapping_mul(&self, &other); [I8, I16, I32, I64, I128])
    }

    fn div(self, other: Self) -> ValueResult<Self> {
        let denominator = other.clone().into_int()?;

        // Check if we are dividing INT_MIN / -1. This causes an integer overflow trap.
        let min = Value::int(1i128 << (self.ty().bits() - 1), self.ty())?;
        if self == min && denominator == -1 {
            return Err(ValueError::IntegerOverflow);
        }

        if denominator == 0 {
            return Err(ValueError::IntegerDivisionByZero);
        }

        binary_match!(/(&self, &other); [I8, I16, I32, I64, U8, U16, U32, U64])
    }

    fn rem(self, other: Self) -> ValueResult<Self> {
        if other.clone().into_int()? == 0 {
            return Err(ValueError::IntegerDivisionByZero);
        }

        binary_match!(%(&self, &other); [I8, I16, I32, I64])
    }

    fn add_sat(self, other: Self) -> ValueResult<Self> {
        binary_match!(saturating_add(self, &other); [I8, I16, I32, I64, I128, U8, U16, U32, U64, U128])
    }

    fn sub_sat(self, other: Self) -> ValueResult<Self> {
        binary_match!(saturating_sub(self, &other); [I8, I16, I32, I64, I128, U8, U16, U32, U64, U128])
    }

    fn shl(self, other: Self) -> ValueResult<Self> {
        binary_match!(<<(&self, &other); [I8, I16, I32, I64])
    }

    fn ushr(self, other: Self) -> ValueResult<Self> {
        binary_match!(>>(&self, &other); unsigned integers)
    }

    fn ishr(self, other: Self) -> ValueResult<Self> {
        binary_match!(>>(&self, &other); [I8, I16, I32, I64])
    }

    fn rotl(self, _other: Self) -> ValueResult<Self> {
        unimplemented!()
    }

    fn rotr(self, _other: Self) -> ValueResult<Self> {
        unimplemented!()
    }

    fn and(self, other: Self) -> ValueResult<Self> {
        binary_match!(&(&self, &other); [B, I8, I16, I32, I64])
    }

    fn or(self, other: Self) -> ValueResult<Self> {
        binary_match!(|(&self, &other); [B, I8, I16, I32, I64])
    }

    fn xor(self, other: Self) -> ValueResult<Self> {
        binary_match!(^(&self, &other); [I8, I16, I32, I64])
    }

    fn not(self) -> ValueResult<Self> {
        unary_match!(!(&self); [I8, I16, I32, I64])
    }

    fn count_ones(self) -> ValueResult<Self> {
        unary_match!(count_ones(&self); [I8, I16, I32, I64, I128, U8, U16, U32, U64, U128]; [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128])
    }

    fn leading_ones(self) -> ValueResult<Self> {
        unary_match!(leading_ones(&self); [I8, I16, I32, I64, I128, U8, U16, U32, U64, U128]; [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128])
    }

    fn leading_zeros(self) -> ValueResult<Self> {
        unary_match!(leading_zeros(&self); [I8, I16, I32, I64, I128, U8, U16, U32, U64, U128]; [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128])
    }

    fn trailing_zeros(self) -> ValueResult<Self> {
        unary_match!(trailing_zeros(&self); [I8, I16, I32, I64, I128, U8, U16, U32, U64, U128]; [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128])
    }

    fn reverse_bits(self) -> ValueResult<Self> {
        unary_match!(reverse_bits(&self); [I8, I16, I32, I64, I128, U8, U16, U32, U64, U128])
    }
}
