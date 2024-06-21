//! The [DataValueExt] trait is an extension trait for [DataValue]. It provides a lot of functions
//! used by the rest of the interpreter.

#![allow(trivial_numeric_casts)]

use core::fmt::{self, Display, Formatter};
use cranelift_codegen::data_value::{DataValue, DataValueCastFailure};
use cranelift_codegen::ir::immediates::{Ieee128, Ieee16, Ieee32, Ieee64};
use cranelift_codegen::ir::{types, Type};
use thiserror::Error;

use crate::step::{extractlanes, SimdVec};

pub type ValueResult<T> = Result<T, ValueError>;

pub trait DataValueExt: Sized {
    // Identity.
    fn int(n: i128, ty: Type) -> ValueResult<Self>;
    fn into_int_signed(self) -> ValueResult<i128>;
    fn into_int_unsigned(self) -> ValueResult<u128>;
    fn float(n: u64, ty: Type) -> ValueResult<Self>;
    fn into_float(self) -> ValueResult<f64>;
    fn is_float(&self) -> bool;
    fn is_nan(&self) -> ValueResult<bool>;
    fn bool(b: bool, vec_elem: bool, ty: Type) -> ValueResult<Self>;
    fn into_bool(self) -> ValueResult<bool>;
    fn vector(v: [u8; 16], ty: Type) -> ValueResult<Self>;
    fn into_array(&self) -> ValueResult<[u8; 16]>;
    fn convert(self, kind: ValueConversionKind) -> ValueResult<Self>;
    fn concat(self, other: Self) -> ValueResult<Self>;

    fn is_negative(&self) -> ValueResult<bool>;
    fn is_zero(&self) -> ValueResult<bool>;

    fn umax(self, other: Self) -> ValueResult<Self>;
    fn smax(self, other: Self) -> ValueResult<Self>;
    fn umin(self, other: Self) -> ValueResult<Self>;
    fn smin(self, other: Self) -> ValueResult<Self>;

    // Comparison.
    fn uno(&self, other: &Self) -> ValueResult<bool>;

    // Arithmetic.
    fn add(self, other: Self) -> ValueResult<Self>;
    fn sub(self, other: Self) -> ValueResult<Self>;
    fn mul(self, other: Self) -> ValueResult<Self>;
    fn udiv(self, other: Self) -> ValueResult<Self>;
    fn sdiv(self, other: Self) -> ValueResult<Self>;
    fn urem(self, other: Self) -> ValueResult<Self>;
    fn srem(self, other: Self) -> ValueResult<Self>;
    fn sqrt(self) -> ValueResult<Self>;
    fn fma(self, a: Self, b: Self) -> ValueResult<Self>;
    fn abs(self) -> ValueResult<Self>;
    fn uadd_checked(self, other: Self) -> ValueResult<Option<Self>>;
    fn sadd_checked(self, other: Self) -> ValueResult<Option<Self>>;
    fn uadd_overflow(self, other: Self) -> ValueResult<(Self, bool)>;
    fn sadd_overflow(self, other: Self) -> ValueResult<(Self, bool)>;
    fn usub_overflow(self, other: Self) -> ValueResult<(Self, bool)>;
    fn ssub_overflow(self, other: Self) -> ValueResult<(Self, bool)>;
    fn umul_overflow(self, other: Self) -> ValueResult<(Self, bool)>;
    fn smul_overflow(self, other: Self) -> ValueResult<(Self, bool)>;

    // Float operations
    fn neg(self) -> ValueResult<Self>;
    fn copysign(self, sign: Self) -> ValueResult<Self>;
    fn ceil(self) -> ValueResult<Self>;
    fn floor(self) -> ValueResult<Self>;
    fn trunc(self) -> ValueResult<Self>;
    fn nearest(self) -> ValueResult<Self>;

    // Saturating arithmetic.
    fn uadd_sat(self, other: Self) -> ValueResult<Self>;
    fn sadd_sat(self, other: Self) -> ValueResult<Self>;
    fn usub_sat(self, other: Self) -> ValueResult<Self>;
    fn ssub_sat(self, other: Self) -> ValueResult<Self>;

    // Bitwise.
    fn shl(self, other: Self) -> ValueResult<Self>;
    fn ushr(self, other: Self) -> ValueResult<Self>;
    fn sshr(self, other: Self) -> ValueResult<Self>;
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
    fn swap_bytes(self) -> ValueResult<Self>;

