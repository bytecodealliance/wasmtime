//! Generate Wasm values, primarily for differential execution.

use arbitrary::{Arbitrary, Unstructured};
use std::hash::Hash;

/// A value passed to and from evaluation. Note that reference types are not
/// (yet) supported.
///
/// TODO implement correct partial equality comparisons for FP.
#[derive(Clone, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum DiffValue {
    I32(i32),
    I64(i64),
    F32(u32),
    F64(u64),
    V128(u128),
}

impl DiffValue {
    fn ty(&self) -> DiffValueType {
        match self {
            DiffValue::I32(_) => DiffValueType::I32,
            DiffValue::I64(_) => DiffValueType::I64,
            DiffValue::F32(_) => DiffValueType::F32,
            DiffValue::F64(_) => DiffValueType::F64,
            DiffValue::V128(_) => DiffValueType::V128,
        }
    }

    /// Generate a [`DiffValue`] of the given `ty` type.
    ///
    /// This function will bias the returned value 50% of the time towards one
    /// of a set of known values (e.g., NaN, -1, 0, infinity, etc.).
    pub fn arbitrary_of_type(
        u: &mut Unstructured<'_>,
        ty: DiffValueType,
    ) -> arbitrary::Result<Self> {
        use DiffValueType::*;
        let val = match ty {
            I32 => DiffValue::I32(biased_arbitrary_value(u, KNOWN_I32_VALUES)?),
            I64 => DiffValue::I64(biased_arbitrary_value(u, KNOWN_I64_VALUES)?),
            F32 => {
                // TODO once `to_bits` is stable as a `const` function, move
                // this to a `const` definition.
                let known_f32_values = &[
                    f32::NAN.to_bits(),
                    f32::INFINITY.to_bits(),
                    f32::NEG_INFINITY.to_bits(),
                    f32::MIN.to_bits(),
                    (-1.0f32).to_bits(),
                    (0.0f32).to_bits(),
                    (1.0f32).to_bits(),
                    f32::MAX.to_bits(),
                ];
                DiffValue::F32(biased_arbitrary_value(u, known_f32_values)?)
            }
            F64 => {
                // TODO once `to_bits` is stable as a `const` function, move
                // this to a `const` definition.
                let known_f64_values = &[
                    f64::NAN.to_bits(),
                    f64::INFINITY.to_bits(),
                    f64::NEG_INFINITY.to_bits(),
                    f64::MIN.to_bits(),
                    (-1.0f64).to_bits(),
                    (0.0f64).to_bits(),
                    (1.0f64).to_bits(),
                    f64::MAX.to_bits(),
                ];
                DiffValue::F64(biased_arbitrary_value(u, known_f64_values)?)
            }
            V128 => DiffValue::V128(biased_arbitrary_value(u, KNOWN_U128_VALUES)?),
        };
        arbitrary::Result::Ok(val)
    }
}

const KNOWN_I32_VALUES: &[i32] = &[i32::MIN, -1, 0, 1, i32::MAX];
const KNOWN_I64_VALUES: &[i64] = &[i64::MIN, -1, 0, 1, i64::MAX];
const KNOWN_U128_VALUES: &[u128] = &[u128::MIN, 1, u128::MAX];

/// Helper function to pick a known value from the list of `known_values` half
/// the time.
fn biased_arbitrary_value<'a, T>(
    u: &mut Unstructured<'a>,
    known_values: &[T],
) -> arbitrary::Result<T>
where
    T: Arbitrary<'a> + Copy,
{
    let pick_from_known_values: bool = u.arbitrary()?;
    if pick_from_known_values {
        Ok(*u.choose(known_values)?)
    } else {
        u.arbitrary()
    }
}

impl<'a> Arbitrary<'a> for DiffValue {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let ty: DiffValueType = u.arbitrary()?;
        DiffValue::arbitrary_of_type(u, ty)
    }
}

impl Hash for DiffValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ty().hash(state);
        match self {
            DiffValue::I32(n) => n.hash(state),
            DiffValue::I64(n) => n.hash(state),
            DiffValue::F32(n) => n.hash(state),
            DiffValue::F64(n) => n.hash(state),
            DiffValue::V128(n) => n.hash(state),
        }
    }
}

/// Enumerate the supported value types.
#[derive(Clone, Debug, Arbitrary, Hash)]
#[allow(missing_docs)]
pub enum DiffValueType {
    I32,
    I64,
    F32,
    F64,
    V128,
}

impl From<wasmparser::ValType> for DiffValueType {
    fn from(ty: wasmparser::ValType) -> Self {
        use wasmparser::ValType::*;
        match ty {
            I32 => Self::I32,
            I64 => Self::I64,
            F32 => Self::F32,
            F64 => Self::F64,
            V128 => Self::V128,
            FuncRef => unimplemented!(),
            ExternRef => unimplemented!(),
        }
    }
}
