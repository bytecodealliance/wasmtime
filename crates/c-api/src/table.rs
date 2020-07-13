use crate::r#ref::{ref_to_val, val_into_ref};
use crate::{handle_result, wasm_func_t, wasm_ref_t, wasmtime_error_t};
use crate::{wasm_extern_t, wasm_store_t, wasm_tabletype_t};
use std::ptr;
use wasmtime::{Extern, Table, TableType, Val, ValType};

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_table_t {
    ext: wasm_extern_t,
}

wasmtime_c_api_macros::declare_ref!(wasm_table_t);

pub type wasm_table_size_t = u32;

impl wasm_table_t {
    pub(crate) fn try_from(e: &wasm_extern_t) -> Option<&wasm_table_t> {
        match &e.which {
            Extern::Table(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    fn table(&self) -> &Table {
        match &self.ext.which {
            Extern::Table(t) => t,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

fn ref_to_val_for_table(r: Option<&wasm_ref_t>, table_ty: &TableType) -> Val {
    r.map_or_else(
        || match table_ty.element() {
            ValType::FuncRef => Val::FuncRef(None),
            ValType::ExternRef => Val::ExternRef(None),
            ty => panic!("unsupported table element type: {:?}", ty),
        },
        |r| ref_to_val(r),
    )
}

#[no_mangle]
pub extern "C" fn wasm_table_new(
    store: &wasm_store_t,
    tt: &wasm_tabletype_t,
    init: Option<&wasm_ref_t>,
) -> Option<Box<wasm_table_t>> {
    let init = ref_to_val_for_table(init, &tt.ty().ty);
    let table = Table::new(&store.store, tt.ty().ty.clone(), init).ok()?;
    Some(Box::new(wasm_table_t {
        ext: wasm_extern_t {
            which: table.into(),
        },
    }))
}

#[no_mangle]
pub extern "C" fn wasmtime_funcref_table_new(
    store: &wasm_store_t,
    tt: &wasm_tabletype_t,
    init: Option<&wasm_func_t>,
    out: &mut *mut wasm_table_t,
) -> Option<Box<wasmtime_error_t>> {
    let init: Val = match init {
        Some(val) => Val::FuncRef(Some(val.func().clone())),
        None => Val::FuncRef(None),
    };
    handle_result(
        Table::new(&store.store, tt.ty().ty.clone(), init),
        |table| {
            *out = Box::into_raw(Box::new(wasm_table_t {
                ext: wasm_extern_t {
                    which: table.into(),
                },
            }));
        },
    )
}

#[no_mangle]
pub extern "C" fn wasm_table_type(t: &wasm_table_t) -> Box<wasm_tabletype_t> {
    let ty = t.table().ty();
    Box::new(wasm_tabletype_t::new(ty))
}

#[no_mangle]
pub extern "C" fn wasm_table_get(
    t: &wasm_table_t,
    index: wasm_table_size_t,
) -> Option<Box<wasm_ref_t>> {
    let val = t.table().get(index)?;
    Some(val_into_ref(val).unwrap())
}

#[no_mangle]
pub extern "C" fn wasmtime_funcref_table_get(
    t: &wasm_table_t,
    index: wasm_table_size_t,
    ptr: &mut *mut wasm_func_t,
) -> bool {
    match t.table().get(index) {
        Some(val) => {
            *ptr = match val {
                Val::FuncRef(None) => ptr::null_mut(),
                Val::FuncRef(Some(f)) => Box::into_raw(Box::new(f.into())),
                _ => return false,
            };
        }

        _ => return false,
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_set(
    t: &wasm_table_t,
    index: wasm_table_size_t,
    r: Option<&wasm_ref_t>,
) -> bool {
    let val = ref_to_val_for_table(r, &t.table().ty());
    t.table().set(index, val).is_ok()
}

#[no_mangle]
pub extern "C" fn wasmtime_funcref_table_set(
    t: &wasm_table_t,
    index: wasm_table_size_t,
    val: Option<&wasm_func_t>,
) -> Option<Box<wasmtime_error_t>> {
    let val = match val {
        Some(val) => Val::FuncRef(Some(val.func().clone())),
        None => Val::FuncRef(None),
    };
    handle_result(t.table().set(index, val), |()| {})
}

#[no_mangle]
pub extern "C" fn wasm_table_size(t: &wasm_table_t) -> wasm_table_size_t {
    t.table().size()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_grow(
    t: &wasm_table_t,
    delta: wasm_table_size_t,
    init: Option<&wasm_ref_t>,
) -> bool {
    let init = ref_to_val_for_table(init, &t.table().ty());
    t.table().grow(delta, init).is_ok()
}

#[no_mangle]
pub extern "C" fn wasmtime_funcref_table_grow(
    t: &wasm_table_t,
    delta: wasm_table_size_t,
    init: Option<&wasm_func_t>,
    prev_size: Option<&mut wasm_table_size_t>,
) -> Option<Box<wasmtime_error_t>> {
    let val = match init {
        Some(val) => Val::FuncRef(Some(val.func().clone())),
        None => Val::FuncRef(None),
    };
    handle_result(t.table().grow(delta, val), |prev| {
        if let Some(ptr) = prev_size {
            *ptr = prev;
        }
    })
}

#[no_mangle]
pub extern "C" fn wasm_table_as_extern(t: &wasm_table_t) -> &wasm_extern_t {
    &t.ext
}