    // An iterator over the lanes of a SIMD type
    fn iter_lanes(&self, ty: Type) -> ValueResult<DataValueIterator>;
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
    /// Convert a floating point number by rounding to the nearest possible value with ties to even.
    /// See `fdemote`, e.g.
    RoundNearestEven(Type),
    /// Converts an integer into a boolean, zero integers are converted into a
    /// `false`, while other integers are converted into `true`. Booleans are passed through.
    ToBoolean,
    /// Converts an integer into either -1 or zero.
    Mask(Type),
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
    ( $op:ident($arg1:expr, $arg2:expr); [ $( $data_value_ty:ident ),* ]; [ $( $op_type:ty ),* ] ) => {
        match ($arg1, $arg2) {
            $( (DataValue::$data_value_ty(a), DataValue::$data_value_ty(b)) => { Ok(DataValue::$data_value_ty((*a as $op_type).$op(*b as $op_type) as _)) } )*
            _ => unimplemented!()
        }
    };
    ( option $op:ident($arg1:expr, $arg2:expr); [ $( $data_value_ty:ident ),* ]; [ $( $op_type:ty ),* ] ) => {
        match ($arg1, $arg2) {
            $( (DataValue::$data_value_ty(a), DataValue::$data_value_ty(b)) => { Ok((*a as $op_type).$op(*b as $op_type).map(|v| DataValue::$data_value_ty(v as _))) } )*
            _ => unimplemented!()
        }
    };
    ( pair $op:ident($arg1:expr, $arg2:expr); [ $( $data_value_ty:ident ),* ]; [ $( $op_type:ty ),* ] ) => {
        match ($arg1, $arg2) {
            $( (DataValue::$data_value_ty(a), DataValue::$data_value_ty(b)) => {
                let (f, s) = (*a as $op_type).$op(*b as $op_type);
                Ok((DataValue::$data_value_ty(f as _), s))
            } )*
            _ => unimplemented!()
        }
    };
    ( $op:tt($arg1:expr, $arg2:expr); [ $( $data_value_ty:ident ),* ] ) => {
        match ($arg1, $arg2) {
            $( (DataValue::$data_value_ty(a), DataValue::$data_value_ty(b)) => { Ok(DataValue::$data_value_ty(a $op b)) } )*
            _ => unimplemented!()
        }
    };
    ( $op:tt($arg1:expr, $arg2:expr); [ $( $data_value_ty:ident ),* ]; [ $( $op_type:ty ),* ] ) => {
        match ($arg1, $arg2) {
            $( (DataValue::$data_value_ty(a), DataValue::$data_value_ty(b)) => { Ok(DataValue::$data_value_ty(((*a as $op_type) $op (*b as $op_type)) as _)) } )*
            _ => unimplemented!()
        }
    };
    ( $op:tt($arg1:expr, $arg2:expr); [ $( $data_value_ty:ident ),* ]; [ $( $a_type:ty ),* ]; rhs: $rhs:tt,$rhs_type:ty ) => {
        match ($arg1, $arg2) {
            $( (DataValue::$data_value_ty(a), DataValue::$rhs(b)) => { Ok(DataValue::$data_value_ty((*a as $a_type).$op(*b as $rhs_type) as _)) } )*
            _ => unimplemented!()
        }
    };
    ( $op:ident($arg1:expr, $arg2:expr); unsigned integers ) => {
        match ($arg1, $arg2) {
            (DataValue::I8(a), DataValue::I8(b)) => { Ok(DataValue::I8((u8::try_from(*a)?.$op(u8::try_from(*b)?) as i8))) }
            (DataValue::I16(a), DataValue::I16(b)) => { Ok(DataValue::I16((u16::try_from(*a)?.$op(u16::try_from(*b)?) as i16))) }
            (DataValue::I32(a), DataValue::I32(b)) => { Ok(DataValue::I32((u32::try_from(*a)?.$op(u32::try_from(*b)?) as i32))) }
            (DataValue::I64(a), DataValue::I64(b)) => { Ok(DataValue::I64((u64::try_from(*a)?.$op(u64::try_from(*b)?) as i64))) }
            (DataValue::I128(a), DataValue::I128(b)) => { Ok(DataValue::I128((u128::try_from(*a)?.$op(u128::try_from(*b)?) as i64))) }
            _ => { Err(ValueError::InvalidType(ValueTypeClass::Integer, if !($arg1).ty().is_int() { ($arg1).ty() } else { ($arg2).ty() })) }
        }
    };
}

