use crate::core;
use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;
use std::fmt::Debug;
use wast::component::WastVal;
use wast::core::NanPattern;

pub use wasmtime::component::*;

pub fn val(v: &WastVal<'_>) -> Result<Val> {
    Ok(match v {
        WastVal::Bool(b) => Val::Bool(*b),
        WastVal::U8(b) => Val::U8(*b),
        WastVal::S8(b) => Val::S8(*b),
        WastVal::U16(b) => Val::U16(*b),
        WastVal::S16(b) => Val::S16(*b),
        WastVal::U32(b) => Val::U32(*b),
        WastVal::S32(b) => Val::S32(*b),
        WastVal::U64(b) => Val::U64(*b),
        WastVal::S64(b) => Val::S64(*b),
        WastVal::F32(b) => Val::Float32(f32::from_bits(b.bits)),
        WastVal::F64(b) => Val::Float64(f64::from_bits(b.bits)),
        WastVal::Char(b) => Val::Char(*b),
        WastVal::String(s) => Val::String(s.to_string().into()),
        WastVal::List(vals) => {
            let vals = vals.iter().map(|v| val(v)).collect::<Result<Vec<_>>>()?;
            Val::List(vals.into())
        }
        WastVal::Record(vals) => {
            let mut fields = Vec::new();
            for (name, v) in vals {
                fields.push((name.to_string(), val(v)?));
            }
            Val::Record(fields.into())
        }
        WastVal::Tuple(vals) => Val::Tuple(
            vals.iter()
                .map(|v| val(v))
                .collect::<Result<Vec<_>>>()?
                .into(),
        ),
        WastVal::Enum(name) => Val::Enum(name.to_string()),
        WastVal::Variant(name, payload) => {
            let payload = payload_val(payload.as_deref())?;
            Val::Variant(name.to_string(), payload)
        }
        WastVal::Option(v) => Val::Option(match v {
            Some(v) => Some(Box::new(val(v)?)),
            None => None,
        }),
        WastVal::Result(v) => Val::Result(match v {
            Ok(v) => Ok(payload_val(v.as_deref())?),
            Err(v) => Err(payload_val(v.as_deref())?),
        }),
        WastVal::Flags(v) => Val::Flags(v.iter().map(|s| s.to_string()).collect()),
    })
}

fn payload_val(v: Option<&WastVal<'_>>) -> Result<Option<Box<Val>>> {
    match v {
        Some(v) => Ok(Some(Box::new(val(v)?))),
        None => Ok(None),
    }
}

