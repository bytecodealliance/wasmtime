use anyhow::{bail, Context, Result};
use std::fmt::{Display, LowerHex};
use wasmtime::{ExternRef, Store, Val};
use wast::core::{AbstractHeapType, HeapType, NanPattern, V128Pattern, WastArgCore, WastRetCore};
use wast::token::{F32, F64};

/// Translate from a `script::Value` to a `RuntimeValue`.
pub fn val<T>(store: &mut Store<T>, v: &WastArgCore<'_>) -> Result<Val> {
    use wast::core::WastArgCore::*;

    Ok(match v {
        I32(x) => Val::I32(*x),
        I64(x) => Val::I64(*x),
        F32(x) => Val::F32(x.bits),
        F64(x) => Val::F64(x.bits),
        V128(x) => Val::V128(u128::from_le_bytes(x.to_le_bytes()).into()),
        RefNull(HeapType::Abstract {
            ty: AbstractHeapType::Extern,
            shared: false,
        }) => Val::ExternRef(None),
        RefNull(HeapType::Abstract {
            ty: AbstractHeapType::Func,
            shared: false,
        }) => Val::FuncRef(None),
        RefNull(HeapType::Abstract {
            ty: AbstractHeapType::Any,
            shared: false,
        }) => Val::AnyRef(None),
        RefNull(HeapType::Abstract {
            shared: false,
            ty: AbstractHeapType::None,
        }) => Val::AnyRef(None),
        RefExtern(x) => Val::ExternRef(Some(ExternRef::new(store, *x)?)),
        other => bail!("couldn't convert {:?} to a runtime value", other),
    })
}

fn extract_lane_as_i8(bytes: u128, lane: usize) -> i8 {
    (bytes >> (lane * 8)) as i8
}

fn extract_lane_as_i16(bytes: u128, lane: usize) -> i16 {
    (bytes >> (lane * 16)) as i16
}

fn extract_lane_as_i32(bytes: u128, lane: usize) -> i32 {
    (bytes >> (lane * 32)) as i32
}

fn extract_lane_as_i64(bytes: u128, lane: usize) -> i64 {
    (bytes >> (lane * 64)) as i64
}

pub fn match_val<T>(store: &Store<T>, actual: &Val, expected: &WastRetCore) -> Result<()> {
    match (actual, expected) {
        (_, WastRetCore::Either(expected)) => {
            for expected in expected {
                if match_val(store, actual, expected).is_ok() {
                    return Ok(());
                }
            }
            match_val(store, actual, &expected[0])
        }

        (Val::I32(a), WastRetCore::I32(b)) => match_int(a, b),
        (Val::I64(a), WastRetCore::I64(b)) => match_int(a, b),

        // Note that these float comparisons are comparing bits, not float
        // values, so we're testing for bit-for-bit equivalence
        (Val::F32(a), WastRetCore::F32(b)) => match_f32(*a, b),
        (Val::F64(a), WastRetCore::F64(b)) => match_f64(*a, b),
        (Val::V128(a), WastRetCore::V128(b)) => match_v128(a.as_u128(), b),

        // Null references.
        (
            Val::FuncRef(None) | Val::ExternRef(None) | Val::AnyRef(None),
            WastRetCore::RefNull(_),
        )
        | (Val::ExternRef(None), WastRetCore::RefExtern(None)) => Ok(()),

        // Null and non-null mismatches.
        (Val::ExternRef(None), WastRetCore::RefExtern(Some(_))) => {
            bail!("expected non-null reference, found null")
        }
        (
            Val::ExternRef(Some(x)),
            WastRetCore::RefNull(Some(HeapType::Abstract {
                ty: AbstractHeapType::Extern,
                shared: false,
            })),
        ) => {
            let x = x
                .data(store)?
                .downcast_ref::<u32>()
                .expect("only u32 externrefs created in wast test suites");
            bail!("expected null externref, found non-null externref of {x}");
        }
        (Val::ExternRef(Some(_)) | Val::FuncRef(Some(_)), WastRetCore::RefNull(_)) => {
            bail!("expected null, found non-null reference: {actual:?}")
        }

        // Non-null references.
        (Val::FuncRef(Some(_)), WastRetCore::RefFunc(_)) => Ok(()),
        (Val::ExternRef(Some(x)), WastRetCore::RefExtern(Some(y))) => {
            let x = x
                .data(store)?
                .downcast_ref::<u32>()
                .expect("only u32 externrefs created in wast test suites");
            if x == y {
                Ok(())
            } else {
                bail!("expected {} found {}", y, x);
            }
        }

        (Val::AnyRef(Some(_)), WastRetCore::RefAny) => Ok(()),
        (Val::AnyRef(Some(x)), WastRetCore::RefEq) => {
            if x.is_eqref(store)? {
                Ok(())
            } else {
                bail!("expected an eqref, found {x:?}");
            }
        }
        (Val::AnyRef(Some(x)), WastRetCore::RefI31) => {
            if x.is_i31(store)? {
                Ok(())
            } else {
                bail!("expected a `(ref i31)`, found {x:?}");
            }
        }
        (Val::AnyRef(Some(x)), WastRetCore::RefStruct) => {
            if x.is_struct(store)? {
                Ok(())
            } else {
                bail!("expected a struct reference, found {x:?}")
            }
        }
        (Val::AnyRef(Some(x)), WastRetCore::RefArray) => {
            if x.is_array(store)? {
                Ok(())
            } else {
                bail!("expected a array reference, found {x:?}")
            }
        }

        _ => bail!(
            "don't know how to compare {:?} and {:?} yet",
            actual,
            expected
        ),
    }
}

