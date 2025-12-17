use crate::wasmtime_component_valtype_t;
use std::mem::MaybeUninit;
use wasmtime::component::types::ComponentFunc;

type_wrapper! {
    pub struct wasmtime_component_func_type_t {
        pub(crate) ty: ComponentFunc,
    }

    clone: wasmtime_component_func_type_clone,
    delete: wasmtime_component_func_type_delete,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_func_type_async(ty: &wasmtime_component_func_type_t) -> bool {
    ty.ty.async_()
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_func_type_param_count(
    ty: &wasmtime_component_func_type_t,
) -> usize {
    ty.ty.params().len()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_func_type_param_nth(
    ty: &wasmtime_component_func_type_t,
    nth: usize,
    name_ret: &mut MaybeUninit<*const u8>,
    name_len_ret: &mut MaybeUninit<usize>,
    type_ret: &mut MaybeUninit<wasmtime_component_valtype_t>,
) -> bool {
    match ty.ty.params().nth(nth) {
        Some((name, item)) => {
            let name: &str = name;
            name_ret.write(name.as_ptr());
            name_len_ret.write(name.len());
            type_ret.write(item.into());
            true
        }
        None => false,
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_func_type_result(
    ty: &wasmtime_component_func_type_t,
    type_ret: &mut MaybeUninit<wasmtime_component_valtype_t>,
) -> bool {
    let len = ty.ty.results().len();
    assert!(len <= 1);
    match ty.ty.results().next() {
        Some(item) => {
            type_ret.write(item.into());
            true
        }
        None => false,
    }
}
