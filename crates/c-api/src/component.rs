use anyhow::{anyhow, bail, ensure, Context, Result};
use wasmtime::component::{Component, Func, Instance, Linker, LinkerInstance, Type, Val};
use wasmtime::{AsContext, AsContextMut};

use crate::{
    bad_utf8, declare_vecs, handle_call_error, handle_result, to_str, wasm_byte_vec_t,
    wasm_config_t, wasm_engine_t, wasm_name_t, wasm_trap_t, wasmtime_error_t,
    WasmtimeStoreContextMut, WasmtimeStoreData,
};
use core::ffi::c_void;
use std::collections::HashMap;
use std::ops::Deref;
use std::{mem, mem::MaybeUninit, ptr, slice, str};

#[no_mangle]
pub extern "C" fn wasmtime_config_component_model_set(c: &mut wasm_config_t, enable: bool) {
    c.config.wasm_component_model(enable);
}

pub type wasmtime_component_string_t = wasm_byte_vec_t;

#[repr(C)]
#[derive(Clone)]
pub struct wasmtime_component_val_record_field_t {
    pub name: wasm_name_t,
    pub val: wasmtime_component_val_t,
}

impl Default for wasmtime_component_val_record_field_t {
    fn default() -> Self {
        Self {
            name: Vec::new().into(),
            val: Default::default(),
        }
    }
}

declare_vecs! {
    (
        name: wasmtime_component_val_vec_t,
        ty: wasmtime_component_val_t,
        new: wasmtime_component_val_vec_new,
        empty: wasmtime_component_val_vec_new_empty,
        uninit: wasmtime_component_val_vec_new_uninitialized,
        copy: wasmtime_component_val_vec_copy,
        delete: wasmtime_component_val_vec_delete,
    )
    (
        name: wasmtime_component_val_record_t,
        ty: wasmtime_component_val_record_field_t,
        new: wasmtime_component_val_record_new,
        empty: wasmtime_component_val_record_new_empty,
        uninit: wasmtime_component_val_record_new_uninitialized,
        copy: wasmtime_component_val_record_copy,
        delete: wasmtime_component_val_record_delete,
    )
    (
        name: wasmtime_component_val_flags_t,
        ty: u32,
        new: wasmtime_component_val_flags_new,
        empty: wasmtime_component_val_flags_new_empty,
        uninit: wasmtime_component_val_flags_new_uninitialized,
        copy: wasmtime_component_val_flags_copy,
        delete: wasmtime_component_val_flags_delete,
    )
}

#[repr(C)]
#[derive(Clone)]
pub struct wasmtime_component_val_variant_t {
    pub discriminant: u32,
    pub val: Option<Box<wasmtime_component_val_t>>,
}

#[repr(C)]
#[derive(Clone)]
pub struct wasmtime_component_val_result_t {
    pub value: Option<Box<wasmtime_component_val_t>>,
    pub error: bool,
}

#[repr(C)]
#[derive(Clone)]
pub struct wasmtime_component_val_enum_t {
    pub discriminant: u32,
}

#[no_mangle]
pub extern "C" fn wasmtime_component_val_flags_set(
    flags: &mut wasmtime_component_val_flags_t,
    index: u32,
    enabled: bool,
) {
    let mut f = flags.take();
    let (idx, bit) = ((index / u32::BITS) as usize, index % u32::BITS);
    if idx >= f.len() {
        f.resize(idx + 1, Default::default());
    }
    if enabled {
        f[idx] |= 1 << (bit);
    } else {
        f[idx] &= !(1 << (bit));
    }
    flags.set_buffer(f);
}

#[no_mangle]
pub extern "C" fn wasmtime_component_val_flags_test(
    flags: &wasmtime_component_val_flags_t,
    index: u32,
) -> bool {
    let flags = flags.as_slice();
    let (idx, bit) = ((index / u32::BITS) as usize, index % u32::BITS);
    flags.get(idx).map(|v| v & (1 << bit) != 0).unwrap_or(false)
}

#[repr(C, u8)]
#[derive(Clone)]
pub enum wasmtime_component_val_t {
    Bool(bool),
    S8(i8),
    U8(u8),
    S16(i16),
    U16(u16),
    S32(i32),
    U32(u32),
    S64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Char(char),
    String(wasmtime_component_string_t),
    List(wasmtime_component_val_vec_t),
    Record(wasmtime_component_val_record_t),
    Tuple(wasmtime_component_val_vec_t),
    Variant(wasmtime_component_val_variant_t),
    Enum(wasmtime_component_val_enum_t),
    Option(Option<Box<wasmtime_component_val_t>>),
    Result(wasmtime_component_val_result_t),
    Flags(wasmtime_component_val_flags_t),
}

macro_rules! ensure_type {
    ($ty:ident, $variant:pat) => {
        ensure!(
            matches!($ty, $variant),
            "attempted to create a {} for a {}",
            $ty.desc(),
            stringify!($variant)
        );
    };
}

// a c_api value and its associated Type (from the component model runtime)
struct TypedCVal<'a>(&'a wasmtime_component_val_t, &'a Type);

