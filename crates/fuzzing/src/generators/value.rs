//! Generate Wasm values, primarily for differential execution.

use arbitrary::{Arbitrary, Unstructured};
use std::hash::Hash;
use wasmtime::HeapType;

/// A value passed to and from evaluation. Note that reference types are not
/// (yet) supported.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub enum DiffValue {
    I32(i32),
    I64(i64),
    F32(u32),
    F64(u64),
    V128(u128),
    FuncRef { null: bool },
    ExternRef { null: bool },
    AnyRef { null: bool },
}

impl DiffValue {
    fn ty(&self) -> DiffValueType {
        match self {
            DiffValue::I32(_) => DiffValueType::I32,
            DiffValue::I64(_) => DiffValueType::I64,
            DiffValue::F32(_) => DiffValueType::F32,
            DiffValue::F64(_) => DiffValueType::F64,
            DiffValue::V128(_) => DiffValueType::V128,
            DiffValue::FuncRef { .. } => DiffValueType::FuncRef,
            DiffValue::ExternRef { .. } => DiffValueType::ExternRef,
            DiffValue::AnyRef { .. } => DiffValueType::AnyRef,
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
                let bits = biased_arbitrary_value(u, known_f32_values)?;

                // If the chosen bits are NaN then always use the canonical bit
                // pattern of NaN to enable better compatibility with engines
                // where arbitrary NaN patterns can't make their way into wasm
                // (e.g. v8 through JS can't do that).
                let bits = if f32::from_bits(bits).is_nan() {
                    f32::NAN.to_bits()
                } else {
                    bits
                };
                DiffValue::F32(bits)
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
                let bits = biased_arbitrary_value(u, known_f64_values)?;
                // See `f32` above for why canonical NaN patterns are always
                // used.
                let bits = if f64::from_bits(bits).is_nan() {
                    f64::NAN.to_bits()
                } else {
                    bits
                };
                DiffValue::F64(bits)
            }
            V128 => {
                // Generate known values for each sub-type of V128.
                let ty: DiffSimdTy = u.arbitrary()?;
                match ty {
                    DiffSimdTy::I8x16 => {
                        let mut i8 = || biased_arbitrary_value(u, KNOWN_I8_VALUES).map(|b| b as u8);
                        let vector = u128::from_le_bytes([
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                            i8()?,
                        ]);
                        DiffValue::V128(vector)
                    }
                    DiffSimdTy::I16x8 => {
                        let mut i16 =
                            || biased_arbitrary_value(u, KNOWN_I16_VALUES).map(i16::to_le_bytes);
                        let vector: Vec<u8> = i16()?
                            .into_iter()
                            .chain(i16()?)
                            .chain(i16()?)
                            .chain(i16()?)
                            .chain(i16()?)
                            .chain(i16()?)
                            .chain(i16()?)
                            .chain(i16()?)
                            .collect();
                        DiffValue::V128(u128::from_le_bytes(vector.try_into().unwrap()))
                    }
                    DiffSimdTy::I32x4 => {
                        let mut i32 =
                            || biased_arbitrary_value(u, KNOWN_I32_VALUES).map(i32::to_le_bytes);
                        let vector: Vec<u8> = i32()?
                            .into_iter()
                            .chain(i32()?)
                            .chain(i32()?)
                            .chain(i32()?)
                            .collect();
                        DiffValue::V128(u128::from_le_bytes(vector.try_into().unwrap()))
                    }
                    DiffSimdTy::I64x2 => {
                        let mut i64 =
                            || biased_arbitrary_value(u, KNOWN_I64_VALUES).map(i64::to_le_bytes);
                        let vector: Vec<u8> = i64()?.into_iter().chain(i64()?).collect();
                        DiffValue::V128(u128::from_le_bytes(vector.try_into().unwrap()))
                    }
                    DiffSimdTy::F32x4 => {
                        let mut f32 = || {
                            Self::arbitrary_of_type(u, DiffValueType::F32).map(|v| match v {
                                DiffValue::F32(v) => v.to_le_bytes(),
                                _ => unreachable!(),
                            })
                        };
                        let vector: Vec<u8> = f32()?
                            .into_iter()
                            .chain(f32()?)
                            .chain(f32()?)
                            .chain(f32()?)
                            .collect();
                        DiffValue::V128(u128::from_le_bytes(vector.try_into().unwrap()))
                    }
                    DiffSimdTy::F64x2 => {
                        let mut f64 = || {
                            Self::arbitrary_of_type(u, DiffValueType::F64).map(|v| match v {
                                DiffValue::F64(v) => v.to_le_bytes(),
                                _ => unreachable!(),
                            })
                        };
                        let vector: Vec<u8> = f64()?.into_iter().chain(f64()?).collect();
                        DiffValue::V128(u128::from_le_bytes(vector.try_into().unwrap()))
                    }
                }
            }

            // TODO: this isn't working in most engines so just always pass a
            // null in which if an engine supports this is should at least
            // support doing that.
            FuncRef => DiffValue::FuncRef { null: true },
            ExternRef => DiffValue::ExternRef { null: true },
            AnyRef => DiffValue::AnyRef { null: true },
        };
        arbitrary::Result::Ok(val)
    }
}

