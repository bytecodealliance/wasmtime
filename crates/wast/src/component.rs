use crate::core;
use anyhow::{Context, Result, bail};
use json_from_wast::{ComponentConst, FloatConst};
use std::collections::BTreeSet;
use std::fmt::Debug;

pub use wasmtime::component::*;

pub fn val(v: &ComponentConst<'_>) -> Result<Val> {
    Ok(match v {
        ComponentConst::Bool(b) => Val::Bool(*b),
        ComponentConst::U8(b) => Val::U8(b.0),
        ComponentConst::S8(b) => Val::S8(b.0),
        ComponentConst::U16(b) => Val::U16(b.0),
        ComponentConst::S16(b) => Val::S16(b.0),
        ComponentConst::U32(b) => Val::U32(b.0),
        ComponentConst::S32(b) => Val::S32(b.0),
        ComponentConst::U64(b) => Val::U64(b.0),
        ComponentConst::S64(b) => Val::S64(b.0),
        ComponentConst::F32(b) => Val::Float32(f32::from_bits(b.0)),
        ComponentConst::F64(b) => Val::Float64(f64::from_bits(b.0)),
        ComponentConst::Char(b) => Val::Char(*b),
        ComponentConst::String(s) => Val::String(s.to_string()),
        ComponentConst::List(vals) => {
            let vals = vals.iter().map(|v| val(v)).collect::<Result<Vec<_>>>()?;
            Val::List(vals)
        }
        ComponentConst::Record(vals) => {
            let mut fields = Vec::new();
            for (name, v) in vals {
                fields.push((name.to_string(), val(v)?));
            }
            Val::Record(fields)
        }
        ComponentConst::Tuple(vals) => {
            Val::Tuple(vals.iter().map(|v| val(v)).collect::<Result<Vec<_>>>()?)
        }
        ComponentConst::Enum(name) => Val::Enum(name.to_string()),
        ComponentConst::Variant { case, payload } => {
            let payload = payload_val(payload.as_deref())?;
            Val::Variant(case.to_string(), payload)
        }
        ComponentConst::Option(v) => Val::Option(match v {
            Some(v) => Some(Box::new(val(v)?)),
            None => None,
        }),
        ComponentConst::Result(v) => Val::Result(match v {
            Ok(v) => Ok(payload_val(v.as_deref())?),
            Err(v) => Err(payload_val(v.as_deref())?),
        }),
        ComponentConst::Flags(v) => Val::Flags(v.iter().map(|s| s.to_string()).collect()),
    })
}

fn payload_val(v: Option<&ComponentConst<'_>>) -> Result<Option<Box<Val>>> {
    match v {
        Some(v) => Ok(Some(Box::new(val(v)?))),
        None => Ok(None),
    }
}

pub fn match_val(expected: &ComponentConst<'_>, actual: &Val) -> Result<()> {
    match expected {
        ComponentConst::Bool(e) => match actual {
            Val::Bool(a) => match_debug(a, e),
            _ => mismatch(expected, actual),
        },
        ComponentConst::U8(e) => match actual {
            Val::U8(a) => core::match_int(a, &e.0),
            _ => mismatch(expected, actual),
        },
        ComponentConst::S8(e) => match actual {
            Val::S8(a) => core::match_int(a, &e.0),
            _ => mismatch(expected, actual),
        },
        ComponentConst::U16(e) => match actual {
            Val::U16(a) => core::match_int(a, &e.0),
            _ => mismatch(expected, actual),
        },
        ComponentConst::S16(e) => match actual {
            Val::S16(a) => core::match_int(a, &e.0),
            _ => mismatch(expected, actual),
        },
        ComponentConst::U32(e) => match actual {
            Val::U32(a) => core::match_int(a, &e.0),
            _ => mismatch(expected, actual),
        },
        ComponentConst::S32(e) => match actual {
            Val::S32(a) => core::match_int(a, &e.0),
            _ => mismatch(expected, actual),
        },
        ComponentConst::U64(e) => match actual {
            Val::U64(a) => core::match_int(a, &e.0),
            _ => mismatch(expected, actual),
        },
        ComponentConst::S64(e) => match actual {
            Val::S64(a) => core::match_int(a, &e.0),
            _ => mismatch(expected, actual),
        },
        ComponentConst::F32(e) => match actual {
            Val::Float32(a) => {
                core::match_f32(a.to_bits(), &FloatConst::Value(f32::from_bits(e.0)))
            }
            _ => mismatch(expected, actual),
        },
        ComponentConst::F64(e) => match actual {
            Val::Float64(a) => {
                core::match_f64(a.to_bits(), &FloatConst::Value(f64::from_bits(e.0)))
            }
            _ => mismatch(expected, actual),
        },
        ComponentConst::Char(e) => match actual {
            Val::Char(a) => match_debug(a, e),
            _ => mismatch(expected, actual),
        },
        ComponentConst::String(e) => match actual {
            Val::String(a) => match_debug(&a[..], e),
            _ => mismatch(expected, actual),
        },
        ComponentConst::List(e) => match actual {
            Val::List(a) => {
                if e.len() != a.len() {
                    bail!("expected {} values got {}", e.len(), a.len());
                }
                for (i, (expected, actual)) in e.iter().zip(a.iter()).enumerate() {
                    match_val(expected, actual)
                        .with_context(|| format!("failed to match list element {i}"))?;
                }
                Ok(())
            }
            _ => mismatch(expected, actual),
        },
        ComponentConst::Record(e) => match actual {
            Val::Record(a) => {
                if e.len() != e.len() {
                    bail!("mismatched number of record fields");
                }
                for ((e_name, e_val), (a_name, a_val)) in e.iter().zip(a.iter()) {
                    if e_name != a_name {
                        bail!("expected field `{e_name}` got `{a_name}`");
                    }
                    match_val(e_val, a_val)
                        .with_context(|| format!("failed to match field `{e_name}`"))?;
                }
                Ok(())
            }
            _ => mismatch(expected, actual),
        },
        ComponentConst::Tuple(e) => match actual {
            Val::Tuple(a) => {
                if e.len() != a.len() {
                    bail!("expected {}-tuple, found {}-tuple", e.len(), a.len());
                }
                for (i, (expected, actual)) in e.iter().zip(a.iter()).enumerate() {
                    match_val(expected, actual)
                        .with_context(|| format!("failed to match tuple element {i}"))?;
                }
                Ok(())
            }
            _ => mismatch(expected, actual),
        },
        ComponentConst::Variant {
            case: name,
            payload: e,
        } => match actual {
            Val::Variant(discr, payload) => {
                if *discr != *name {
                    bail!("expected discriminant `{name}` got `{discr}`");
                }
                match_payload_val(name, e.as_deref(), payload.as_deref())
            }
            _ => mismatch(expected, actual),
        },
        ComponentConst::Enum(name) => match actual {
            Val::Enum(a) => {
                if *a != *name {
                    bail!("expected discriminant `{name}` got `{a}`");
                } else {
                    Ok(())
                }
            }
            _ => mismatch(expected, actual),
        },
        ComponentConst::Option(e) => match actual {
            Val::Option(a) => match (e, a) {
                (None, None) => Ok(()),
                (Some(expected), Some(actual)) => match_val(expected, actual),
                (None, Some(_)) => bail!("expected `none`, found `some`"),
                (Some(_), None) => bail!("expected `some`, found `none`"),
            },
            _ => mismatch(expected, actual),
        },
        ComponentConst::Result(e) => match actual {
            Val::Result(a) => match (e, a) {
                (Ok(_), Err(_)) => bail!("expected `ok`, found `err`"),
                (Err(_), Ok(_)) => bail!("expected `err`, found `ok`"),
                (Err(e), Err(a)) => match_payload_val("err", e.as_deref(), a.as_deref()),
                (Ok(e), Ok(a)) => match_payload_val("ok", e.as_deref(), a.as_deref()),
            },
            _ => mismatch(expected, actual),
        },
        ComponentConst::Flags(e) => match actual {
            Val::Flags(a) => {
                let expected = e.iter().map(|s| &s[..]).collect::<BTreeSet<_>>();
                let actual = a.iter().map(|s| s.as_str()).collect::<BTreeSet<_>>();
                match_debug(&actual, &expected)
            }
            _ => mismatch(expected, actual),
        },
    }
}

