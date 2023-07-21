use crate::core;
use anyhow::{anyhow, bail, Context, Result};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Debug;
use wast::component::WastVal;
use wast::core::NanPattern;

pub use wasmtime::component::*;

pub fn val(v: &WastVal<'_>, ty: &Type) -> Result<Val> {
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
        WastVal::Float32(b) => Val::Float32(f32::from_bits(b.bits)),
        WastVal::Float64(b) => Val::Float64(f64::from_bits(b.bits)),
        WastVal::Char(b) => Val::Char(*b),
        WastVal::String(s) => Val::String(s.to_string().into()),
        WastVal::List(vals) => match ty {
            Type::List(t) => {
                let element = t.ty();
                let vals = vals
                    .iter()
                    .map(|v| val(v, &element))
                    .collect::<Result<Vec<_>>>()?;
                t.new_val(vals.into())?
            }
            _ => bail!("expected a list value"),
        },
        WastVal::Record(fields) => match ty {
            Type::Record(t) => {
                let mut fields_by_name = HashMap::new();
                for (name, val) in fields {
                    let prev = fields_by_name.insert(*name, val);
                    if prev.is_some() {
                        bail!("field `{name}` specified twice");
                    }
                }
                let mut values = Vec::new();
                for field in t.fields() {
                    let name = field.name;
                    let v = fields_by_name
                        .remove(name)
                        .ok_or_else(|| anyhow!("field `{name}` not specified"))?;
                    values.push((name, val(v, &field.ty)?));
                }
                if let Some((field, _)) = fields_by_name.iter().next() {
                    bail!("extra field `{field}` specified");
                }
                t.new_val(values)?
            }
            _ => bail!("expected a record value"),
        },
        WastVal::Tuple(vals) => match ty {
            Type::Tuple(t) => {
                if vals.len() != t.types().len() {
                    bail!("expected {} values got {}", t.types().len(), vals.len());
                }
                t.new_val(
                    vals.iter()
                        .zip(t.types())
                        .map(|(v, ty)| val(v, &ty))
                        .collect::<Result<Vec<_>>>()?
                        .into(),
                )?
            }
            _ => bail!("expected a tuple value"),
        },
        WastVal::Enum(name) => match ty {
            Type::Enum(t) => t.new_val(name)?,
            _ => bail!("expected an enum value"),
        },
        WastVal::Variant(name, payload) => match ty {
            Type::Variant(t) => {
                let case = match t.cases().find(|c| c.name == *name) {
                    Some(case) => case,
                    None => bail!("no case named `{}", name),
                };
                let payload = payload_val(case.name, payload.as_deref(), case.ty.as_ref())?;
                t.new_val(name, payload)?
            }
            _ => bail!("expected a variant value"),
        },
        WastVal::Union(idx, payload) => match ty {
            Type::Union(t) => {
                let case = match t.types().nth(*idx as usize) {
                    Some(case) => case,
                    None => bail!("case {idx} too large"),
                };
                let payload = val(payload, &case)?;
                t.new_val(*idx, payload)?
            }
            _ => bail!("expected a union value"),
        },
        WastVal::Option(v) => match ty {
            Type::Option(t) => {
                let v = match v {
                    Some(v) => Some(val(v, &t.ty())?),
                    None => None,
                };
                t.new_val(v)?
            }
            _ => bail!("expected an option value"),
        },
        WastVal::Result(v) => match ty {
            Type::Result(t) => {
                let v = match v {
                    Ok(v) => Ok(payload_val("ok", v.as_deref(), t.ok().as_ref())?),
                    Err(v) => Err(payload_val("err", v.as_deref(), t.err().as_ref())?),
                };
                t.new_val(v)?
            }
            _ => bail!("expected an expected value"),
        },
        WastVal::Flags(v) => match ty {
            Type::Flags(t) => t.new_val(v)?,
            _ => bail!("expected a flags value"),
        },
    })
}