pub fn match_val(expected: &WastVal<'_>, actual: &Val) -> Result<()> {
    match expected {
        WastVal::Bool(e) => match actual {
            Val::Bool(a) => match_debug(a, e),
            _ => mismatch(expected, actual),
        },
        WastVal::U8(e) => match actual {
            Val::U8(a) => core::match_int(a, e),
            _ => mismatch(expected, actual),
        },
        WastVal::S8(e) => match actual {
            Val::S8(a) => core::match_int(a, e),
            _ => mismatch(expected, actual),
        },
        WastVal::U16(e) => match actual {
            Val::U16(a) => core::match_int(a, e),
            _ => mismatch(expected, actual),
        },
        WastVal::S16(e) => match actual {
            Val::S16(a) => core::match_int(a, e),
            _ => mismatch(expected, actual),
        },
        WastVal::U32(e) => match actual {
            Val::U32(a) => core::match_int(a, e),
            _ => mismatch(expected, actual),
        },
        WastVal::S32(e) => match actual {
            Val::S32(a) => core::match_int(a, e),
            _ => mismatch(expected, actual),
        },
        WastVal::U64(e) => match actual {
            Val::U64(a) => core::match_int(a, e),
            _ => mismatch(expected, actual),
        },
        WastVal::S64(e) => match actual {
            Val::S64(a) => core::match_int(a, e),
            _ => mismatch(expected, actual),
        },
        WastVal::F32(e) => match actual {
            Val::Float32(a) => core::match_f32(a.to_bits(), &NanPattern::Value(*e)),
            _ => mismatch(expected, actual),
        },
        WastVal::F64(e) => match actual {
            Val::Float64(a) => core::match_f64(a.to_bits(), &NanPattern::Value(*e)),
            _ => mismatch(expected, actual),
        },
        WastVal::Char(e) => match actual {
            Val::Char(a) => match_debug(a, e),
            _ => mismatch(expected, actual),
        },
        WastVal::String(e) => match actual {
            Val::String(a) => match_debug(&a[..], *e),
            _ => mismatch(expected, actual),
        },
        WastVal::List(e) => match actual {
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
        WastVal::Record(e) => match actual {
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
        WastVal::Tuple(e) => match actual {
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
        WastVal::Variant(name, e) => match actual {
            Val::Variant(discr, payload) => {
                if *discr != *name {
                    bail!("expected discriminant `{name}` got `{discr}`");
                }
                match_payload_val(name, e.as_deref(), payload.as_deref())
            }
            _ => mismatch(expected, actual),
        },
        WastVal::Enum(name) => match actual {
            Val::Enum(a) => {
                if *a != *name {
                    bail!("expected discriminant `{name}` got `{a}`");
                } else {
                    Ok(())
                }
            }
            _ => mismatch(expected, actual),
        },
        WastVal::Option(e) => match actual {
            Val::Option(a) => match (e, a) {
                (None, None) => Ok(()),
                (Some(expected), Some(actual)) => match_val(expected, actual),
                (None, Some(_)) => bail!("expected `none`, found `some`"),
                (Some(_), None) => bail!("expected `some`, found `none`"),
            },
            _ => mismatch(expected, actual),
        },
        WastVal::Result(e) => match actual {
            Val::Result(a) => match (e, a) {
                (Ok(_), Err(_)) => bail!("expected `ok`, found `err`"),
                (Err(_), Ok(_)) => bail!("expected `err`, found `ok`"),
                (Err(e), Err(a)) => match_payload_val("err", e.as_deref(), a.as_deref()),
                (Ok(e), Ok(a)) => match_payload_val("ok", e.as_deref(), a.as_deref()),
            },
            _ => mismatch(expected, actual),
        },
        WastVal::Flags(e) => match actual {
            Val::Flags(a) => {
                let expected = e.iter().copied().collect::<BTreeSet<_>>();
                let actual = a.iter().map(|s| s.as_str()).collect::<BTreeSet<_>>();
                match_debug(&actual, &expected)
            }
            _ => mismatch(expected, actual),
        },
    }
}

fn match_payload_val(
    name: &str,
    expected: Option<&WastVal<'_>>,
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

fn mismatch(expected: &WastVal<'_>, actual: &Val) -> Result<()> {
    let expected = match expected {
        WastVal::Bool(..) => "bool",
        WastVal::U8(..) => "u8",
        WastVal::S8(..) => "s8",
        WastVal::U16(..) => "u16",
        WastVal::S16(..) => "s16",
        WastVal::U32(..) => "u32",
        WastVal::S32(..) => "s32",
        WastVal::U64(..) => "u64",
        WastVal::S64(..) => "s64",
        WastVal::F32(..) => "f32",
        WastVal::F64(..) => "f64",
        WastVal::Char(..) => "char",
        WastVal::String(..) => "string",
        WastVal::List(..) => "list",
        WastVal::Record(..) => "record",
        WastVal::Tuple(..) => "tuple",
        WastVal::Enum(..) => "enum",
        WastVal::Variant(..) => "variant",
        WastVal::Option(..) => "option",
        WastVal::Result(..) => "result",
        WastVal::Flags(..) => "flags",
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
    };
    bail!("expected `{expected}` got `{actual}`")
}
