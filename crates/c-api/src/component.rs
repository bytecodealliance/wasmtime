use anyhow::{bail, ensure, Context, Result};
use wasmtime::component::{Component, Func, Instance, Linker, Type, Val};
use wasmtime::{AsContext, AsContextMut};

use crate::{
    declare_vecs, handle_call_error, handle_result, wasm_byte_vec_t, wasm_config_t, wasm_engine_t,
    wasm_name_t, wasm_trap_t, wasmtime_error_t, WasmtimeStoreContextMut, WasmtimeStoreData,
};
use std::collections::HashMap;
use std::{mem, mem::MaybeUninit, ptr, slice};

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

#[repr(C)]
pub struct wasmtime_component_linker_t {
    linker: Linker<WasmtimeStoreData>,
}

#[no_mangle]
pub extern "C" fn wasmtime_component_linker_new(
    engine: &wasm_engine_t,
) -> Box<wasmtime_component_linker_t> {
    Box::new(wasmtime_component_linker_t {
        linker: Linker::new(&engine.engine),
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_component_linker_delete(_: Box<wasmtime_component_linker_t>) {}

#[no_mangle]
pub extern "C" fn wasmtime_component_linker_instantiate(
    linker: &wasmtime_component_linker_t,
    store: WasmtimeStoreContextMut<'_>,
    component: &wasmtime_component_t,
    out: &mut *mut wasmtime_component_instance_t,
) -> Option<Box<wasmtime_error_t>> {
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