pub fn match_int<T>(actual: &T, expected: &T) -> Result<()>
where
    T: Eq + Display + LowerHex,
{
    if actual == expected {
        Ok(())
    } else {
        bail!(
            "expected {:18} / {0:#018x}\n\
             actual   {:18} / {1:#018x}",
            expected,
            actual
        )
    }
}

pub fn match_f32(actual: u32, expected: &NanPattern<F32>) -> Result<()> {
    match expected {
        // Check if an f32 (as u32 bits to avoid possible quieting when moving values in registers, e.g.
        // https://developer.arm.com/documentation/ddi0344/i/neon-and-vfp-programmers-model/modes-of-operation/default-nan-mode?lang=en)
        // is a canonical NaN:
        //  - the sign bit is unspecified,
        //  - the 8-bit exponent is set to all 1s
        //  - the MSB of the payload is set to 1 (a quieted NaN) and all others to 0.
        // See https://webassembly.github.io/spec/core/syntax/values.html#floating-point.
        NanPattern::CanonicalNan => {
            let canon_nan = 0x7fc0_0000;
            if (actual & 0x7fff_ffff) == canon_nan {
                Ok(())
            } else {
                bail!(
                    "expected {:10} / {:#010x}\n\
                     actual   {:10} / {:#010x}",
                    "canon-nan",
                    canon_nan,
                    f32::from_bits(actual),
                    actual,
                )
            }
        }

        // Check if an f32 (as u32, see comments above) is an arithmetic NaN.
        // This is the same as a canonical NaN including that the payload MSB is
        // set to 1, but one or more of the remaining payload bits MAY BE set to
        // 1 (a canonical NaN specifies all 0s). See
        // https://webassembly.github.io/spec/core/syntax/values.html#floating-point.
        NanPattern::ArithmeticNan => {
            const AF32_NAN: u32 = 0x7f80_0000;
            let is_nan = actual & AF32_NAN == AF32_NAN;
            const AF32_PAYLOAD_MSB: u32 = 0x0040_0000;
            let is_msb_set = actual & AF32_PAYLOAD_MSB == AF32_PAYLOAD_MSB;
            if is_nan && is_msb_set {
                Ok(())
            } else {
                bail!(
                    "expected {:>10} / {:>10}\n\
                     actual   {:10} / {:#010x}",
                    "arith-nan",
                    "0x7fc*****",
                    f32::from_bits(actual),
                    actual,
                )
            }
        }
        NanPattern::Value(expected_value) => {
            if actual == expected_value.bits {
                Ok(())
            } else {
                bail!(
                    "expected {:10} / {:#010x}\n\
                     actual   {:10} / {:#010x}",
                    f32::from_bits(expected_value.bits),
                    expected_value.bits,
                    f32::from_bits(actual),
                    actual,
                )
            }
        }
    }
}

