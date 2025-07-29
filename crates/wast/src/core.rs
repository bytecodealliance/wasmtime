use crate::WastContext;
use anyhow::{Context, Result, anyhow, bail};
use json_from_wast::{CoreConst, FloatConst, V128};
use std::fmt::{Display, LowerHex};
use wasmtime::{Store, Val};

/// Translate from a `script::Value` to a `RuntimeValue`.
pub fn val<T>(ctx: &mut WastContext<T>, v: &CoreConst) -> Result<Val> {
    use CoreConst::*;

    Ok(match v {
        I32 { value } => Val::I32(value.0),
        I64 { value } => Val::I64(value.0),
        F32 { value } => Val::F32(value.to_bits()),
        F64 { value } => Val::F64(value.to_bits()),
        V128(value) => Val::V128(value.to_u128().into()),
        FuncRef {
            value: None | Some(json_from_wast::FuncRef::Null),
        } => Val::FuncRef(None),

        ExternRef {
            value: None | Some(json_from_wast::ExternRef::Null),
        } => Val::ExternRef(None),
        ExternRef {
            value: Some(json_from_wast::ExternRef::Host(x)),
        } => Val::ExternRef(if let Some(rt) = ctx.async_runtime.as_ref() {
            Some(rt.block_on(wasmtime::ExternRef::new_async(&mut ctx.store, x.0))?)
        } else {
            Some(wasmtime::ExternRef::new(&mut ctx.store, x.0)?)
        }),

        AnyRef {
            value: None | Some(json_from_wast::AnyRef::Null),
        } => Val::AnyRef(None),
        AnyRef {
            value: Some(json_from_wast::AnyRef::Host(x)),
        } => {
            let x = if let Some(rt) = ctx.async_runtime.as_ref() {
                rt.block_on(wasmtime::ExternRef::new_async(&mut ctx.store, x.0))?
            } else {
                wasmtime::ExternRef::new(&mut ctx.store, x.0)?
            };
            let x = wasmtime::AnyRef::convert_extern(&mut ctx.store, x)?;
            Val::AnyRef(Some(x))
        }
        NullRef => Val::AnyRef(None),
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

pub fn match_val<T>(store: &mut Store<T>, actual: &Val, expected: &CoreConst) -> Result<()> {
    match (actual, expected) {
        (_, CoreConst::Either { values }) => {
            for expected in values {
                if match_val(store, actual, expected).is_ok() {
                    return Ok(());
                }
            }
            match_val(store, actual, &values[0])
        }

        (Val::I32(a), CoreConst::I32 { value }) => match_int(a, &value.0),
        (Val::I64(a), CoreConst::I64 { value }) => match_int(a, &value.0),

        // Note that these float comparisons are comparing bits, not float
        // values, so we're testing for bit-for-bit equivalence
        (Val::F32(a), CoreConst::F32 { value }) => match_f32(*a, value),
        (Val::F64(a), CoreConst::F64 { value }) => match_f64(*a, value),
        (Val::V128(a), CoreConst::V128(value)) => match_v128(a.as_u128(), value),

        // Null references, or blanket "any reference" assertions
        (Val::FuncRef(None) | Val::ExternRef(None) | Val::AnyRef(None), CoreConst::RefNull)
        | (Val::FuncRef(_), CoreConst::FuncRef { value: None })
        | (Val::AnyRef(_), CoreConst::AnyRef { value: None })
        | (Val::ExternRef(_), CoreConst::ExternRef { value: None })
        | (Val::AnyRef(None), CoreConst::NullRef)
        | (Val::FuncRef(None), CoreConst::NullFuncRef)
        | (Val::ExternRef(None), CoreConst::NullExternRef)
        | (
            Val::FuncRef(None),
            CoreConst::FuncRef {
                value: Some(json_from_wast::FuncRef::Null),
            },
        )
        | (
            Val::AnyRef(None),
            CoreConst::AnyRef {
                value: Some(json_from_wast::AnyRef::Null),
            },
        )
        | (
            Val::ExternRef(None),
            CoreConst::ExternRef {
                value: Some(json_from_wast::ExternRef::Null),
            },
        ) => Ok(()),

        // Ideally we'd compare the actual index, but Wasmtime doesn't expose
        // the raw index a function in the embedder API.
        (
            Val::FuncRef(Some(_)),
            CoreConst::FuncRef {
                value: Some(json_from_wast::FuncRef::Index(_)),
            },
        ) => Ok(()),

        (
            Val::ExternRef(Some(x)),
            CoreConst::ExternRef {
                value: Some(json_from_wast::ExternRef::Host(y)),
            },
        ) => {
            let x = x
                .data(store)?
                .ok_or_else(|| {
                    anyhow!("expected an externref of a u32, found externref without host data")
                })?
                .downcast_ref::<u32>()
                .expect("only u32 externrefs created in wast test suites");
            if *x == y.0 {
                Ok(())
            } else {
                bail!("expected {} found {x}", y.0);
            }
        }

        (Val::AnyRef(Some(x)), CoreConst::EqRef) => {
            if x.is_eqref(store)? {
                Ok(())
            } else {
                bail!("expected an eqref, found {x:?}");
            }
        }
        (Val::AnyRef(Some(x)), CoreConst::I31Ref) => {
            if x.is_i31(store)? {
                Ok(())
            } else {
                bail!("expected a `(ref i31)`, found {x:?}");
            }
        }
        (Val::AnyRef(Some(x)), CoreConst::StructRef) => {
            if x.is_struct(store)? {
                Ok(())
            } else {
                bail!("expected a struct reference, found {x:?}")
            }
        }
        (Val::AnyRef(Some(x)), CoreConst::ArrayRef) => {
            if x.is_array(store)? {
                Ok(())
            } else {
                bail!("expected a array reference, found {x:?}")
            }
        }
        (
            Val::AnyRef(Some(x)),
            CoreConst::AnyRef {
                value: Some(json_from_wast::AnyRef::Host(y)),
            },
        ) => {
            let x = wasmtime::ExternRef::convert_any(&mut *store, *x)?;
            let x = x
                .data(&mut *store)?
                .ok_or_else(|| {
                    anyhow!(
                        "expected anyref of externref of u32, found anyref that is \
                         not a converted externref"
                    )
                })?
                .downcast_ref::<u32>()
                .expect("only u32 externrefs created in wast test suites");
            if *x == y.0 {
                Ok(())
            } else {
                bail!(
                    "expected anyref of externref of {}, found anyref of externref of {x}",
                    y.0
                )
            }
        }

        _ => bail!("expected {expected:?} got {actual:?}"),
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

pub fn match_f32(actual: u32, expected: &FloatConst<f32>) -> Result<()> {
    match expected {
        // Check if an f32 (as u32 bits to avoid possible quieting when moving values in registers, e.g.
        // https://developer.arm.com/documentation/ddi0344/i/neon-and-vfp-programmers-model/modes-of-operation/default-nan-mode?lang=en)
        // is a canonical NaN:
        //  - the sign bit is unspecified,
        //  - the 8-bit exponent is set to all 1s
        //  - the MSB of the payload is set to 1 (a quieted NaN) and all others to 0.
        // See https://webassembly.github.io/spec/core/syntax/values.html#floating-point.
        FloatConst::CanonicalNan => {
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
        FloatConst::ArithmeticNan => {
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
        FloatConst::Value(expected_value) => {
            if actual == expected_value.to_bits() {
                Ok(())
            } else {
                bail!(
                    "expected {:10} / {:#010x}\n\
                     actual   {:10} / {:#010x}",
                    expected_value,
                    expected_value.to_bits(),
                    f32::from_bits(actual),
                    actual,
                )
            }
        }
    }
}

pub fn match_f64(actual: u64, expected: &FloatConst<f64>) -> Result<()> {
    match expected {
        // Check if an f64 (as u64 bits to avoid possible quieting when moving values in registers, e.g.
        // https://developer.arm.com/documentation/ddi0344/i/neon-and-vfp-programmers-model/modes-of-operation/default-nan-mode?lang=en)
        // is a canonical NaN:
        //  - the sign bit is unspecified,
        //  - the 11-bit exponent is set to all 1s
        //  - the MSB of the payload is set to 1 (a quieted NaN) and all others to 0.
        // See https://webassembly.github.io/spec/core/syntax/values.html#floating-point.
        FloatConst::CanonicalNan => {
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
        FloatConst::ArithmeticNan => {
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
        FloatConst::Value(expected_value) => {
            if actual == expected_value.to_bits() {
                Ok(())
            } else {
                bail!(
                    "expected {:18} / {:#018x}\n\
                     actual   {:18} / {:#018x}",
                    expected_value,
                    expected_value.to_bits(),
                    f64::from_bits(actual),
                    actual,
                )
            }
        }
    }
}

fn match_v128(actual: u128, expected: &V128) -> Result<()> {
    match expected {
        V128::I8 { value } => {
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
            if actual == value.map(|i| i.0) {
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
        V128::I16 { value } => {
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
            if actual == value.map(|i| i.0) {
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
        V128::I32 { value } => {
            let actual = [
                extract_lane_as_i32(actual, 0),
                extract_lane_as_i32(actual, 1),
                extract_lane_as_i32(actual, 2),
                extract_lane_as_i32(actual, 3),
            ];
            if actual == value.map(|i| i.0) {
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
        V128::I64 { value } => {
            let actual = [
                extract_lane_as_i64(actual, 0),
                extract_lane_as_i64(actual, 1),
            ];
            if actual == value.map(|i| i.0) {
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
        V128::F32 { value } => {
            for (i, expected) in value.iter().enumerate() {
                let a = extract_lane_as_i32(actual, i) as u32;
                match_f32(a, expected).with_context(|| format!("difference in lane {i}"))?;
            }
            Ok(())
        }
        V128::F64 { value } => {
            for (i, expected) in value.iter().enumerate() {
                let a = extract_lane_as_i64(actual, i) as u64;
                match_f64(a, expected).with_context(|| format!("difference in lane {i}"))?;
            }
            Ok(())
        }
    }
}