impl TryFrom<TypedCVal<'_>> for Val {
    type Error = anyhow::Error;
    fn try_from(value: TypedCVal) -> Result<Self> {
        let (value, ty) = (value.0, value.1);
        Ok(match value {
            &wasmtime_component_val_t::Bool(b) => {
                ensure_type!(ty, Type::Bool);
                Val::Bool(b)
            }
            &wasmtime_component_val_t::S8(v) => {
                ensure_type!(ty, Type::S8);
                Val::S8(v)
            }
            &wasmtime_component_val_t::U8(v) => {
                ensure_type!(ty, Type::U8);
                Val::U8(v)
            }
            &wasmtime_component_val_t::S16(v) => {
                ensure_type!(ty, Type::S16);
                Val::S16(v)
            }
            &wasmtime_component_val_t::U16(v) => {
                ensure_type!(ty, Type::U16);
                Val::U16(v)
            }
            &wasmtime_component_val_t::S32(v) => {
                ensure_type!(ty, Type::S32);
                Val::S32(v)
            }
            &wasmtime_component_val_t::U32(v) => {
                ensure_type!(ty, Type::U32);
                Val::U32(v)
            }
            &wasmtime_component_val_t::S64(v) => {
                ensure_type!(ty, Type::S64);
                Val::S64(v)
            }
            &wasmtime_component_val_t::U64(v) => {
                ensure_type!(ty, Type::U64);
                Val::U64(v)
            }
            &wasmtime_component_val_t::F32(v) => {
                ensure_type!(ty, Type::Float32);
                Val::Float32(v)
            }
            &wasmtime_component_val_t::F64(v) => {
                ensure_type!(ty, Type::Float64);
                Val::Float64(v)
            }
            &wasmtime_component_val_t::Char(v) => {
                ensure_type!(ty, Type::Char);
                Val::Char(v)
            }
            wasmtime_component_val_t::String(v) => {
                ensure_type!(ty, Type::String);
                Val::String(String::from_utf8(v.as_slice().to_vec())?)
            }
            wasmtime_component_val_t::List(v) => {
                if let Type::List(ty) = ty {
                    Val::List(
                        v.as_slice()
                            .iter()
                            .map(|v| TypedCVal(v, &ty.ty()).try_into())
                            .collect::<Result<Vec<_>>>()?,
                    )
                } else {
                    bail!("attempted to create a list for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Record(v) => {
                if let Type::Record(ty) = ty {
                    let mut field_vals: HashMap<&[u8], &wasmtime_component_val_t> =
                        HashMap::from_iter(
                            v.as_slice().iter().map(|f| (f.name.as_slice(), &f.val)),
                        );
                    let field_tys = ty.fields();
                    Val::Record(
                        field_tys
                            .map(|tyf| {
                                if let Some(v) = field_vals.remove(tyf.name.as_bytes()) {
                                    Ok((tyf.name.to_string(), TypedCVal(v, &tyf.ty).try_into()?))
                                } else {
                                    bail!("record missing field: {}", tyf.name);
                                }
                            })
                            .collect::<Result<Vec<_>>>()?,
                    )
                } else {
                    bail!("attempted to create a record for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Tuple(v) => {
                if let Type::Tuple(ty) = ty {
                    Val::Tuple(
                        ty.types()
                            .zip(v.as_slice().iter())
                            .map(|(ty, v)| TypedCVal(v, &ty).try_into())
                            .collect::<Result<Vec<_>>>()?,
                    )
                } else {
                    bail!("attempted to create a tuple for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Variant(v) => {
                if let Type::Variant(ty) = ty {
                    let case = ty
                        .cases()
                        .nth(v.discriminant as usize)
                        .with_context(|| format!("missing variant {}", v.discriminant))?;
                    ensure!(
                        case.ty.is_some() == v.val.is_some(),
                        "variant type mismatch: {}",
                        case.ty.map(|ty| ty.desc()).unwrap_or("none")
                    );
                    if let (Some(t), Some(v)) = (case.ty, &v.val) {
                        let v = TypedCVal(v.as_ref(), &t).try_into()?;
                        Val::Variant(case.name.to_string(), Some(Box::new(v)))
                    } else {
                        Val::Variant(case.name.to_string(), None)
                    }
                } else {
                    bail!("attempted to create a variant for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Enum(v) => {
                if let Type::Enum(ty) = ty {
                    let name = ty
                        .names()
                        .nth(v.discriminant as usize)
                        .with_context(|| format!("missing enumeration {}", v.discriminant))?;
                    Val::Enum(name.to_string())
                } else {
                    bail!("attempted to create an enum for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Option(v) => {
                if let Type::Option(ty) = ty {
                    Val::Option(match v {
                        Some(v) => Some(Box::new(TypedCVal(v.as_ref(), &ty.ty()).try_into()?)),
                        None => None,
                    })
                } else {
                    bail!("attempted to create an option for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Result(v) => {
                if let Type::Result(ty) = ty {
                    if v.error {
                        match &v.value {
                            Some(v) => {
                                let ty = ty.err().context("expected err type")?;
                                Val::Result(Err(Some(Box::new(
                                    TypedCVal(v.as_ref(), &ty).try_into()?,
                                ))))
                            }
                            None => {
                                ensure!(ty.err().is_none(), "expected no err type");
                                Val::Result(Err(None))
                            }
                        }
                    } else {
                        match &v.value {
                            Some(v) => {
                                let ty = ty.ok().context("expected ok type")?;
                                Val::Result(Ok(Some(Box::new(
                                    TypedCVal(v.as_ref(), &ty).try_into()?,
                                ))))
                            }
                            None => {
                                ensure!(ty.ok().is_none(), "expected no ok type");
                                Val::Result(Ok(None))
                            }
                        }
                    }
                } else {
                    bail!("attempted to create a result for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Flags(flags) => {
                if let Type::Flags(ty) = ty {
                    let mut set = Vec::new();
                    for (idx, name) in ty.names().enumerate() {
                        if wasmtime_component_val_flags_test(&flags, idx as u32) {
                            set.push(name.to_string());
                        }
                    }
                    Val::Flags(set)
                } else {
                    bail!("attempted to create a flags for a {}", ty.desc());
                }
            }
        })
    }
}

// a Val and its associated wasmtime_component_type_t (from the c_api)
struct CTypedVal<'a>(&'a Val, &'a CType);

impl TryFrom<CTypedVal<'_>> for wasmtime_component_val_t {
    type Error = anyhow::Error;
    fn try_from(value: CTypedVal) -> Result<Self> {
        let (value, ty) = (value.0, value.1);
        Ok(match value {
            Val::Bool(v) => {
                ensure_type!(ty, CType::Bool);
                wasmtime_component_val_t::Bool(*v)
            }
            Val::S8(v) => {
                ensure_type!(ty, CType::S8);
                wasmtime_component_val_t::S8(*v)
            }
            Val::U8(v) => {
                ensure_type!(ty, CType::U8);
                wasmtime_component_val_t::U8(*v)
            }
            Val::S16(v) => {
                ensure_type!(ty, CType::S16);
                wasmtime_component_val_t::S16(*v)
            }
            Val::U16(v) => {
                ensure_type!(ty, CType::U16);
                wasmtime_component_val_t::U16(*v)
            }
            Val::S32(v) => {
                ensure_type!(ty, CType::S32);
                wasmtime_component_val_t::S32(*v)
            }
            Val::U32(v) => {
                ensure_type!(ty, CType::U32);
                wasmtime_component_val_t::U32(*v)
            }
            Val::S64(v) => {
                ensure_type!(ty, CType::S64);
                wasmtime_component_val_t::S64(*v)
            }
            Val::U64(v) => {
                ensure_type!(ty, CType::U64);
                wasmtime_component_val_t::U64(*v)
            }
            Val::Float32(v) => {
                ensure_type!(ty, CType::F32);
                wasmtime_component_val_t::F32(*v)
            }
            Val::Float64(v) => {
                ensure_type!(ty, CType::F64);
                wasmtime_component_val_t::F64(*v)
            }
            Val::Char(v) => {
                ensure_type!(ty, CType::Char);
                wasmtime_component_val_t::Char(*v)
            }
            Val::String(v) => {
                ensure_type!(ty, CType::String);
                wasmtime_component_val_t::String(v.clone().into_bytes().into())
            }
            Val::List(vec) => {
                if let CType::List(ty) = ty {
                    wasmtime_component_val_t::List(
                        vec.iter()
                            .map(|v| CTypedVal(v, ty.deref()).try_into())
                            .collect::<Result<Vec<_>>>()?
                            .into(),
                    )
                } else {
                    bail!("attempted to create a List for a {}", ty.desc());
                }
            }
            Val::Record(vec) => {
                if let CType::Record(ty) = ty {
                    let mut field_vals: HashMap<&str, &Val> =
                        HashMap::from_iter(vec.iter().map(|f| (f.0.as_str(), &f.1)));

                    wasmtime_component_val_t::Record(
                        ty.iter()
                            .map(|(field_name, field_type)| {
                                match field_vals.remove(field_name.as_str()) {
                                    Some(v) => Ok(wasmtime_component_val_record_field_t {
                                        name: field_name.clone().into_bytes().into(),
                                        val: CTypedVal(v, field_type).try_into()?,
                                    }),
                                    None => bail!("missing field {} in record", field_name),
                                }
                            })
                            .collect::<Result<Vec<_>>>()?
                            .into(),
                    )
                } else {
                    bail!("attempted to create a Record for a {}", ty.desc());
                }
            }
            Val::Tuple(vec) => {
                if let CType::Tuple(ty) = ty {
                    wasmtime_component_val_t::Tuple(
                        vec.iter()
                            .zip(ty.iter())
                            .map(|(v, ty)| CTypedVal(v, ty).try_into())
                            .collect::<Result<Vec<_>>>()?
                            .into(),
                    )
                } else {
                    bail!("attempted to create a Tuple for a {}", ty.desc());
                }
            }
            Val::Variant(case, val) => {
                if let CType::Variant(ty) = ty {
                    let index = ty
                        .iter()
                        .position(|c| &c.0 == case)
                        .context(format!("case {case} not found in type"))?;
                    ensure!(
                        val.is_some() == ty[index].1.is_some(),
                        "mismatched variant case {} : value is {}, but type is {}",
                        case,
                        if val.is_some() { "some" } else { "none" },
                        ty[index].1.as_ref().map(|ty| ty.desc()).unwrap_or("none")
                    );
                    wasmtime_component_val_t::Variant(wasmtime_component_val_variant_t {
                        discriminant: index as u32,
                        val: match val {
                            Some(val) => Some(Box::new(
                                CTypedVal(val.deref(), ty[index].1.as_ref().unwrap()).try_into()?,
                            )),
                            None => None,
                        },
                    })
                } else {
                    bail!("attempted to create a Variant for a {}", ty.desc());
                }
            }
            Val::Enum(v) => {
                if let CType::Enum(ty) = ty {
                    let index = ty
                        .iter()
                        .position(|s| s == v)
                        .context(format!("enum value {v} not found in type"))?;
                    wasmtime_component_val_t::Enum(wasmtime_component_val_enum_t {
                        discriminant: index as u32,
                    })
                } else {
                    bail!("attempted to create a Enum for a {}", ty.desc());
                }
            }
            Val::Option(val) => {
                if let CType::Option(ty) = ty {
                    wasmtime_component_val_t::Option(match val {
                        Some(val) => Some(Box::new(CTypedVal(val.deref(), ty.deref()).try_into()?)),
                        None => None,
                    })
                } else {
                    bail!("attempted to create a Option for a {}", ty.desc());
                }
            }
            Val::Result(val) => {
                if let CType::Result(ok_type, err_type) = ty {
                    wasmtime_component_val_t::Result(match val {
                        Ok(Some(ok)) => {
                            let ok_type = ok_type
                                .as_ref()
                                .context("some ok result found instead of none")?;
                            wasmtime_component_val_result_t {
                                value: Some(Box::new(
                                    CTypedVal(ok.deref(), ok_type.deref()).try_into()?,
                                )),
                                error: false,
                            }
                        }
                        Ok(None) => {
                            ensure!(
                                ok_type.is_none(),
                                "none ok result found instead of {}",
                                ok_type.as_ref().unwrap().desc()
                            );
                            wasmtime_component_val_result_t {
                                value: None,
                                error: false,
                            }
                        }
                        Err(Some(err)) => {
                            let err_type = err_type
                                .as_ref()
                                .context("some err result found instead of none")?;
                            wasmtime_component_val_result_t {
                                value: Some(Box::new(
                                    CTypedVal(err.deref(), err_type.deref()).try_into()?,
                                )),
                                error: true,
                            }
                        }
                        Err(None) => {
                            ensure!(
                                err_type.is_none(),
                                "none err result found instead of {}",
                                err_type.as_ref().unwrap().desc()
                            );
                            wasmtime_component_val_result_t {
                                value: None,
                                error: true,
                            }
                        }
                    })
                } else {
                    bail!("attempted to create a Result for a {}", ty.desc());
                }
            }
            Val::Flags(vec) => {
                if let CType::Flags(ty) = ty {
                    let mapping: HashMap<_, _> = ty.iter().zip(0u32..).collect();
                    let mut flags: wasmtime_component_val_flags_t = Vec::new().into();
                    for name in vec.iter() {
                        let idx = mapping.get(name).context("expected valid name")?;
                        wasmtime_component_val_flags_set(&mut flags, *idx, true);
                    }
                    wasmtime_component_val_t::Flags(flags)
                } else {
                    bail!("attempted to create a Flags for a {}", ty.desc());
                }
            }
            Val::Resource(_) => bail!("resources not supported"),
        })
    }
}

// a wasmtime_component_val_t and its associated wasmtime_component_type_t
struct CTypedCVal<'a>(&'a wasmtime_component_val_t, &'a CType);

impl TryFrom<CTypedCVal<'_>> for Val {
    type Error = anyhow::Error;
    fn try_from(value: CTypedCVal) -> Result<Self> {
        let (value, ty) = (value.0, value.1);
        Ok(match value {
            &wasmtime_component_val_t::Bool(b) => {
                ensure_type!(ty, CType::Bool);
                Val::Bool(b)
            }
            &wasmtime_component_val_t::S8(v) => {
                ensure_type!(ty, CType::S8);
                Val::S8(v)
            }
            &wasmtime_component_val_t::U8(v) => {
                ensure_type!(ty, CType::U8);
                Val::U8(v)
            }
            &wasmtime_component_val_t::S16(v) => {
                ensure_type!(ty, CType::S16);
                Val::S16(v)
            }
            &wasmtime_component_val_t::U16(v) => {
                ensure_type!(ty, CType::U16);
                Val::U16(v)
            }
            &wasmtime_component_val_t::S32(v) => {
                ensure_type!(ty, CType::S32);
                Val::S32(v)
            }
            &wasmtime_component_val_t::U32(v) => {
                ensure_type!(ty, CType::U32);
                Val::U32(v)
            }
            &wasmtime_component_val_t::S64(v) => {
                ensure_type!(ty, CType::S64);
                Val::S64(v)
            }
            &wasmtime_component_val_t::U64(v) => {
                ensure_type!(ty, CType::U64);
                Val::U64(v)
            }
            &wasmtime_component_val_t::F32(v) => {
                ensure_type!(ty, CType::F32);
                Val::Float32(v)
            }
            &wasmtime_component_val_t::F64(v) => {
                ensure_type!(ty, CType::F64);
                Val::Float64(v)
            }
            &wasmtime_component_val_t::Char(v) => {
                ensure_type!(ty, CType::Char);
                Val::Char(v)
            }
            wasmtime_component_val_t::String(v) => {
                ensure_type!(ty, CType::String);
                Val::String(String::from_utf8(v.as_slice().to_vec())?)
            }
            wasmtime_component_val_t::List(v) => {
                if let CType::List(ty) = ty {
                    Val::List(
                        v.as_slice()
                            .iter()
                            .map(|v| CTypedCVal(v, ty.deref()).try_into())
                            .collect::<Result<Vec<_>>>()?,
                    )
                } else {
                    bail!("attempted to create a list for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Record(v) => {
                if let CType::Record(ty) = ty {
                    let mut field_vals: HashMap<&[u8], &wasmtime_component_val_t> =
                        HashMap::from_iter(
                            v.as_slice().iter().map(|f| (f.name.as_slice(), &f.val)),
                        );
                    Val::Record(
                        ty.iter()
                            .map(|tyf| {
                                if let Some(v) = field_vals.remove(tyf.0.as_bytes()) {
                                    Ok((tyf.0.clone(), CTypedCVal(v, &tyf.1).try_into()?))
                                } else {
                                    bail!("record missing field: {}", tyf.0);
                                }
                            })
                            .collect::<Result<Vec<_>>>()?,
                    )
                } else {
                    bail!("attempted to create a record for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Tuple(v) => {
                if let CType::Tuple(ty) = ty {
                    Val::Tuple(
                        ty.iter()
                            .zip(v.as_slice().iter())
                            .map(|(ty, v)| CTypedCVal(v, ty).try_into())
                            .collect::<Result<Vec<_>>>()?,
                    )
                } else {
                    bail!("attempted to create a tuple for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Variant(v) => {
                if let CType::Variant(ty) = ty {
                    let index = v.discriminant as usize;
                    ensure!(index < ty.len(), "variant index outside range");
                    let case = &ty[index];
                    let case_name = case.0.clone();
                    ensure!(
                        case.1.is_some() == v.val.is_some(),
                        "variant type mismatch for case {}: {} instead of {}",
                        case_name,
                        if v.val.is_some() { "some" } else { "none" },
                        case.1.as_ref().map(|ty| ty.desc()).unwrap_or("none")
                    );
                    if let (Some(t), Some(v)) = (&case.1, &v.val) {
                        let v = CTypedCVal(v.as_ref(), t.deref()).try_into()?;
                        Val::Variant(case_name, Some(Box::new(v)))
                    } else {
                        Val::Variant(case_name, None)
                    }
                } else {
                    bail!("attempted to create a variant for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Enum(v) => {
                if let CType::Enum(ty) = ty {
                    let index = v.discriminant as usize;
                    ensure!(index < ty.as_slice().len(), "variant index outside range");
                    Val::Enum(ty[index].clone())
                } else {
                    bail!("attempted to create an enum for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Option(v) => {
                if let CType::Option(ty) = ty {
                    Val::Option(match v {
                        Some(v) => Some(Box::new(CTypedCVal(v.as_ref(), ty.deref()).try_into()?)),
                        None => None,
                    })
                } else {
                    bail!("attempted to create an option for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Result(v) => {
                if let CType::Result(ok_ty, err_ty) = ty {
                    if v.error {
                        match &v.value {
                            Some(v) => {
                                let ty = err_ty.as_deref().context("expected err type")?;
                                Val::Result(Err(Some(Box::new(
                                    CTypedCVal(v.as_ref(), ty).try_into()?,
                                ))))
                            }
                            None => {
                                ensure!(err_ty.is_none(), "expected no err type");
                                Val::Result(Err(None))
                            }
                        }
                    } else {
                        match &v.value {
                            Some(v) => {
                                let ty = ok_ty.as_deref().context("expected ok type")?;
                                Val::Result(Ok(Some(Box::new(
                                    CTypedCVal(v.as_ref(), ty).try_into()?,
                                ))))
                            }
                            None => {
                                ensure!(ok_ty.is_none(), "expected no ok type");
                                Val::Result(Ok(None))
                            }
                        }
                    }
                } else {
                    bail!("attempted to create a result for a {}", ty.desc());
                }
            }
            wasmtime_component_val_t::Flags(flags) => {
                if let CType::Flags(ty) = ty {
                    let mut set = Vec::new();
                    for (idx, name) in ty.iter().enumerate() {
                        if wasmtime_component_val_flags_test(&flags, idx as u32) {
                            set.push(name.clone());
                        }
                    }
                    Val::Flags(set)
                } else {
                    bail!("attempted to create a flags for a {}", ty.desc());
                }
            }
        })
    }
}

impl TryFrom<(&Val, &Type)> for wasmtime_component_val_t {
    type Error = anyhow::Error;

    fn try_from((value, ty): (&Val, &Type)) -> Result<Self> {
        Ok(match value {
            Val::Bool(v) => wasmtime_component_val_t::Bool(*v),
            Val::S8(v) => wasmtime_component_val_t::S8(*v),
            Val::U8(v) => wasmtime_component_val_t::U8(*v),
            Val::S16(v) => wasmtime_component_val_t::S16(*v),
            Val::U16(v) => wasmtime_component_val_t::U16(*v),
            Val::S32(v) => wasmtime_component_val_t::S32(*v),
            Val::U32(v) => wasmtime_component_val_t::U32(*v),
            Val::S64(v) => wasmtime_component_val_t::S64(*v),
            Val::U64(v) => wasmtime_component_val_t::U64(*v),
            Val::Float32(v) => wasmtime_component_val_t::F32(*v),
            Val::Float64(v) => wasmtime_component_val_t::F64(*v),
            Val::Char(v) => wasmtime_component_val_t::Char(*v),
            Val::String(v) => wasmtime_component_val_t::String(v.clone().into_bytes().into()),
            Val::List(v) => {
                if let Type::List(ty) = ty {
                    let v = v
                        .iter()
                        .map(|v| (v, &ty.ty()).try_into())
                        .collect::<Result<Vec<_>>>()?;
                    wasmtime_component_val_t::List(v.into())
                } else {
                    bail!("attempted to create a {} from a list", ty.desc());
                }
            }
            Val::Record(v) => {
                if let Type::Record(ty) = ty {
                    let fields_types: HashMap<String, Type> =
                        HashMap::from_iter(ty.fields().map(|f| (f.name.to_string(), f.ty)));
                    let v = v
                        .iter()
                        .map(|(name, v)| {
                            if let Some(ty) = fields_types.get(name.as_str()) {
                                Ok(wasmtime_component_val_record_field_t {
                                    name: name.clone().into_bytes().into(),
                                    val: (v, ty).try_into()?,
                                })
                            } else {
                                bail!("field {} not found in record type", name);
                            }
                        })
                        .collect::<Result<Vec<_>>>()?;
                    wasmtime_component_val_t::Record(v.into())
                } else {
                    bail!("attempted to create a {} from a record", ty.desc());
                }
            }
            Val::Tuple(v) => {
                if let Type::Tuple(ty) = ty {
                    let elem_types = ty.types().collect::<Vec<_>>();
                    if v.len() != elem_types.len() {
                        bail!(
                            "attempted to create a size {} tuple from a size {} tuple",
                            elem_types.len(),
                            v.len()
                        );
                    }
                    let v = v
                        .iter()
                        .zip(elem_types.iter())
                        .map(|v| v.try_into())
                        .collect::<Result<Vec<_>>>()?;
                    wasmtime_component_val_t::Tuple(v.into())
                } else {
                    bail!("attempted to create a {} from a tuple", ty.desc());
                }
            }
            Val::Variant(discriminant, v) => {
                if let Type::Variant(ty) = ty {
                    let (index, case) = ty
                        .cases()
                        .enumerate()
                        .find(|(_, v)| v.name == discriminant)
                        .map(|(idx, case)| (idx as u32, case))
                        .context("expected valid discriminant")?;
                    let val = match v {
                        Some(v) => {
                            if let Some(ty) = &case.ty {
                                Some(Box::new((v.as_ref(), ty).try_into()?))
                            } else {
                                bail!("attempted to create a None Variant from a Some variant");
                            }
                        }
                        None => None,
                    };
                    wasmtime_component_val_t::Variant(wasmtime_component_val_variant_t {
                        discriminant: index,
                        val,
                    })
                } else {
                    bail!("attempted to create a {} from a variant", ty.desc());
                }
            }
            Val::Enum(discriminant) => {
                if let Type::Enum(ty) = ty {
                    let index = ty
                        .names()
                        .zip(0u32..)
                        .find(|(n, _)| *n == discriminant)
                        .map(|(_, idx)| idx)
                        .context("expected valid discriminant")?;
                    wasmtime_component_val_t::Enum(wasmtime_component_val_enum_t {
                        discriminant: index,
                    })
                } else {
                    bail!("attempted to create a {} from an enum", ty.desc());
                }
            }
            Val::Option(v) => {
                if let Type::Option(ty) = ty {
                    wasmtime_component_val_t::Option(match v {
                        Some(v) => Some(Box::new((v.as_ref(), &ty.ty()).try_into()?)),
                        None => None,
                    })
                } else {
                    bail!("attempted to create a {} from an option", ty.desc());
                }
            }
            Val::Result(v) => {
                if let Type::Result(ty) = ty {
                    let (error, value) = match v {
                        Err(v) => {
                            let value = match v {
                                Some(v) => {
                                    if let Some(ty) = ty.err() {
                                        Some(Box::new((v.as_ref(), &ty).try_into()?))
                                    } else {
                                        bail!(
                                            "attempted to create a None result from a Some result"
                                        );
                                    }
                                }
                                None => None,
                            };
                            (true, value)
                        }
                        Ok(v) => {
                            let value = match v {
                                Some(v) => {
                                    if let Some(ty) = ty.ok() {
                                        Some(Box::new((v.as_ref(), &ty).try_into()?))
                                    } else {
                                        bail!(
                                            "attempted to create a None result from a Some result"
                                        );
                                    }
                                }
                                None => None,
                            };
                            (false, value)
                        }
                    };
                    wasmtime_component_val_t::Result(wasmtime_component_val_result_t {
                        value,
                        error,
                    })
                } else {
                    bail!("attempted to create a {} from a result", ty.desc());
                }
            }
            Val::Flags(v) => {
                if let Type::Flags(ty) = ty {
                    let mapping: HashMap<_, _> = ty.names().zip(0u32..).collect();
                    let mut flags: wasmtime_component_val_flags_t = Vec::new().into();
                    for name in v {
                        let idx = mapping.get(name.as_str()).context("expected valid name")?;
                        wasmtime_component_val_flags_set(&mut flags, *idx, true);
                    }
                    wasmtime_component_val_t::Flags(flags)
                } else {
                    bail!("attempted to create a {} from a flags", ty.desc());
                }
            }
            Val::Resource(_) => bail!("resource types are unimplemented"),
        })
    }
}

impl Default for wasmtime_component_val_t {
    fn default() -> Self {
        Self::Bool(false)
    }
}

#[no_mangle]
pub extern "C" fn wasmtime_component_val_new() -> Box<wasmtime_component_val_t> {
    Box::new(wasmtime_component_val_t::default())
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_component_val_delete(_: Box<wasmtime_component_val_t>) {}

pub type wasmtime_component_kind_t = u8;
pub const WASMTIME_COMPONENT_KIND_BOOL: wasmtime_component_kind_t = 0;
pub const WASMTIME_COMPONENT_KIND_S8: wasmtime_component_kind_t = 1;
pub const WASMTIME_COMPONENT_KIND_U8: wasmtime_component_kind_t = 2;
pub const WASMTIME_COMPONENT_KIND_S16: wasmtime_component_kind_t = 3;
pub const WASMTIME_COMPONENT_KIND_U16: wasmtime_component_kind_t = 4;
pub const WASMTIME_COMPONENT_KIND_S32: wasmtime_component_kind_t = 5;
pub const WASMTIME_COMPONENT_KIND_U32: wasmtime_component_kind_t = 6;
pub const WASMTIME_COMPONENT_KIND_S64: wasmtime_component_kind_t = 7;
pub const WASMTIME_COMPONENT_KIND_U64: wasmtime_component_kind_t = 8;
pub const WASMTIME_COMPONENT_KIND_F32: wasmtime_component_kind_t = 9;
pub const WASMTIME_COMPONENT_KIND_F64: wasmtime_component_kind_t = 10;
pub const WASMTIME_COMPONENT_KIND_CHAR: wasmtime_component_kind_t = 11;
pub const WASMTIME_COMPONENT_KIND_STRING: wasmtime_component_kind_t = 12;
pub const WASMTIME_COMPONENT_KIND_LIST: wasmtime_component_kind_t = 13;
pub const WASMTIME_COMPONENT_KIND_RECORD: wasmtime_component_kind_t = 14;
pub const WASMTIME_COMPONENT_KIND_TUPLE: wasmtime_component_kind_t = 15;
pub const WASMTIME_COMPONENT_KIND_VARIANT: wasmtime_component_kind_t = 16;
pub const WASMTIME_COMPONENT_KIND_ENUM: wasmtime_component_kind_t = 17;
pub const WASMTIME_COMPONENT_KIND_OPTION: wasmtime_component_kind_t = 18;
pub const WASMTIME_COMPONENT_KIND_RESULT: wasmtime_component_kind_t = 19;
pub const WASMTIME_COMPONENT_KIND_FLAGS: wasmtime_component_kind_t = 20;

#[repr(C)]
#[derive(Clone)]
pub struct wasmtime_component_type_field_t {
    pub name: wasm_name_t,
    pub ty: Option<wasmtime_component_type_t>,
}

impl Default for wasmtime_component_type_field_t {
    fn default() -> Self {
        Self {
            name: Vec::new().into(),
            ty: Default::default(),
        }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct wasmtime_component_type_result_t {
    pub ok_ty: Option<Box<wasmtime_component_type_t>>,
    pub err_ty: Option<Box<wasmtime_component_type_t>>,
}

declare_vecs! {
    (
        name: wasmtime_component_type_vec_t,
        ty: wasmtime_component_type_t,
        new: wasmtime_component_type_vec_new,
        empty: wasmtime_component_type_vec_new_empty,
        uninit: wasmtime_component_type_vec_new_uninitialized,
        copy: wasmtime_component_type_vec_copy,
        delete: wasmtime_component_type_vec_delete,
    )
    (
        name: wasmtime_component_type_field_vec_t,
        ty: wasmtime_component_type_field_t,
        new: wasmtime_component_type_field_vec_new,
        empty: wasmtime_component_type_field_vec_new_empty,
        uninit: wasmtime_component_type_field_vec_new_uninitialized,
        copy: wasmtime_component_type_field_vec_copy,
        delete: wasmtime_component_type_field_vec_delete,
    )
    (
        name: wasmtime_component_string_vec_t,
        ty: wasmtime_component_string_t,
        new: wasmtime_component_string_vec_new,
        empty: wasmtime_component_string_vec_new_empty,
        uninit: wasmtime_component_string_vec_new_uninitialized,
        copy: wasmtime_component_string_vec_copy,
        delete: wasmtime_component_string_vec_delete,
    )
}

#[repr(C, u8)]
#[derive(Clone)]
pub enum wasmtime_component_type_t {
    Bool,
    S8,
    U8,
    S16,
    U16,
    S32,
    U32,
    S64,
    U64,
    F32,
    F64,
    Char,
    String,
    List(Box<wasmtime_component_type_t>),
    Record(wasmtime_component_type_field_vec_t),
    Tuple(wasmtime_component_type_vec_t),
    Variant(wasmtime_component_type_field_vec_t),
    Enum(wasmtime_component_string_vec_t),
    Option(Box<wasmtime_component_type_t>),
    Result(wasmtime_component_type_result_t),
    Flags(wasmtime_component_string_vec_t),
}

impl Default for wasmtime_component_type_t {
    fn default() -> Self {
        Self::Bool
    }
}

#[no_mangle]
pub extern "C" fn wasmtime_component_type_new() -> Box<wasmtime_component_type_t> {
    Box::new(wasmtime_component_type_t::Bool)
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_component_type_delete(_: Box<wasmtime_component_type_t>) {}

#[derive(Clone)]
pub enum CType {
    Bool,
    S8,
    U8,
    S16,
    U16,
    S32,
    U32,
    S64,
    U64,
    F32,
    F64,
    Char,
    String,
    List(Box<CType>),
    Record(Vec<(String, CType)>),
    Tuple(Vec<CType>),
    Variant(Vec<(String, Option<Box<CType>>)>),
    Enum(Vec<String>),
    Option(Box<CType>),
    Result(Option<Box<CType>>, Option<Box<CType>>),
    Flags(Vec<String>),
}

impl TryFrom<&wasmtime_component_type_t> for CType {
    type Error = anyhow::Error;

    fn try_from(ty: &wasmtime_component_type_t) -> Result<Self> {
        Ok(match ty {
            wasmtime_component_type_t::Bool => CType::Bool,
            wasmtime_component_type_t::S8 => CType::S8,
            wasmtime_component_type_t::U8 => CType::U8,
            wasmtime_component_type_t::S16 => CType::S16,
            wasmtime_component_type_t::U16 => CType::U16,
            wasmtime_component_type_t::S32 => CType::S32,
            wasmtime_component_type_t::U32 => CType::U32,
            wasmtime_component_type_t::S64 => CType::S64,
            wasmtime_component_type_t::U64 => CType::U64,
            wasmtime_component_type_t::F32 => CType::F32,
            wasmtime_component_type_t::F64 => CType::F64,
            wasmtime_component_type_t::Char => CType::Char,
            wasmtime_component_type_t::String => CType::String,
            wasmtime_component_type_t::List(ty) => CType::List(Box::new(ty.as_ref().try_into()?)),
            wasmtime_component_type_t::Record(fields) => CType::Record(
                fields
                    .as_slice()
                    .iter()
                    .map(|field| {
                        let field_name = String::from_utf8(field.name.as_slice().to_vec())?;
                        let field_type = match &field.ty {
                            Some(ty) => ty.try_into()?,
                            None => bail!("missing type of field {} in record", field_name),
                        };
                        Ok((field_name, field_type))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            wasmtime_component_type_t::Tuple(types) => CType::Tuple(
                types
                    .as_slice()
                    .iter()
                    .map(|ty| ty.try_into())
                    .collect::<Result<Vec<_>>>()?,
            ),
            wasmtime_component_type_t::Variant(cases) => CType::Variant(
                cases
                    .as_slice()
                    .iter()
                    .map(|case| {
                        let case_name = String::from_utf8(case.name.as_slice().to_vec())?;
                        let case_type = match &case.ty {
                            Some(ty) => Some(Box::new(ty.try_into()?)),
                            None => None,
                        };
                        Ok((case_name, case_type))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            wasmtime_component_type_t::Enum(enums) => CType::Enum(
                enums
                    .as_slice()
                    .iter()
                    .map(|s| Ok(String::from_utf8(s.as_slice().to_vec())?))
                    .collect::<Result<Vec<_>>>()?,
            ),
            wasmtime_component_type_t::Option(ty) => {
                CType::Option(Box::new(ty.as_ref().try_into()?))
            }
            wasmtime_component_type_t::Result(wasmtime_component_type_result_t {
                ok_ty,
                err_ty,
            }) => CType::Result(
                match ok_ty {
                    Some(ty) => Some(Box::new(ty.as_ref().try_into()?)),
                    None => None,
                },
                match err_ty {
                    Some(ty) => Some(Box::new(ty.as_ref().try_into()?)),
                    None => None,
                },
            ),
            wasmtime_component_type_t::Flags(flags) => CType::Flags(
                flags
                    .as_slice()
                    .iter()
                    .map(|s| Ok(String::from_utf8(s.as_slice().to_vec())?))
                    .collect::<Result<Vec<_>>>()?,
            ),
        })
    }
}

impl CType {
    /// Return a string description of this type
    fn desc(&self) -> &'static str {
        match self {
            CType::Bool => "bool",
            CType::S8 => "s8",
            CType::U8 => "u8",
            CType::S16 => "s16",
            CType::U16 => "u16",
            CType::S32 => "s32",
            CType::U32 => "u32",
            CType::S64 => "s64",
            CType::U64 => "u64",
            CType::F32 => "f32",
            CType::F64 => "f64",
            CType::Char => "char",
            CType::String => "string",
            CType::List(_) => "list",
            CType::Record(_) => "record",
            CType::Tuple(_) => "tuple",
            CType::Variant(_) => "variant",
            CType::Enum(_) => "enum",
            CType::Option(_) => "option",
            CType::Result(_, _) => "result",
            CType::Flags(_) => "flags",
        }
    }

    fn default_cval(&self) -> wasmtime_component_val_t {
        match self {
            CType::Bool => wasmtime_component_val_t::Bool(false),
            CType::S8 => wasmtime_component_val_t::S8(0),
            CType::U8 => wasmtime_component_val_t::U8(0),
            CType::S16 => wasmtime_component_val_t::S16(0),
            CType::U16 => wasmtime_component_val_t::U16(0),
            CType::S32 => wasmtime_component_val_t::S32(0),
            CType::U32 => wasmtime_component_val_t::U32(0),
            CType::S64 => wasmtime_component_val_t::S64(0),
            CType::U64 => wasmtime_component_val_t::U64(0),
            CType::F32 => wasmtime_component_val_t::F32(0.0),
            CType::F64 => wasmtime_component_val_t::F64(0.0),
            CType::Char => wasmtime_component_val_t::Char('\0'),
            CType::String => {
                wasmtime_component_val_t::String(wasmtime_component_string_t::default())
            }
            CType::List(_) => {
                wasmtime_component_val_t::List(wasmtime_component_val_vec_t::default())
            }
            CType::Record(fields) => wasmtime_component_val_t::Record(
                fields
                    .iter()
                    .map(|(name, ty)| wasmtime_component_val_record_field_t {
                        name: name.clone().into_bytes().into(),
                        val: ty.default_cval(),
                    })
                    .collect::<Vec<_>>()
                    .into(),
            ),
            CType::Tuple(tuple) => wasmtime_component_val_t::Tuple(
                tuple
                    .iter()
                    .map(|ty| ty.default_cval())
                    .collect::<Vec<_>>()
                    .into(),
            ),
            CType::Variant(cases) => {
                wasmtime_component_val_t::Variant(wasmtime_component_val_variant_t {
                    discriminant: 0,
                    val: match &cases[0].1 {
                        Some(ty) => Some(Box::new(ty.default_cval())),
                        None => None,
                    },
                })
            }
            CType::Enum(_) => {
                wasmtime_component_val_t::Enum(wasmtime_component_val_enum_t { discriminant: 0 })
            }
            CType::Option(_) => wasmtime_component_val_t::Option(None),
            CType::Result(_, _) => {
                wasmtime_component_val_t::Result(wasmtime_component_val_result_t {
                    value: None,
                    error: false,
                })
            }
            CType::Flags(_) => {
                wasmtime_component_val_t::Flags(wasmtime_component_val_flags_t::default())
            }
        }
    }
}

#[repr(transparent)]
pub struct wasmtime_component_t {
    component: Component,
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_component_from_binary(
    engine: &wasm_engine_t,
    bytes: *const u8,
    len: usize,
    out: &mut *mut wasmtime_component_t,
) -> Option<Box<wasmtime_error_t>> {
    let bytes = crate::slice_from_raw_parts(bytes, len);
    handle_result(Component::from_binary(&engine.engine, bytes), |component| {
        *out = Box::into_raw(Box::new(wasmtime_component_t { component }));
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_component_delete(_: Box<wasmtime_component_t>) {}

pub type wasmtime_component_func_callback_t = extern "C" fn(
    *mut c_void,
    WasmtimeStoreContextMut<'_>,
    *const wasmtime_component_val_t,
    usize,
    *mut wasmtime_component_val_t,
    usize,
) -> Option<Box<wasm_trap_t>>;

struct HostFuncDefinition {
    path: Vec<String>,
    name: String,
    params_types: Vec<CType>,
    outputs_types: Vec<CType>,
    callback: wasmtime_component_func_callback_t,
    data: *mut c_void,
    finalizer: Option<extern "C" fn(*mut std::ffi::c_void)>,
}

#[repr(C)]
pub struct wasmtime_component_linker_t {
    linker: Linker<WasmtimeStoreData>,
    is_built: bool,
    functions: Vec<HostFuncDefinition>,
}

#[no_mangle]
pub extern "C" fn wasmtime_component_linker_new(
    engine: &wasm_engine_t,
) -> Box<wasmtime_component_linker_t> {
    Box::new(wasmtime_component_linker_t {
        linker: Linker::new(&engine.engine),
        is_built: false,
        functions: Vec::new(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_component_linker_delete(_: Box<wasmtime_component_linker_t>) {}

fn to_ctype_vec(buf: *mut wasmtime_component_type_t, len: usize) -> Result<Vec<CType>> {
    if len == 0 {
        return Ok(Vec::new());
    }
    let v = unsafe { crate::slice_from_raw_parts(buf, len) };
    v.iter().map(|t| t.try_into()).collect::<Result<Vec<_>>>()
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_component_linker_define_func(
    linker: &mut wasmtime_component_linker_t,
    path_buf: *const u8,
    path_len: usize,
    name_buf: *const u8,
    name_len: usize,
    params_types_buf: *mut wasmtime_component_type_t,
    params_types_len: usize,
    outputs_types_buf: *mut wasmtime_component_type_t,
    outputs_types_len: usize,
    callback: wasmtime_component_func_callback_t,
    data: *mut c_void,
    finalizer: Option<extern "C" fn(*mut std::ffi::c_void)>,
) -> Option<Box<wasmtime_error_t>> {
    let path = to_str!(path_buf, path_len)
        .split('.')
        .filter(|s| s.len() > 0)
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let name = to_str!(name_buf, name_len).to_string();
    let params_types = match to_ctype_vec(params_types_buf, params_types_len) {
        Err(err) => return Some(Box::new(wasmtime_error_t::from(err))),
        Ok(p) => p,
    };
    let outputs_types = match to_ctype_vec(outputs_types_buf, outputs_types_len) {
        Err(err) => return Some(Box::new(wasmtime_error_t::from(err))),
        Ok(p) => p,
    };

    linker.functions.push(HostFuncDefinition {
        path,
        name,
        params_types,
        outputs_types,
        callback,
        data,
        finalizer,
    });
    None
}

fn build_closure(
    function: &HostFuncDefinition,
) -> impl Fn(WasmtimeStoreContextMut<'_>, &[Val], &mut [Val]) -> Result<()> {
    let func = function.callback;
    let params_types = function.params_types.clone();
    let outputs_types = function.outputs_types.clone();
    let foreign = crate::ForeignData {
        data: function.data,
        finalizer: function.finalizer,
    };
    move |context, parameters, outputs| {
        let _ = &foreign;
        let _ = &params_types;
        let _ = &outputs_types;
        let mut params = Vec::new();
        for param in parameters.iter().zip(params_types.iter()) {
            params.push(CTypedVal(param.0, param.1).try_into()?);
        }
        let mut outs = Vec::new();
        for output_type in outputs_types.iter() {
            outs.push(output_type.default_cval());
        }
        let res = func(
            foreign.data,
            context,
            params.as_ptr(),
            params.len(),
            outs.as_mut_ptr(),
            outs.len(),
        );
        match res {
            None => {
                for (i, (output, output_type)) in outs.iter().zip(outputs_types.iter()).enumerate()
                {
                    outputs[i] = CTypedCVal(output, output_type).try_into()?;
                }
                Ok(())
            }
            Some(trap) => Err(trap.error),
        }
    }
}

#[no_mangle]
pub extern "C" fn wasmtime_component_linker_build(
    linker: &mut wasmtime_component_linker_t,
) -> Option<Box<wasmtime_error_t>> {
    if linker.is_built {
        return Some(Box::new(wasmtime_error_t::from(anyhow!(
            "cannot build an already built linker"
        ))));
    }

    struct InstanceTree {
        children: HashMap<String, InstanceTree>,
        functions: Vec<HostFuncDefinition>,
    }

    impl InstanceTree {
        fn insert(&mut self, depth: usize, function: HostFuncDefinition) {
            if function.path.len() == depth {
                self.functions.push(function);
            } else {
                let child = self
                    .children
                    .entry(function.path[depth].to_string())
                    .or_insert_with(|| InstanceTree {
                        children: HashMap::new(),
                        functions: Vec::new(),
                    });
                child.insert(depth + 1, function);
            }
        }
        fn build(&self, mut instance: LinkerInstance<WasmtimeStoreData>) -> Result<()> {
            for function in self.functions.iter() {
                instance.func_new(&function.name, build_closure(function))?;
            }
            for (name, child) in self.children.iter() {
                let child_instance = instance.instance(&name)?;
                child.build(child_instance)?;
            }
            Ok(())
        }
    }

    let mut root = InstanceTree {
        children: HashMap::new(),
        functions: Vec::new(),
    };
    for function in linker.functions.drain(..) {
        root.insert(0, function);
    }
    match root.build(linker.linker.root()) {
        Ok(()) => {
            linker.is_built = true;
            None
        }
        Err(err) => Some(Box::new(wasmtime_error_t::from(anyhow!(err)))),
    }
}

#[no_mangle]
pub extern "C" fn wasmtime_component_linker_instantiate(
    linker: &wasmtime_component_linker_t,
    store: WasmtimeStoreContextMut<'_>,
    component: &wasmtime_component_t,
    out: &mut *mut wasmtime_component_instance_t,
) -> Option<Box<wasmtime_error_t>> {
    if !linker.is_built && !linker.functions.is_empty() {
        return Some(Box::new(wasmtime_error_t::from(anyhow!(
            "cannot instantiate with a linker not built"
        ))));
    }
    match linker.linker.instantiate(store, &component.component) {
        Ok(instance) => {
            *out = Box::into_raw(Box::new(wasmtime_component_instance_t { instance }));
            None
        }
        Err(e) => Some(Box::new(wasmtime_error_t::from(e))),
    }
}

#[repr(transparent)]
pub struct wasmtime_component_instance_t {
    instance: Instance,
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_component_instance_get_func(
    instance: &wasmtime_component_instance_t,
    context: WasmtimeStoreContextMut<'_>,
    name: *const u8,
    len: usize,
    item: &mut *mut wasmtime_component_func_t,
) -> bool {
    let name = crate::slice_from_raw_parts(name, len);
    let name = match std::str::from_utf8(name) {
        Ok(name) => name,
        Err(_) => return false,
    };
    let func = instance.instance.get_func(context, name);
    if let Some(func) = func {
        *item = Box::into_raw(Box::new(wasmtime_component_func_t { func }));
    }
    func.is_some()
}

#[repr(transparent)]
pub struct wasmtime_component_func_t {
    func: Func,
}

fn call_func(
    func: &wasmtime_component_func_t,
    mut context: WasmtimeStoreContextMut<'_>,
    raw_params: &[wasmtime_component_val_t],
    raw_results: &mut [wasmtime_component_val_t],
) -> Result<()> {
    let params_types = func.func.params(context.as_context());
    if params_types.len() != raw_params.len() {
        bail!(
            "called with {} parameters instead of the expected {}",
            raw_params.len(),
            params_types.len()
        );
    }
    let results_types = func.func.results(context.as_context());
    if results_types.len() != raw_results.len() {
        bail!(
            "returns {} results instead of the expected {}",
            raw_results.len(),
            results_types.len()
        );
    }
    let params = func
        .func
        .params(context.as_context())
        .iter()
        .zip(raw_params.iter())
        .map(|(ty, v)| TypedCVal(v, &ty.1).try_into())
        .collect::<Result<Vec<_>>>()?;
    let mut results = vec![Val::Bool(false); raw_results.len()];
    func.func
        .call(context.as_context_mut(), &params, &mut results)?;
    func.func.post_return(context)?;
    for (i, (ty, r)) in results_types.iter().zip(results.iter()).enumerate() {
        raw_results[i] = (r, ty).try_into()?;
    }
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_component_func_call(
    func: &wasmtime_component_func_t,
    context: WasmtimeStoreContextMut<'_>,
    params: *const wasmtime_component_val_t,
    params_len: usize,
    results: *mut wasmtime_component_val_t,
    results_len: usize,
    out_trap: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    let raw_params = crate::slice_from_raw_parts(params, params_len);
    let mut raw_results = crate::slice_from_raw_parts_mut(results, results_len);
    match call_func(func, context, &raw_params, &mut raw_results) {
        Ok(_) => None,
        Err(e) => handle_call_error(e, out_trap),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        wasmtime_component_val_flags_set, wasmtime_component_val_flags_t,
        wasmtime_component_val_flags_test,
    };

    #[test]
    fn bit_fiddling() {
        let mut flags: wasmtime_component_val_flags_t = Vec::new().into();
        wasmtime_component_val_flags_set(&mut flags, 1, true);
        assert!(wasmtime_component_val_flags_test(&flags, 1));
        assert!(!wasmtime_component_val_flags_test(&flags, 0));
        wasmtime_component_val_flags_set(&mut flags, 260, true);
        assert!(wasmtime_component_val_flags_test(&flags, 260));
        assert!(!wasmtime_component_val_flags_test(&flags, 261));
        assert!(!wasmtime_component_val_flags_test(&flags, 259));
        assert!(wasmtime_component_val_flags_test(&flags, 1));
        assert!(!wasmtime_component_val_flags_test(&flags, 0));
    }
}
