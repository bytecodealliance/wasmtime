//! This module gives users to instantiate values that Cranelift understands. These values are used,
//! for example, during interpretation and for wrapping immediates.
use crate::ir::immediates::{Ieee32, Ieee64, Imm64, Offset32};
use crate::ir::{types, ConstantData, Type};
use core::convert::TryInto;
use core::fmt::{self, Display, Formatter};
use thiserror::Error;

/// Represent a data value. Where [Value] is an SSA reference, [DataValue] is the type + value
/// that would be referred to by a [Value].
///
/// [Value]: crate::ir::Value
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq)]
pub enum DataValue {
    B(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(Ieee32),
    F64(Ieee64),
    V128([u8; 16]),
}

impl DataValue {
    /// Try to cast an immediate integer ([Imm64]) to the given Cranelift [Type].
    pub fn from_integer(imm: Imm64, ty: Type) -> Result<DataValue, DataValueCastFailure> {
        match ty {
            types::I8 => Ok(DataValue::I8(imm.bits() as i8)),
            types::I16 => Ok(DataValue::I16(imm.bits() as i16)),
            types::I32 => Ok(DataValue::I32(imm.bits() as i32)),
            types::I64 => Ok(DataValue::I64(imm.bits())),
            _ => Err(DataValueCastFailure::FromImm64(imm, ty)),
        }
    }

    /// Return the Cranelift IR [Type] for this [DataValue].
    pub fn ty(&self) -> Type {
        match self {
            DataValue::B(_) => types::B8, // A default type.
            DataValue::I8(_) => types::I8,
            DataValue::I16(_) => types::I16,
            DataValue::I32(_) => types::I32,
            DataValue::I64(_) => types::I64,
            DataValue::F32(_) => types::F32,
            DataValue::F64(_) => types::F64,
            DataValue::V128(_) => types::I8X16, // A default type.
        }
    }

    /// Return true if the value is a vector (i.e. `DataValue::V128`).
    pub fn is_vector(&self) -> bool {
        match self {
            DataValue::V128(_) => true,
            _ => false,
        }
    }
}

/// Record failures to cast [DataValue].
#[derive(Error, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum DataValueCastFailure {
    #[error("unable to cast data value of type {0} to type {1}")]
    TryInto(Type, Type),
    #[error("unable to cast Imm64({0}) to a data value of type {1}")]
    FromImm64(Imm64, Type),
}

/// Helper for creating conversion implementations for [DataValue].
macro_rules! build_conversion_impl {
    ( $rust_ty:ty, $data_value_ty:ident, $cranelift_ty:ident ) => {
        impl From<$rust_ty> for DataValue {
            fn from(data: $rust_ty) -> Self {
                DataValue::$data_value_ty(data)
            }
        }

        impl TryInto<$rust_ty> for DataValue {
            type Error = DataValueCastFailure;
            fn try_into(self) -> Result<$rust_ty, Self::Error> {
                if let DataValue::$data_value_ty(v) = self {
                    Ok(v)
                } else {
                    Err(DataValueCastFailure::TryInto(
                        self.ty(),
                        types::$cranelift_ty,
                    ))
                }
            }
        }
    };
}
build_conversion_impl!(bool, B, B8);
build_conversion_impl!(i8, I8, I8);
build_conversion_impl!(i16, I16, I16);
build_conversion_impl!(i32, I32, I32);
build_conversion_impl!(i64, I64, I64);
build_conversion_impl!(Ieee32, F32, F32);
build_conversion_impl!(Ieee64, F64, F64);
build_conversion_impl!([u8; 16], V128, I8X16);
impl From<Offset32> for DataValue {
    fn from(o: Offset32) -> Self {
        DataValue::from(Into::<i32>::into(o))
    }
}

impl Display for DataValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DataValue::B(dv) => write!(f, "{}", dv),
            DataValue::I8(dv) => write!(f, "{}", dv),
            DataValue::I16(dv) => write!(f, "{}", dv),
            DataValue::I32(dv) => write!(f, "{}", dv),
            DataValue::I64(dv) => write!(f, "{}", dv),
            // The Ieee* wrappers here print the expected syntax.
            DataValue::F32(dv) => write!(f, "{}", dv),
            DataValue::F64(dv) => write!(f, "{}", dv),
            // Again, for syntax consistency, use ConstantData, which in this case displays as hex.
            DataValue::V128(dv) => write!(f, "{}", ConstantData::from(&dv[..])),
        }
    }
}

/// Helper structure for printing bracket-enclosed vectors of [DataValue]s.
/// - for empty vectors, display `[]`
/// - for single item vectors, display `42`, e.g.
/// - for multiple item vectors, display `[42, 43, 44]`, e.g.
pub struct DisplayDataValues<'a>(pub &'a [DataValue]);

impl<'a> Display for DisplayDataValues<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0.len() == 1 {
            write!(f, "{}", self.0[0])
        } else {
            write!(f, "[")?;
            write_data_value_list(f, &self.0)?;
            write!(f, "]")
        }
    }
}

/// Helper function for displaying `Vec<DataValue>`.
pub fn write_data_value_list(f: &mut Formatter<'_>, list: &[DataValue]) -> fmt::Result {
    match list.len() {
        0 => Ok(()),
        1 => write!(f, "{}", list[0]),
        _ => {
            write!(f, "{}", list[0])?;
            for dv in list.iter().skip(1) {
                write!(f, ", {}", dv)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn type_conversions() {
        assert_eq!(DataValue::B(true).ty(), types::B8);
        assert_eq!(
            TryInto::<bool>::try_into(DataValue::B(false)).unwrap(),
            false
        );
        assert_eq!(
            TryInto::<i32>::try_into(DataValue::B(false)).unwrap_err(),
            DataValueCastFailure::TryInto(types::B8, types::I32)
        );

        assert_eq!(DataValue::V128([0; 16]).ty(), types::I8X16);
        assert_eq!(
            TryInto::<[u8; 16]>::try_into(DataValue::V128([0; 16])).unwrap(),
            [0; 16]
        );
        assert_eq!(
            TryInto::<i32>::try_into(DataValue::V128([0; 16])).unwrap_err(),
            DataValueCastFailure::TryInto(types::I8X16, types::I32)
        );
    }
}