pub fn match_f64(actual: u64, expected: &NanPattern<F64>) -> Result<()> {
    match expected {
        // Check if an f64 (as u64 bits to avoid possible quieting when moving values in registers, e.g.
        // https://developer.arm.com/documentation/ddi0344/i/neon-and-vfp-programmers-model/modes-of-operation/default-nan-mode?lang=en)
        // is a canonical NaN:
        //  - the sign bit is unspecified,
        //  - the 11-bit exponent is set to all 1s
        //  - the MSB of the payload is set to 1 (a quieted NaN) and all others to 0.
        // See https://webassembly.github.io/spec/core/syntax/values.html#floating-point.
        NanPattern::CanonicalNan => {
            let canon_nan = 0x7ff8_0000_0000_0000;
            if (actual & 0x7fff_ffff_ffff_ffff) == canon_nan {
                Ok(())
            } else {
                bail!(
                    "expected {:18} / {:#018x}\n\
                     actual   {:18} / {:#018x}",
                    "canon-nan",
                    canon_nan,
                    f64::from_bits(actual),
                    actual,
                )
            }
        }

        // Check if an f64 (as u64, see comments above) is an arithmetic NaN. This is the same as a
        // canonical NaN including that the payload MSB is set to 1, but one or more of the remaining
        // payload bits MAY BE set to 1 (a canonical NaN specifies all 0s). See
        // https://webassembly.github.io/spec/core/syntax/values.html#floating-point.
        NanPattern::ArithmeticNan => {
            const AF64_NAN: u64 = 0x7ff0_0000_0000_0000;
            let is_nan = actual & AF64_NAN == AF64_NAN;
            const AF64_PAYLOAD_MSB: u64 = 0x0008_0000_0000_0000;
            let is_msb_set = actual & AF64_PAYLOAD_MSB == AF64_PAYLOAD_MSB;
            if is_nan && is_msb_set {
                Ok(())
            } else {
                bail!(
                    "expected {:>18} / {:>18}\n\
                     actual   {:18} / {:#018x}",
                    "arith-nan",
                    "0x7ff8************",
                    f64::from_bits(actual),
                    actual,
                )
            }
        }
        NanPattern::Value(expected_value) => {
            if actual == expected_value.bits {
                Ok(())
            } else {
                bail!(
                    "expected {:18} / {:#018x}\n\
                     actual   {:18} / {:#018x}",
                    f64::from_bits(expected_value.bits),
                    expected_value.bits,
                    f64::from_bits(actual),
                    actual,
                )
            }
        }
    }
}

fn match_v128(actual: u128, expected: &V128Pattern) -> Result<()> {
    match expected {
        V128Pattern::I8x16(expected) => {
            let actual = [
                extract_lane_as_i8(actual, 0),
                extract_lane_as_i8(actual, 1),
                extract_lane_as_i8(actual, 2),
                extract_lane_as_i8(actual, 3),
                extract_lane_as_i8(actual, 4),
                extract_lane_as_i8(actual, 5),
                extract_lane_as_i8(actual, 6),
                extract_lane_as_i8(actual, 7),
                extract_lane_as_i8(actual, 8),
                extract_lane_as_i8(actual, 9),
                extract_lane_as_i8(actual, 10),
                extract_lane_as_i8(actual, 11),
                extract_lane_as_i8(actual, 12),
                extract_lane_as_i8(actual, 13),
                extract_lane_as_i8(actual, 14),
                extract_lane_as_i8(actual, 15),
            ];
            if actual == *expected {
                return Ok(());
            }
            bail!(
                "expected {:4?}\n\
                 actual   {:4?}\n\
                 \n\
                 expected (hex) {0:02x?}\n\
                 actual (hex)   {1:02x?}",
                expected,
                actual,
            )
        }
        V128Pattern::I16x8(expected) => {
            let actual = [
                extract_lane_as_i16(actual, 0),
                extract_lane_as_i16(actual, 1),
                extract_lane_as_i16(actual, 2),
                extract_lane_as_i16(actual, 3),
                extract_lane_as_i16(actual, 4),
                extract_lane_as_i16(actual, 5),
                extract_lane_as_i16(actual, 6),
                extract_lane_as_i16(actual, 7),
            ];
            if actual == *expected {
                return Ok(());
            }
            bail!(
                "expected {:6?}\n\
                 actual   {:6?}\n\
                 \n\
                 expected (hex) {0:04x?}\n\
                 actual (hex)   {1:04x?}",
                expected,
                actual,
            )
        }
        V128Pattern::I32x4(expected) => {
            let actual = [
                extract_lane_as_i32(actual, 0),
                extract_lane_as_i32(actual, 1),
                extract_lane_as_i32(actual, 2),
                extract_lane_as_i32(actual, 3),
            ];
            if actual == *expected {
                return Ok(());
            }
            bail!(
                "expected {:11?}\n\
                 actual   {:11?}\n\
                 \n\
                 expected (hex) {0:08x?}\n\
                 actual (hex)   {1:08x?}",
                expected,
                actual,
            )
        }
        V128Pattern::I64x2(expected) => {
            let actual = [
                extract_lane_as_i64(actual, 0),
                extract_lane_as_i64(actual, 1),
            ];
            if actual == *expected {
                return Ok(());
            }
            bail!(
                "expected {:20?}\n\
                 actual   {:20?}\n\
                 \n\
                 expected (hex) {0:016x?}\n\
                 actual (hex)   {1:016x?}",
                expected,
                actual,
            )
        }
        V128Pattern::F32x4(expected) => {
            for (i, expected) in expected.iter().enumerate() {
                let a = extract_lane_as_i32(actual, i) as u32;
                match_f32(a, expected).with_context(|| format!("difference in lane {i}"))?;
            }
            Ok(())
        }
        V128Pattern::F64x2(expected) => {
            for (i, expected) in expected.iter().enumerate() {
                let a = extract_lane_as_i64(actual, i) as u64;
                match_f64(a, expected).with_context(|| format!("difference in lane {i}"))?;
            }
            Ok(())
        }
    }
}