macro_rules! bitop {
    ( $op:tt($arg1:expr, $arg2:expr) ) => {
        Ok(match ($arg1, $arg2) {
            (DataValue::I8(a), DataValue::I8(b)) => DataValue::I8(a $op b),
            (DataValue::I16(a), DataValue::I16(b)) => DataValue::I16(a $op b),
            (DataValue::I32(a), DataValue::I32(b)) => DataValue::I32(a $op b),
            (DataValue::I64(a), DataValue::I64(b)) => DataValue::I64(a $op b),
            (DataValue::I128(a), DataValue::I128(b)) => DataValue::I128(a $op b),
            (DataValue::F32(a), DataValue::F32(b)) => DataValue::F32(a $op b),
            (DataValue::F64(a), DataValue::F64(b)) => DataValue::F64(a $op b),
            (DataValue::V128(a), DataValue::V128(b)) => {
                let mut a2 = a.clone();
                for (a, b) in a2.iter_mut().zip(b.iter()) {
                    *a = *a $op *b;
                }
                DataValue::V128(a2)
            }
            _ => unimplemented!(),
        })
    };
}

impl DataValueExt for DataValue {
    fn int(n: i128, ty: Type) -> ValueResult<Self> {
        if ty.is_vector() {
            // match ensures graceful failure since read_from_slice_ne()
            // panics on anything other than 8 and 16 bytes
            match ty.bytes() {
                8 | 16 => Ok(DataValue::read_from_slice_ne(&n.to_ne_bytes(), ty)),
                _ => Err(ValueError::InvalidType(ValueTypeClass::Vector, ty)),
            }
        } else if ty.is_int() {
            DataValue::from_integer(n, ty).map_err(|_| ValueError::InvalidValue(ty))
        } else {
            Err(ValueError::InvalidType(ValueTypeClass::Integer, ty))
        }
    }

    fn into_int_signed(self) -> ValueResult<i128> {
        match self {
            DataValue::I8(n) => Ok(n as i128),
            DataValue::I16(n) => Ok(n as i128),
            DataValue::I32(n) => Ok(n as i128),
            DataValue::I64(n) => Ok(n as i128),
            DataValue::I128(n) => Ok(n),
            _ => Err(ValueError::InvalidType(ValueTypeClass::Integer, self.ty())),
        }
    }

