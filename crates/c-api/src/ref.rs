use crate::{abort, WasmtimeStoreContextMut};
use std::{
    mem::{ManuallyDrop, MaybeUninit},
    os::raw::c_void,
    ptr,
};
use wasmtime::{AnyRef, ExternRef, ManuallyRooted, Ref, RootScope, Val, I31};

/// `*mut wasm_ref_t` is a reference type (`externref` or `funcref`), as seen by
/// the C API. Because we do not have a uniform representation for `funcref`s
/// and `externref`s, a `*mut wasm_ref_t` is morally a
/// `Option<Box<Either<ExternRef, Func>>>`.
///
/// A null `*mut wasm_ref_t` is either a null `funcref` or a null `externref`
/// depending on context (e.g. the table's element type that it is going into or
/// coming out of).
///
/// Note: this is not `#[repr(C)]` because it is an opaque type in the header,
/// and only ever referenced as `*mut wasm_ref_t`. This also lets us use a
/// regular, non-`repr(C)` `enum` to define `WasmRefInner`.
#[derive(Clone)]
pub struct wasm_ref_t {
    pub(crate) r: Ref,
}

wasmtime_c_api_macros::declare_own!(wasm_ref_t);

impl wasm_ref_t {
    pub(crate) fn new(r: Ref) -> Option<Box<wasm_ref_t>> {
        if r.is_null() || !r.is_func() {
            None
        } else {
            Some(Box::new(wasm_ref_t { r }))
        }
    }
}

pub(crate) fn ref_to_val(r: &wasm_ref_t) -> Val {
    Val::from(r.r.clone())
}

#[no_mangle]
pub extern "C" fn wasm_ref_copy(r: Option<&wasm_ref_t>) -> Option<Box<wasm_ref_t>> {
    r.map(|r| Box::new(r.clone()))
}

#[no_mangle]
pub extern "C" fn wasm_ref_same(_a: Option<&wasm_ref_t>, _b: Option<&wasm_ref_t>) -> bool {
    // We need a store to determine whether these are the same reference or not.
    abort("wasm_ref_same")
}

#[no_mangle]
pub extern "C" fn wasm_ref_get_host_info(_ref: Option<&wasm_ref_t>) -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn wasm_ref_set_host_info(_ref: Option<&wasm_ref_t>, _info: *mut c_void) {
    abort("wasm_ref_set_host_info")
}