const KNOWN_I8_VALUES: &[i8] = &[i8::MIN, -1, 0, 1, i8::MAX];
const KNOWN_I16_VALUES: &[i16] = &[i16::MIN, -1, 0, 1, i16::MAX];
const KNOWN_I32_VALUES: &[i32] = &[i32::MIN, -1, 0, 1, i32::MAX];
const KNOWN_I64_VALUES: &[i64] = &[i64::MIN, -1, 0, 1, i64::MAX];

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
            DiffValue::ExternRef { null } => null.hash(state),
            DiffValue::FuncRef { null } => null.hash(state),
            DiffValue::AnyRef { null } => null.hash(state),
        }
    }
}

/// Implement equality checks. Note that floating-point values are not compared
/// bit-for-bit in the case of NaNs: because Wasm floating-point numbers may be
/// [arithmetic NaNs with arbitrary payloads] and Wasm operations are [not
/// required to propagate NaN payloads], we simply check that both sides are
/// NaNs here. We could be more strict, though: we could check that the NaN
/// signs are equal and that [canonical NaN payloads remain canonical].
///
/// [arithmetic NaNs with arbitrary payloads]:
///     https://webassembly.github.io/spec/core/bikeshed/index.html#floating-point%E2%91%A0
/// [not required to propagate NaN payloads]:
///     https://webassembly.github.io/spec/core/bikeshed/index.html#floating-point-operations%E2%91%A0
/// [canonical NaN payloads remain canonical]:
///     https://webassembly.github.io/spec/core/bikeshed/index.html#nan-propagation%E2%91%A0
impl PartialEq for DiffValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::I32(l0), Self::I32(r0)) => l0 == r0,
            (Self::I64(l0), Self::I64(r0)) => l0 == r0,
            (Self::V128(l0), Self::V128(r0)) => l0 == r0,
            (Self::F32(l0), Self::F32(r0)) => {
                let l0 = f32::from_bits(*l0);
                let r0 = f32::from_bits(*r0);
                l0 == r0 || (l0.is_nan() && r0.is_nan())
            }
            (Self::F64(l0), Self::F64(r0)) => {
                let l0 = f64::from_bits(*l0);
                let r0 = f64::from_bits(*r0);
                l0 == r0 || (l0.is_nan() && r0.is_nan())
            }
            (Self::FuncRef { null: a }, Self::FuncRef { null: b }) => a == b,
            (Self::ExternRef { null: a }, Self::ExternRef { null: b }) => a == b,
            _ => false,
        }
    }
}

/// Enumerate the supported value types.
#[derive(Copy, Clone, Debug, Arbitrary, Hash)]
#[allow(missing_docs)]
pub enum DiffValueType {
    I32,
    I64,
    F32,
    F64,
    V128,
    FuncRef,
    ExternRef,
    AnyRef,
}

impl TryFrom<wasmtime::ValType> for DiffValueType {
    type Error = &'static str;
    fn try_from(ty: wasmtime::ValType) -> Result<Self, Self::Error> {
        use wasmtime::ValType::*;
        match ty {
            I32 => Ok(Self::I32),
            I64 => Ok(Self::I64),
            F32 => Ok(Self::F32),
            F64 => Ok(Self::F64),
            V128 => Ok(Self::V128),
            Ref(r) => match (r.is_nullable(), r.heap_type()) {
                (true, HeapType::Func) => Ok(Self::FuncRef),
                (true, HeapType::Extern) => Ok(Self::ExternRef),
                (true, HeapType::Any) => Ok(Self::AnyRef),
                (true, HeapType::I31) => Ok(Self::AnyRef),
                (true, HeapType::None) => Ok(Self::AnyRef),
                _ => Err("non-funcref and non-externref reference types are not supported yet"),
            },
        }
    }
}

/// Enumerate the types of v128.
#[derive(Copy, Clone, Debug, Arbitrary, Hash)]
#[allow(missing_docs)]
pub enum DiffSimdTy {
    I8x16,
    I16x8,
    I32x4,
    I64x2,
    F32x4,
    F64x2,
}