    fn into_int_unsigned(self) -> ValueResult<u128> {
        match self {
            DataValue::I8(n) => Ok(n as u8 as u128),
            DataValue::I16(n) => Ok(n as u16 as u128),
            DataValue::I32(n) => Ok(n as u32 as u128),
            DataValue::I64(n) => Ok(n as u64 as u128),
            DataValue::I128(n) => Ok(n as u128),
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
        match self {
            DataValue::F32(n) => Ok(n.as_f32() as f64),
            DataValue::F64(n) => Ok(n.as_f64()),
            _ => Err(ValueError::InvalidType(ValueTypeClass::Float, self.ty())),
        }
    }

    fn is_float(&self) -> bool {
        match self {
            DataValue::F16(_) | DataValue::F32(_) | DataValue::F64(_) | DataValue::F128(_) => true,
            _ => false,
        }
    }

    fn is_nan(&self) -> ValueResult<bool> {
        match self {
            DataValue::F32(f) => Ok(f.is_nan()),
            DataValue::F64(f) => Ok(f.is_nan()),
            _ => Err(ValueError::InvalidType(ValueTypeClass::Float, self.ty())),
        }
    }

    fn bool(b: bool, vec_elem: bool, ty: Type) -> ValueResult<Self> {
        assert!(ty.is_int());
        macro_rules! make_bool {
            ($ty:ident) => {
                Ok(DataValue::$ty(if b {
                    if vec_elem {
                        -1
                    } else {
                        1
                    }
                } else {
                    0
                }))
            };
        }

        match ty {
            types::I8 => make_bool!(I8),
            types::I16 => make_bool!(I16),
            types::I32 => make_bool!(I32),
            types::I64 => make_bool!(I64),
            types::I128 => make_bool!(I128),
            _ => Err(ValueError::InvalidType(ValueTypeClass::Integer, ty)),
        }
    }

    fn into_bool(self) -> ValueResult<bool> {
        match self {
            DataValue::I8(b) => Ok(b != 0),
            DataValue::I16(b) => Ok(b != 0),
            DataValue::I32(b) => Ok(b != 0),
            DataValue::I64(b) => Ok(b != 0),
            DataValue::I128(b) => Ok(b != 0),
            _ => Err(ValueError::InvalidType(ValueTypeClass::Boolean, self.ty())),
        }
    }

    fn vector(v: [u8; 16], ty: Type) -> ValueResult<Self> {
        assert!(ty.is_vector() && [8, 16].contains(&ty.bytes()));
        if ty.bytes() == 16 {
            Ok(DataValue::V128(v))
        } else if ty.bytes() == 8 {
            let v64: [u8; 8] = v[..8].try_into().unwrap();
            Ok(DataValue::V64(v64))
        } else {
            unimplemented!()
        }
    }

    fn into_array(&self) -> ValueResult<[u8; 16]> {
        match *self {
            DataValue::V128(v) => Ok(v),
            DataValue::V64(v) => {
                let mut v128 = [0; 16];
                v128[..8].clone_from_slice(&v);
                Ok(v128)
            }
            _ => Err(ValueError::InvalidType(ValueTypeClass::Vector, self.ty())),
        }
    }

    fn convert(self, kind: ValueConversionKind) -> ValueResult<Self> {
        Ok(match kind {
            ValueConversionKind::Exact(ty) => match (self, ty) {
                // TODO a lot to do here: from bmask to ireduce to bitcast...
                (val, ty) if val.ty().is_int() && ty.is_int() => {
                    DataValue::from_integer(val.into_int_signed()?, ty)?
                }
                (DataValue::I16(n), types::F16) => DataValue::F16(Ieee16::with_bits(n as u16)),
                (DataValue::I32(n), types::F32) => DataValue::F32(f32::from_bits(n as u32).into()),
                (DataValue::I64(n), types::F64) => DataValue::F64(f64::from_bits(n as u64).into()),
                (DataValue::I128(n), types::F128) => DataValue::F128(Ieee128::with_bits(n as u128)),
                (DataValue::F16(n), types::I16) => DataValue::I16(n.bits() as i16),
                (DataValue::F32(n), types::I32) => DataValue::I32(n.bits() as i32),
                (DataValue::F64(n), types::I64) => DataValue::I64(n.bits() as i64),
                (DataValue::F128(n), types::I128) => DataValue::I128(n.bits() as i128),
                (DataValue::F32(n), types::F64) => DataValue::F64((n.as_f32() as f64).into()),
                (dv, t) if (t.is_int() || t.is_float()) && dv.ty() == t => dv,
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
                let truncated = self.into_int_signed()? & mask;
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

                let extracted = (self.into_int_signed()? & shifted_mask) >> shift_amt;
                Self::from_integer(extracted, ty)?
            }
            ValueConversionKind::SignExtend(ty) => match (self, ty) {
                (DataValue::I8(n), types::I16) => DataValue::I16(n as i16),
                (DataValue::I8(n), types::I32) => DataValue::I32(n as i32),
                (DataValue::I8(n), types::I64) => DataValue::I64(n as i64),
                (DataValue::I8(n), types::I128) => DataValue::I128(n as i128),
                (DataValue::I16(n), types::I32) => DataValue::I32(n as i32),
                (DataValue::I16(n), types::I64) => DataValue::I64(n as i64),
                (DataValue::I16(n), types::I128) => DataValue::I128(n as i128),
                (DataValue::I32(n), types::I64) => DataValue::I64(n as i64),
                (DataValue::I32(n), types::I128) => DataValue::I128(n as i128),
                (DataValue::I64(n), types::I128) => DataValue::I128(n as i128),
                (dv, _) => unimplemented!("conversion: {} -> {:?}", dv.ty(), kind),
            },
            ValueConversionKind::ZeroExtend(ty) => match (self, ty) {
                (DataValue::I8(n), types::I16) => DataValue::I16(n as u8 as i16),
                (DataValue::I8(n), types::I32) => DataValue::I32(n as u8 as i32),
                (DataValue::I8(n), types::I64) => DataValue::I64(n as u8 as i64),
                (DataValue::I8(n), types::I128) => DataValue::I128(n as u8 as i128),
                (DataValue::I16(n), types::I32) => DataValue::I32(n as u16 as i32),
                (DataValue::I16(n), types::I64) => DataValue::I64(n as u16 as i64),
                (DataValue::I16(n), types::I128) => DataValue::I128(n as u16 as i128),
                (DataValue::I32(n), types::I64) => DataValue::I64(n as u32 as i64),
                (DataValue::I32(n), types::I128) => DataValue::I128(n as u32 as i128),
                (DataValue::I64(n), types::I128) => DataValue::I128(n as u64 as i128),
                (from, to) if from.ty() == to => from,
                (dv, _) => unimplemented!("conversion: {} -> {:?}", dv.ty(), kind),
            },
            ValueConversionKind::RoundNearestEven(ty) => match (self, ty) {
                (DataValue::F64(n), types::F32) => DataValue::F32(Ieee32::from(n.as_f64() as f32)),
                (s, _) => unimplemented!("conversion: {} -> {:?}", s.ty(), kind),
            },
            ValueConversionKind::ToBoolean => match self.ty() {
                ty if ty.is_int() => {
                    DataValue::I8(if self.into_int_signed()? != 0 { 1 } else { 0 })
                }
                ty => unimplemented!("conversion: {} -> {:?}", ty, kind),
            },
            ValueConversionKind::Mask(ty) => {
                let b = self.into_bool()?;
                Self::bool(b, true, ty).unwrap()
            }
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

    fn is_negative(&self) -> ValueResult<bool> {
        match self {
            DataValue::F32(f) => Ok(f.is_negative()),
            DataValue::F64(f) => Ok(f.is_negative()),
            _ => Err(ValueError::InvalidType(ValueTypeClass::Float, self.ty())),
        }
    }

    fn is_zero(&self) -> ValueResult<bool> {
        match self {
            DataValue::I8(f) => Ok(*f == 0),
            DataValue::I16(f) => Ok(*f == 0),
            DataValue::I32(f) => Ok(*f == 0),
            DataValue::I64(f) => Ok(*f == 0),
            DataValue::I128(f) => Ok(*f == 0),
            DataValue::F16(f) => Ok(f.is_zero()),
            DataValue::F32(f) => Ok(f.is_zero()),
            DataValue::F64(f) => Ok(f.is_zero()),
            DataValue::F128(f) => Ok(f.is_zero()),
            DataValue::V64(_) | DataValue::V128(_) => {
                Err(ValueError::InvalidType(ValueTypeClass::Float, self.ty()))
            }
        }
    }

    fn umax(self, other: Self) -> ValueResult<Self> {
        let lhs = self.clone().into_int_unsigned()?;
        let rhs = other.clone().into_int_unsigned()?;
        if lhs > rhs {
            Ok(self)
        } else {
            Ok(other)
        }
    }

    fn smax(self, other: Self) -> ValueResult<Self> {
        if self > other {
            Ok(self)
        } else {
            Ok(other)
        }
    }

    fn umin(self, other: Self) -> ValueResult<Self> {
        let lhs = self.clone().into_int_unsigned()?;
        let rhs = other.clone().into_int_unsigned()?;
        if lhs < rhs {
            Ok(self)
        } else {
            Ok(other)
        }
    }

    fn smin(self, other: Self) -> ValueResult<Self> {
        if self < other {
            Ok(self)
        } else {
            Ok(other)
        }
    }

    fn uno(&self, other: &Self) -> ValueResult<bool> {
        Ok(self.is_nan()? || other.is_nan()?)
    }

    fn add(self, other: Self) -> ValueResult<Self> {
        if self.is_float() {
            binary_match!(+(self, other); [F32, F64])
        } else {
            binary_match!(wrapping_add(&self, &other); [I8, I16, I32, I64, I128])
        }
    }

    fn sub(self, other: Self) -> ValueResult<Self> {
        if self.is_float() {
            binary_match!(-(self, other); [F32, F64])
        } else {
            binary_match!(wrapping_sub(&self, &other); [I8, I16, I32, I64, I128])
        }
    }

    fn mul(self, other: Self) -> ValueResult<Self> {
        if self.is_float() {
            binary_match!(*(self, other); [F32, F64])
        } else {
            binary_match!(wrapping_mul(&self, &other); [I8, I16, I32, I64, I128])
        }
    }

    fn sdiv(self, other: Self) -> ValueResult<Self> {
        if self.is_float() {
            return binary_match!(/(self, other); [F32, F64]);
        }

        let denominator = other.clone().into_int_signed()?;

        // Check if we are dividing INT_MIN / -1. This causes an integer overflow trap.
        let min = DataValueExt::int(1i128 << (self.ty().bits() - 1), self.ty())?;
        if self == min && denominator == -1 {
            return Err(ValueError::IntegerOverflow);
        }

        if denominator == 0 {
            return Err(ValueError::IntegerDivisionByZero);
        }

        binary_match!(/(&self, &other); [I8, I16, I32, I64, I128])
    }

    fn udiv(self, other: Self) -> ValueResult<Self> {
        if self.is_float() {
            return binary_match!(/(self, other); [F32, F64]);
        }

        let denominator = other.clone().into_int_unsigned()?;

        if denominator == 0 {
            return Err(ValueError::IntegerDivisionByZero);
        }

        binary_match!(/(&self, &other); [I8, I16, I32, I64, I128]; [u8, u16, u32, u64, u128])
    }

    fn srem(self, other: Self) -> ValueResult<Self> {
        let denominator = other.clone().into_int_signed()?;

        // Check if we are dividing INT_MIN / -1. This causes an integer overflow trap.
        let min = DataValueExt::int(1i128 << (self.ty().bits() - 1), self.ty())?;
        if self == min && denominator == -1 {
            return Err(ValueError::IntegerOverflow);
        }

        if denominator == 0 {
            return Err(ValueError::IntegerDivisionByZero);
        }

        binary_match!(%(&self, &other); [I8, I16, I32, I64, I128])
    }

    fn urem(self, other: Self) -> ValueResult<Self> {
        let denominator = other.clone().into_int_unsigned()?;

        if denominator == 0 {
            return Err(ValueError::IntegerDivisionByZero);
        }

        binary_match!(%(&self, &other); [I8, I16, I32, I64, I128]; [u8, u16, u32, u64, u128])
    }

    fn sqrt(self) -> ValueResult<Self> {
        unary_match!(sqrt(&self); [F32, F64]; [Ieee32, Ieee64])
    }

    fn fma(self, b: Self, c: Self) -> ValueResult<Self> {
        match (self, b, c) {
            (DataValue::F32(a), DataValue::F32(b), DataValue::F32(c)) => {
                // The `fma` function for `x86_64-pc-windows-gnu` is incorrect. Use `libm`'s instead.
                // See: https://github.com/bytecodealliance/wasmtime/issues/4512
                #[cfg(all(target_arch = "x86_64", target_os = "windows", target_env = "gnu"))]
                let res = libm::fmaf(a.as_f32(), b.as_f32(), c.as_f32());

                #[cfg(not(all(
                    target_arch = "x86_64",
                    target_os = "windows",
                    target_env = "gnu"
                )))]
                let res = a.as_f32().mul_add(b.as_f32(), c.as_f32());

                Ok(DataValue::F32(res.into()))
            }
            (DataValue::F64(a), DataValue::F64(b), DataValue::F64(c)) => {
                #[cfg(all(target_arch = "x86_64", target_os = "windows", target_env = "gnu"))]
                let res = libm::fma(a.as_f64(), b.as_f64(), c.as_f64());

                #[cfg(not(all(
                    target_arch = "x86_64",
                    target_os = "windows",
                    target_env = "gnu"
                )))]
                let res = a.as_f64().mul_add(b.as_f64(), c.as_f64());

                Ok(DataValue::F64(res.into()))
            }
            (a, _b, _c) => Err(ValueError::InvalidType(ValueTypeClass::Float, a.ty())),
        }
    }

    fn abs(self) -> ValueResult<Self> {
        unary_match!(abs(&self); [F32, F64])
    }

    fn sadd_checked(self, other: Self) -> ValueResult<Option<Self>> {
        binary_match!(option checked_add(&self, &other); [I8, I16, I32, I64, I128]; [i8, i16, i32, i64, i128])
    }

    fn uadd_checked(self, other: Self) -> ValueResult<Option<Self>> {
        binary_match!(option checked_add(&self, &other); [I8, I16, I32, I64, I128]; [u8, u16, u32, u64, u128])
    }

    fn sadd_overflow(self, other: Self) -> ValueResult<(Self, bool)> {
        binary_match!(pair overflowing_add(&self, &other); [I8, I16, I32, I64, I128]; [i8, i16, i32, i64, i128])
    }

    fn uadd_overflow(self, other: Self) -> ValueResult<(Self, bool)> {
        binary_match!(pair overflowing_add(&self, &other); [I8, I16, I32, I64, I128]; [u8, u16, u32, u64, u128])
    }

    fn ssub_overflow(self, other: Self) -> ValueResult<(Self, bool)> {
        binary_match!(pair overflowing_sub(&self, &other); [I8, I16, I32, I64, I128]; [i8, i16, i32, i64, i128])
    }

    fn usub_overflow(self, other: Self) -> ValueResult<(Self, bool)> {
        binary_match!(pair overflowing_sub(&self, &other); [I8, I16, I32, I64, I128]; [u8, u16, u32, u64, u128])
    }

    fn smul_overflow(self, other: Self) -> ValueResult<(Self, bool)> {
        binary_match!(pair overflowing_mul(&self, &other); [I8, I16, I32, I64, I128]; [i8, i16, i32, i64, i128])
    }

    fn umul_overflow(self, other: Self) -> ValueResult<(Self, bool)> {
        binary_match!(pair overflowing_mul(&self, &other); [I8, I16, I32, I64, I128]; [u8, u16, u32, u64, u128])
    }

    fn neg(self) -> ValueResult<Self> {
        unary_match!(neg(&self); [F32, F64])
    }

    fn copysign(self, sign: Self) -> ValueResult<Self> {
        binary_match!(copysign(&self, &sign); [F32, F64])
    }

    fn ceil(self) -> ValueResult<Self> {
        unary_match!(ceil(&self); [F32, F64])
    }

    fn floor(self) -> ValueResult<Self> {
        unary_match!(floor(&self); [F32, F64])
    }

    fn trunc(self) -> ValueResult<Self> {
        unary_match!(trunc(&self); [F32, F64])
    }

    fn nearest(self) -> ValueResult<Self> {
        unary_match!(round_ties_even(&self); [F32, F64])
    }

    fn sadd_sat(self, other: Self) -> ValueResult<Self> {
        binary_match!(saturating_add(self, &other); [I8, I16, I32, I64, I128])
    }

    fn uadd_sat(self, other: Self) -> ValueResult<Self> {
        binary_match!(saturating_add(&self, &other); [I8, I16, I32, I64, I128]; [u8, u16, u32, u64, u128])
    }

    fn ssub_sat(self, other: Self) -> ValueResult<Self> {
        binary_match!(saturating_sub(self, &other); [I8, I16, I32, I64, I128])
    }

    fn usub_sat(self, other: Self) -> ValueResult<Self> {
        binary_match!(saturating_sub(&self, &other); [I8, I16, I32, I64, I128]; [u8, u16, u32, u64, u128])
    }

    fn shl(self, other: Self) -> ValueResult<Self> {
        let amt = other.convert(ValueConversionKind::Exact(types::I32))?;
        binary_match!(wrapping_shl(&self, &amt); [I8, I16, I32, I64, I128]; [i8, i16, i32, i64, i128]; rhs: I32,u32)
    }

    fn ushr(self, other: Self) -> ValueResult<Self> {
        let amt = other.convert(ValueConversionKind::Exact(types::I32))?;
        binary_match!(wrapping_shr(&self, &amt); [I8, I16, I32, I64, I128]; [u8, u16, u32, u64, u128]; rhs: I32,u32)
    }

    fn sshr(self, other: Self) -> ValueResult<Self> {
        let amt = other.convert(ValueConversionKind::Exact(types::I32))?;
        binary_match!(wrapping_shr(&self, &amt); [I8, I16, I32, I64, I128]; [i8, i16, i32, i64, i128]; rhs: I32,u32)
    }

    fn rotl(self, other: Self) -> ValueResult<Self> {
        let amt = other.convert(ValueConversionKind::Exact(types::I32))?;
        binary_match!(rotate_left(&self, &amt); [I8, I16, I32, I64, I128]; [i8, i16, i32, i64, i128]; rhs: I32,u32)
    }

    fn rotr(self, other: Self) -> ValueResult<Self> {
        let amt = other.convert(ValueConversionKind::Exact(types::I32))?;
        binary_match!(rotate_right(&self, &amt); [I8, I16, I32, I64, I128]; [i8, i16, i32, i64, i128]; rhs: I32,u32)
    }

    fn and(self, other: Self) -> ValueResult<Self> {
        bitop!(&(self, other))
    }

    fn or(self, other: Self) -> ValueResult<Self> {
        bitop!(|(self, other))
    }

    fn xor(self, other: Self) -> ValueResult<Self> {
        bitop!(^(self, other))
    }

    fn not(self) -> ValueResult<Self> {
        Ok(match self {
            DataValue::I8(a) => DataValue::I8(!a),
            DataValue::I16(a) => DataValue::I16(!a),
            DataValue::I32(a) => DataValue::I32(!a),
            DataValue::I64(a) => DataValue::I64(!a),
            DataValue::I128(a) => DataValue::I128(!a),
            DataValue::F32(a) => DataValue::F32(!a),
            DataValue::F64(a) => DataValue::F64(!a),
            DataValue::V128(a) => {
                let mut a2 = a.clone();
                for a in a2.iter_mut() {
                    *a = !*a;
                }
                DataValue::V128(a2)
            }
            _ => unimplemented!(),
        })
    }

    fn count_ones(self) -> ValueResult<Self> {
        unary_match!(count_ones(&self); [I8, I16, I32, I64, I128]; [i8, i16, i32, i64, i128])
    }

    fn leading_ones(self) -> ValueResult<Self> {
        unary_match!(leading_ones(&self); [I8, I16, I32, I64, I128]; [i8, i16, i32, i64, i128])
    }

    fn leading_zeros(self) -> ValueResult<Self> {
        unary_match!(leading_zeros(&self); [I8, I16, I32, I64, I128]; [i8, i16, i32, i64, i128])
    }

    fn trailing_zeros(self) -> ValueResult<Self> {
        unary_match!(trailing_zeros(&self); [I8, I16, I32, I64, I128]; [i8, i16, i32, i64, i128])
    }

    fn reverse_bits(self) -> ValueResult<Self> {
        unary_match!(reverse_bits(&self); [I8, I16, I32, I64, I128])
    }

    fn swap_bytes(self) -> ValueResult<Self> {
        unary_match!(swap_bytes(&self); [I16, I32, I64, I128])
    }

    fn iter_lanes(&self, ty: Type) -> ValueResult<DataValueIterator> {
        DataValueIterator::new(self, ty)
    }
}