#[no_mangle]
pub extern "C" fn wasm_ref_set_host_info_with_finalizer(
    _ref: Option<&wasm_ref_t>,
    _info: *mut c_void,
    _finalizer: Option<extern "C" fn(*mut c_void)>,
) {
    abort("wasm_ref_set_host_info_with_finalizer")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_extern(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_extern_t> {
    abort("wasm_ref_as_extern")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_extern_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_extern_t> {
    abort("wasm_ref_as_extern_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_foreign(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_foreign_t> {
    abort("wasm_ref_as_foreign")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_foreign_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_foreign_t> {
    abort("wasm_ref_as_foreign_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_func(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_func_t> {
    abort("wasm_ref_as_func")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_func_const(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_func_t> {
    abort("wasm_ref_as_func_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_global(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_global_t> {
    abort("wasm_ref_as_global")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_global_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_global_t> {
    abort("wasm_ref_as_global_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_instance(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_instance_t> {
    abort("wasm_ref_as_instance")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_instance_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_instance_t> {
    abort("wasm_ref_as_instance_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_memory(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_memory_t> {
    abort("wasm_ref_as_memory")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_memory_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_memory_t> {
    abort("wasm_ref_as_memory_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_module(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_module_t> {
    abort("wasm_ref_as_module")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_module_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_module_t> {
    abort("wasm_ref_as_module_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_table(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_table_t> {
    abort("wasm_ref_as_table")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_table_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_table_t> {
    abort("wasm_ref_as_table_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_trap(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_trap_t> {
    abort("wasm_ref_as_trap")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_trap_const(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_trap_t> {
    abort("wasm_ref_as_trap_const")
}

#[derive(Clone)]
#[repr(C)]
pub struct wasm_foreign_t {}

wasmtime_c_api_macros::declare_ref!(wasm_foreign_t);

#[no_mangle]
pub extern "C" fn wasm_foreign_new(_store: &crate::wasm_store_t) -> Box<wasm_foreign_t> {
    abort("wasm_foreign_new")
}

/// C-API representation of `anyref`.
///
/// This represented differently in the C API from the header to handle how
/// this is dispatched internally. Null anyref values are represented with a
/// `store_id` of zero, and otherwise the `rooted` field is valid.
///
/// Note that this relies on the Wasmtime definition of `ManuallyRooted` to have
/// a 64-bit store_id first.
pub union wasmtime_anyref_t {
    store_id: u64,
    rooted: ManuallyDrop<ManuallyRooted<AnyRef>>,
}

/// Same as `wasmtime_anyref_t`, but for extenref.
pub union wasmtime_externref_t {
    store_id: u64,
    rooted: ManuallyDrop<ManuallyRooted<ExternRef>>,
}

impl wasmtime_anyref_t {
    pub unsafe fn as_wasmtime(&self) -> Option<&ManuallyRooted<AnyRef>> {
        if self.store_id == 0 {
            None
        } else {
            Some(&self.rooted)
        }
    }

    pub unsafe fn into_wasmtime(self) -> Option<ManuallyRooted<AnyRef>> {
        if self.store_id == 0 {
            None
        } else {
            Some(ManuallyDrop::into_inner(self.rooted))
        }
    }
}

impl From<Option<ManuallyRooted<AnyRef>>> for wasmtime_anyref_t {
    fn from(rooted: Option<ManuallyRooted<AnyRef>>) -> wasmtime_anyref_t {
        match rooted {
            Some(val) => wasmtime_anyref_t {
                rooted: ManuallyDrop::new(val),
            },
            None => wasmtime_anyref_t { store_id: 0 },
        }
    }
}

impl wasmtime_externref_t {
    pub unsafe fn as_wasmtime(&self) -> Option<&ManuallyRooted<ExternRef>> {
        if self.store_id == 0 {
            None
        } else {
            Some(&self.rooted)
        }
    }

    pub unsafe fn into_wasmtime(self) -> Option<ManuallyRooted<ExternRef>> {
        if self.store_id == 0 {
            None
        } else {
            Some(ManuallyDrop::into_inner(self.rooted))
        }
    }
}

impl From<Option<ManuallyRooted<ExternRef>>> for wasmtime_externref_t {
    fn from(rooted: Option<ManuallyRooted<ExternRef>>) -> wasmtime_externref_t {
        match rooted {
            Some(val) => wasmtime_externref_t {
                rooted: ManuallyDrop::new(val),
            },
            None => wasmtime_externref_t { store_id: 0 },
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_anyref_clone(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    out: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let anyref = anyref.and_then(|a| a.as_wasmtime()).map(|a| a.clone(cx));
    crate::initialize(out, anyref.into());
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_anyref_unroot(
    cx: WasmtimeStoreContextMut<'_>,
    val: Option<&mut MaybeUninit<wasmtime_anyref_t>>,
) {
    if let Some(val) = val.and_then(|v| v.assume_init_read().into_wasmtime()) {
        val.unroot(cx);
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_anyref_to_raw(
    cx: WasmtimeStoreContextMut<'_>,
    val: Option<&wasmtime_anyref_t>,
) -> u32 {
    val.and_then(|v| v.as_wasmtime())
        .and_then(|e| e.to_raw(cx).ok())
        .unwrap_or_default()
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_anyref_from_raw(
    cx: WasmtimeStoreContextMut<'_>,
    raw: u32,
    val: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let mut scope = RootScope::new(cx);
    let anyref = AnyRef::from_raw(&mut scope, raw)
        .map(|a| a.to_manually_rooted(&mut scope).expect("in scope"));
    crate::initialize(val, anyref.into());
}

#[no_mangle]
pub extern "C" fn wasmtime_anyref_from_i31(
    cx: WasmtimeStoreContextMut<'_>,
    val: u32,
    out: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let mut scope = RootScope::new(cx);
    let anyref = AnyRef::from_i31(&mut scope, I31::wrapping_u32(val));
    let anyref = anyref.to_manually_rooted(&mut scope).expect("in scope");
    crate::initialize(out, Some(anyref).into())
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_anyref_i31_get_u(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    dst: &mut MaybeUninit<u32>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) if anyref.is_i31(&cx).expect("ManuallyRooted always in scope") => {
            let val = anyref
                .unwrap_i31(&cx)
                .expect("ManuallyRooted always in scope")
                .get_u32();
            crate::initialize(dst, val);
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_anyref_i31_get_s(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    dst: &mut MaybeUninit<i32>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) if anyref.is_i31(&cx).expect("ManuallyRooted always in scope") => {
            let val = anyref
                .unwrap_i31(&cx)
                .expect("ManuallyRooted always in scope")
                .get_i32();
            crate::initialize(dst, val);
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn wasmtime_externref_new(
    cx: WasmtimeStoreContextMut<'_>,
    data: *mut c_void,
    finalizer: Option<extern "C" fn(*mut c_void)>,
    out: &mut MaybeUninit<wasmtime_externref_t>,
) -> bool {
    let mut scope = RootScope::new(cx);
    let e = match ExternRef::new(&mut scope, crate::ForeignData { data, finalizer }) {
        Ok(e) => e,
        Err(_) => return false,
    };
    let e = e.to_manually_rooted(&mut scope).expect("in scope");
    crate::initialize(out, Some(e).into());
    true
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_externref_data(
    cx: WasmtimeStoreContextMut<'_>,
    externref: Option<&wasmtime_externref_t>,
) -> *mut c_void {
    externref
        .and_then(|e| e.as_wasmtime())
        .and_then(|e| {
            let data = e.data(cx).ok()?;
            Some(data.downcast_ref::<crate::ForeignData>().unwrap().data)
        })
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_externref_clone(
    cx: WasmtimeStoreContextMut<'_>,
    externref: Option<&wasmtime_externref_t>,
    out: &mut MaybeUninit<wasmtime_externref_t>,
) {
    let externref = externref.and_then(|e| e.as_wasmtime()).map(|e| e.clone(cx));
    crate::initialize(out, externref.into());
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_externref_unroot(
    cx: WasmtimeStoreContextMut<'_>,
    val: Option<&mut MaybeUninit<wasmtime_externref_t>>,
) {
    if let Some(val) = val.and_then(|v| v.assume_init_read().into_wasmtime()) {
        val.unroot(cx);
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_externref_to_raw(
    cx: WasmtimeStoreContextMut<'_>,
    val: Option<&wasmtime_externref_t>,
) -> u32 {
    val.and_then(|e| e.as_wasmtime())
        .and_then(|e| e.to_raw(cx).ok())
        .unwrap_or_default()
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_externref_from_raw(
    cx: WasmtimeStoreContextMut<'_>,
    raw: u32,
    val: &mut MaybeUninit<wasmtime_externref_t>,
) {
    let mut scope = RootScope::new(cx);
    let rooted = ExternRef::from_raw(&mut scope, raw)
        .map(|e| e.to_manually_rooted(&mut scope).expect("in scope"));
    crate::initialize(val, rooted.into());
}
