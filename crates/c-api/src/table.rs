use crate::{handle_result, wasm_func_t, wasm_ref_t, wasmtime_error_t};
use crate::{wasm_extern_t, wasm_store_t, wasm_tabletype_t, ExternHost};
use std::ptr;
use wasmtime::{AnyRef, HostRef, Table, Val};

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
            ExternHost::Table(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    fn table(&self) -> &HostRef<Table> {
        match &self.ext.which {
            ExternHost::Table(t) => t,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    fn anyref(&self) -> wasmtime::AnyRef {
        self.table().anyref()
    }
}

#[no_mangle]
pub extern "C" fn wasm_table_new(
    store: &wasm_store_t,
    tt: &wasm_tabletype_t,
    init: Option<Box<wasm_ref_t>>,
) -> Option<Box<wasm_table_t>> {
    let init: Val = match init {
        Some(init) => init.r.into(),
        None => Val::AnyRef(AnyRef::Null),
    };
    let table = Table::new(&store.store.borrow(), tt.ty().ty.clone(), init).ok()?;
    Some(Box::new(wasm_table_t {
        ext: wasm_extern_t {
            which: ExternHost::Table(HostRef::new(table)),
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
        Some(val) => Val::FuncRef(val.func().borrow().clone()),
        None => Val::AnyRef(AnyRef::Null),
    };
    handle_result(
        Table::new(&store.store.borrow(), tt.ty().ty.clone(), init),
        |table| {
            *out = Box::into_raw(Box::new(wasm_table_t {
                ext: wasm_extern_t {
                    which: ExternHost::Table(HostRef::new(table)),
                },
            }));
            println!("ret at {:?}", *out);
        },
    )
}

#[no_mangle]
pub extern "C" fn wasm_table_type(t: &wasm_table_t) -> Box<wasm_tabletype_t> {
    let ty = t.table().borrow().ty();
    Box::new(wasm_tabletype_t::new(ty))
}

#[no_mangle]
pub extern "C" fn wasm_table_get(t: &wasm_table_t, index: wasm_table_size_t) -> *mut wasm_ref_t {
    match t.table().borrow().get(index) {
        Some(val) => into_funcref(val),
        None => into_funcref(Val::AnyRef(AnyRef::Null)),
    }
}

#[no_mangle]
pub extern "C" fn wasmtime_funcref_table_get(
    t: &wasm_table_t,
    index: wasm_table_size_t,
    ptr: &mut *mut wasm_func_t,
) -> bool {
    println!("get {:p} at {}", t, index);
    match t.table().borrow().get(index) {
        Some(val) => {
            *ptr = match val {
                // TODO: what do do about creating new `HostRef` handles here?
                Val::FuncRef(f) => Box::into_raw(Box::new(HostRef::new(f).into())),
                Val::AnyRef(AnyRef::Null) => ptr::null_mut(),
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
    r: *mut wasm_ref_t,
) -> bool {
    let val = from_funcref(r);
    t.table().borrow().set(index, val).is_ok()
}

#[no_mangle]
pub extern "C" fn wasmtime_funcref_table_set(
    t: &wasm_table_t,
    index: wasm_table_size_t,
    val: Option<&wasm_func_t>,
) -> Option<Box<wasmtime_error_t>> {
    let val = match val {
        Some(val) => Val::FuncRef(val.func().borrow().clone()),
        None => Val::AnyRef(AnyRef::Null),
    };
    handle_result(t.table().borrow().set(index, val), |()| {})
}

fn into_funcref(val: Val) -> *mut wasm_ref_t {
    if let Val::AnyRef(AnyRef::Null) = val {
        return ptr::null_mut();
    }
    let anyref = match val.anyref() {
        Some(anyref) => anyref,
        None => return ptr::null_mut(),
    };
    let r = Box::new(wasm_ref_t { r: anyref });
    Box::into_raw(r)
}

unsafe fn from_funcref(r: *mut wasm_ref_t) -> Val {
    if !r.is_null() {
        Box::from_raw(r).r.into()
    } else {
        Val::AnyRef(AnyRef::Null)
    }
}

#[no_mangle]
pub extern "C" fn wasm_table_size(t: &wasm_table_t) -> wasm_table_size_t {
    t.table().borrow().size()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_grow(
    t: &wasm_table_t,
    delta: wasm_table_size_t,
    init: *mut wasm_ref_t,
) -> bool {
    let init = from_funcref(init);
    t.table().borrow().grow(delta, init).is_ok()
}

#[no_mangle]
pub extern "C" fn wasmtime_funcref_table_grow(
    t: &wasm_table_t,
    delta: wasm_table_size_t,
    init: Option<&wasm_func_t>,
    prev_size: Option<&mut wasm_table_size_t>,
) -> Option<Box<wasmtime_error_t>> {
    let val = match init {
        Some(val) => Val::FuncRef(val.func().borrow().clone()),
        None => Val::AnyRef(AnyRef::Null),
    };
    handle_result(t.table().borrow().grow(delta, val), |prev| {
        if let Some(ptr) = prev_size {
            *ptr = prev;
        }
    })
}

#[no_mangle]
pub extern "C" fn wasm_table_as_extern(t: &wasm_table_t) -> &wasm_extern_t {
    &t.ext
}