/// Iterator for DataValue's
pub struct DataValueIterator {
    ty: Type,
    v: SimdVec<DataValue>,
    idx: usize,
}

impl DataValueIterator {
    fn new(dv: &DataValue, ty: Type) -> Result<Self, ValueError> {
        match extractlanes(dv, ty) {
            Ok(v) => return Ok(Self { ty, v, idx: 0 }),
            Err(err) => return Err(err),
        }
    }
}

impl Iterator for DataValueIterator {
    type Item = DataValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.ty.lane_count() as usize {
            return None;
        }

        let dv = self.v[self.idx].clone();
        self.idx += 1;
        Some(dv)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_iterator_v128() {
        let dv = DataValue::V128([99, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        assert_eq!(simd_sum(dv, types::I8X16), 219);
    }

    #[test]
    fn test_iterator_v128_empty() {
        let dv = DataValue::V128([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(simd_sum(dv, types::I8X16), 0);
    }

    #[test]
    fn test_iterator_v128_ones() {
        let dv = DataValue::V128([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);
        assert_eq!(simd_sum(dv, types::I8X16), 16);
    }

    #[test]
    fn test_iterator_v64_empty() {
        let dv = DataValue::V64([0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(simd_sum(dv, types::I8X8), 0);
    }
    #[test]
    fn test_iterator_v64_ones() {
        let dv = DataValue::V64([1, 1, 1, 1, 1, 1, 1, 1]);
        assert_eq!(simd_sum(dv, types::I8X8), 8);
    }
    #[test]
    fn test_iterator_v64() {
        let dv = DataValue::V64([10, 20, 30, 40, 50, 60, 70, 80]);
        assert_eq!(simd_sum(dv, types::I8X8), 360);
    }

    fn simd_sum(dv: DataValue, ty: types::Type) -> i128 {
        let itr = dv.iter_lanes(ty).unwrap();

        itr.map(|e| {
            if let Some(v) = e.into_int_signed().ok() {
                v
            } else {
                0
            }
        })
        .sum()
    }
}