fn match_payload_val(
    name: &str,
    expected: Option<&ComponentConst<'_>>,
    actual: Option<&Val>,
) -> Result<()> {
    match (expected, actual) {
        (Some(e), Some(a)) => {
            match_val(e, a).with_context(|| format!("failed to match case `{name}`"))
        }
        (None, None) => Ok(()),
        (Some(_), None) => bail!("expected payload for case `{name}`"),
        (None, Some(_)) => bail!("unexpected payload for case `{name}`"),
    }
}

fn match_debug<T>(actual: &T, expected: &T) -> Result<()>
where
    T: Eq + Debug + ?Sized,
{
    if actual == expected {
        Ok(())
    } else {
        bail!(
            "
             expected {expected:?}
             actual   {actual:?}"
        )
    }
}

fn mismatch(expected: &ComponentConst<'_>, actual: &Val) -> Result<()> {
    let expected = match expected {
        ComponentConst::Bool(..) => "bool",
        ComponentConst::U8(..) => "u8",
        ComponentConst::S8(..) => "s8",
        ComponentConst::U16(..) => "u16",
        ComponentConst::S16(..) => "s16",
        ComponentConst::U32(..) => "u32",
        ComponentConst::S32(..) => "s32",
        ComponentConst::U64(..) => "u64",
        ComponentConst::S64(..) => "s64",
        ComponentConst::F32(..) => "f32",
        ComponentConst::F64(..) => "f64",
        ComponentConst::Char(..) => "char",
        ComponentConst::String(..) => "string",
        ComponentConst::List(..) => "list",
        ComponentConst::Record(..) => "record",
        ComponentConst::Tuple(..) => "tuple",
        ComponentConst::Enum(..) => "enum",
        ComponentConst::Variant { .. } => "variant",
        ComponentConst::Option(..) => "option",
        ComponentConst::Result(..) => "result",
        ComponentConst::Flags(..) => "flags",
    };
    let actual = match actual {
        Val::Bool(..) => "bool",
        Val::U8(..) => "u8",
        Val::S8(..) => "s8",
        Val::U16(..) => "u16",
        Val::S16(..) => "s16",
        Val::U32(..) => "u32",
        Val::S32(..) => "s32",
        Val::U64(..) => "u64",
        Val::S64(..) => "s64",
        Val::Float32(..) => "f32",
        Val::Float64(..) => "f64",
        Val::Char(..) => "char",
        Val::String(..) => "string",
        Val::List(..) => "list",
        Val::Record(..) => "record",
        Val::Tuple(..) => "tuple",
        Val::Enum(..) => "enum",
        Val::Variant(..) => "variant",
        Val::Option(..) => "option",
        Val::Result(..) => "result",
        Val::Flags(..) => "flags",
        Val::Resource(..) => "resource",
        Val::Future(..) => "future",
        Val::Stream(..) => "stream",
        Val::ErrorContext(..) => "error-context",
    };
    bail!("expected `{expected}` got `{actual}`")
}