fn payload_val(name: &str, v: Option<&WastVal<'_>>, ty: Option<&Type>) -> Result<Option<Val>> {
    match (v, ty) {
        (Some(v), Some(ty)) => Ok(Some(val(v, ty)?)),
        (None, None) => Ok(None),
        (Some(_), None) => bail!("expected payload for case `{name}`"),
        (None, Some(_)) => bail!("unexpected payload for case `{name}`"),
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
        WastVal::Float32(e) => match actual {
            Val::Float32(a) => core::match_f32(a.to_bits(), &NanPattern::Value(*e)),
            _ => mismatch(expected, actual),
        },
        WastVal::Float64(e) => match actual {
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
                let mut fields_by_name = HashMap::new();
                for (name, val) in e {
                    let prev = fields_by_name.insert(*name, val);
                    if prev.is_some() {
                        bail!("field `{name}` specified twice");
                    }
                }
                for (name, actual) in a.fields() {
                    let expected = fields_by_name
                        .remove(name)
                        .ok_or_else(|| anyhow!("field `{name}` not specified"))?;
                    match_val(expected, actual)
                        .with_context(|| format!("failed to match field `{name}`"))?;
                }
                if let Some((field, _)) = fields_by_name.iter().next() {
                    bail!("extra field `{field}` specified");
                }
                Ok(())
            }
            _ => mismatch(expected, actual),
        },
        WastVal::Tuple(e) => match actual {
            Val::Tuple(a) => {
                if e.len() != a.values().len() {
                    bail!(
                        "expected {}-tuple, found {}-tuple",
                        e.len(),
                        a.values().len()
                    );
                }
                for (i, (expected, actual)) in e.iter().zip(a.values()).enumerate() {
                    match_val(expected, actual)
                        .with_context(|| format!("failed to match tuple element {i}"))?;
                }
                Ok(())
            }
            _ => mismatch(expected, actual),
        },
        WastVal::Variant(name, e) => match actual {
            Val::Variant(a) => {
                if a.discriminant() != *name {
                    bail!("expected discriminant `{name}` got `{}`", a.discriminant());
                }
                match_payload_val(name, e.as_deref(), a.payload())
            }
            _ => mismatch(expected, actual),
        },
        WastVal::Enum(name) => match actual {
            Val::Enum(a) => {
                if a.discriminant() != *name {
                    bail!("expected discriminant `{name}` got `{}`", a.discriminant());
                } else {
                    Ok(())
                }
            }
            _ => mismatch(expected, actual),
        },
        WastVal::Union(idx, e) => match actual {
            Val::Union(a) => {
                if a.discriminant() != *idx {
                    bail!("expected discriminant `{idx}` got `{}`", a.discriminant());
                }
                match_val(e, a.payload())
            }
            _ => mismatch(expected, actual),
        },
        WastVal::Option(e) => match actual {
            Val::Option(a) => match (e, a.value()) {
                (None, None) => Ok(()),
                (Some(expected), Some(actual)) => match_val(expected, actual),
                (None, Some(_)) => bail!("expected `none`, found `some`"),
                (Some(_), None) => bail!("expected `some`, found `none`"),
            },
            _ => mismatch(expected, actual),
        },
        WastVal::Result(e) => match actual {
            Val::Result(a) => match (e, a.value()) {
                (Ok(_), Err(_)) => bail!("expected `ok`, found `err`"),
                (Err(_), Ok(_)) => bail!("expected `err`, found `ok`"),
                (Err(e), Err(a)) => match_payload_val("err", e.as_deref(), a),
                (Ok(e), Ok(a)) => match_payload_val("ok", e.as_deref(), a),
            },
            _ => mismatch(expected, actual),
        },
        WastVal::Flags(e) => match actual {
            Val::Flags(a) => {
                let expected = e.iter().copied().collect::<BTreeSet<_>>();
                let actual = a.flags().collect::<BTreeSet<_>>();
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
        WastVal::Float32(..) => "float32",
        WastVal::Float64(..) => "float64",
        WastVal::Char(..) => "char",
        WastVal::String(..) => "string",
        WastVal::List(..) => "list",
        WastVal::Record(..) => "record",
        WastVal::Tuple(..) => "tuple",
        WastVal::Enum(..) => "enum",
        WastVal::Variant(..) => "variant",
        WastVal::Union(..) => "union",
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
        Val::Float32(..) => "float32",
        Val::Float64(..) => "float64",
        Val::Char(..) => "char",
        Val::String(..) => "string",
        Val::List(..) => "list",
        Val::Record(..) => "record",
        Val::Tuple(..) => "tuple",
        Val::Enum(..) => "enum",
        Val::Variant(..) => "variant",
        Val::Union(..) => "union",
        Val::Option(..) => "option",
        Val::Result(..) => "result",
        Val::Flags(..) => "flags",
        Val::Resource(..) => "resource",
    };
    bail!("expected `{expected}` got `{actual}`")
}
