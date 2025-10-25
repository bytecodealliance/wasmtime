use super::wasmtime_component_val_t;
use crate::{WasmtimeStoreContextMut, wasm_name_t, wasmtime_component_valtype_t, wasmtime_error_t};
use std::mem::MaybeUninit;
use wasmtime::component::{Func, Val};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_func_call(
    func: &Func,
    mut context: WasmtimeStoreContextMut<'_>,
    args: *const wasmtime_component_val_t,
    args_len: usize,
    results: *mut wasmtime_component_val_t,
    results_len: usize,
) -> Option<Box<wasmtime_error_t>> {
    let c_args = unsafe { crate::slice_from_raw_parts(args, args_len) };
    let c_results = unsafe { crate::slice_from_raw_parts_mut(results, results_len) };

    let args = c_args.iter().map(Val::from).collect::<Vec<_>>();
    let mut results = vec![Val::Bool(false); results_len];

    let result = func.call(&mut context, &args, &mut results);

    crate::handle_result(result, |_| {
        for (c_val, rust_val) in std::iter::zip(c_results, results) {
            *c_val = wasmtime_component_val_t::from(&rust_val);
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_func_post_return(
    func: &Func,
    mut context: WasmtimeStoreContextMut<'_>,
) -> Option<Box<wasmtime_error_t>> {
    let result = func.post_return(&mut context);

    crate::handle_result(result, |_| {})
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_func_params_count(
    func: &Func,
    context: WasmtimeStoreContextMut<'_>,
) -> usize {
    func.params(context).len()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_func_params_get(
    func: &Func,
    context: WasmtimeStoreContextMut<'_>,
    out_names: *mut MaybeUninit<wasm_name_t>,
    out_types: *mut MaybeUninit<wasmtime_component_valtype_t>,
    out_size: usize,
) {
    let out_names = crate::slice_from_raw_parts_mut(out_names, out_size);
    let out_types = crate::slice_from_raw_parts_mut(out_types, out_size);
    let params = func.params(context);
    assert_eq!(out_names.len(), params.len());
    for ((slot_name, slot_ty), (name, ty)) in out_names.iter_mut().zip(out_types).zip(params) {
        slot_name.write(name.into_bytes().into());
        slot_ty.write(ty.into());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_func_result(
    func: &Func,
    context: WasmtimeStoreContextMut<'_>,
    out: &mut MaybeUninit<wasmtime_component_valtype_t>,
) -> bool {
    let results = func.results(context);
    assert!(results.len() <= 1);
    match results.into_iter().next() {
        Some(ty) => {
            out.write(ty.into());
            true
        }
        None => false,
    }
}
