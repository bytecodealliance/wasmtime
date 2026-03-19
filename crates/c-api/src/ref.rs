#![cfg(feature = "gc")]

use crate::WasmtimeStoreContextMut;
use crate::abort;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::{num::NonZeroU64, os::raw::c_void, ptr};
use wasmtime::{
    AnyRef, ArrayRef, ArrayRefPre, ArrayType, EqRef, ExnRef, ExternRef, FieldType, I31, Mutability,
    OwnedRooted, Ref, RootScope, StorageType, StructRef, StructRefPre, StructType, Val, ValType,
};

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

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_copy(r: Option<&wasm_ref_t>) -> Option<Box<wasm_ref_t>> {
    r.map(|r| Box::new(r.clone()))
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_same(_a: Option<&wasm_ref_t>, _b: Option<&wasm_ref_t>) -> bool {
    // We need a store to determine whether these are the same reference or not.
    abort("wasm_ref_same")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_get_host_info(_ref: Option<&wasm_ref_t>) -> *mut c_void {
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_set_host_info(_ref: Option<&wasm_ref_t>, _info: *mut c_void) {
    abort("wasm_ref_set_host_info")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_set_host_info_with_finalizer(
    _ref: Option<&wasm_ref_t>,
    _info: *mut c_void,
    _finalizer: Option<extern "C" fn(*mut c_void)>,
) {
    abort("wasm_ref_set_host_info_with_finalizer")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_extern(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_extern_t> {
    abort("wasm_ref_as_extern")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_extern_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_extern_t> {
    abort("wasm_ref_as_extern_const")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_foreign(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_foreign_t> {
    abort("wasm_ref_as_foreign")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_foreign_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_foreign_t> {
    abort("wasm_ref_as_foreign_const")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_func(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_func_t> {
    abort("wasm_ref_as_func")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_func_const(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_func_t> {
    abort("wasm_ref_as_func_const")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_global(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_global_t> {
    abort("wasm_ref_as_global")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_global_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_global_t> {
    abort("wasm_ref_as_global_const")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_instance(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_instance_t> {
    abort("wasm_ref_as_instance")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_instance_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_instance_t> {
    abort("wasm_ref_as_instance_const")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_memory(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_memory_t> {
    abort("wasm_ref_as_memory")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_memory_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_memory_t> {
    abort("wasm_ref_as_memory_const")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_module(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_module_t> {
    abort("wasm_ref_as_module")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_module_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_module_t> {
    abort("wasm_ref_as_module_const")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_table(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_table_t> {
    abort("wasm_ref_as_table")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_table_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_table_t> {
    abort("wasm_ref_as_table_const")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_trap(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_trap_t> {
    abort("wasm_ref_as_trap")
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ref_as_trap_const(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_trap_t> {
    abort("wasm_ref_as_trap_const")
}

#[derive(Clone)]
#[repr(C)]
pub struct wasm_foreign_t {}

wasmtime_c_api_macros::declare_ref!(wasm_foreign_t);

#[unsafe(no_mangle)]
pub extern "C" fn wasm_foreign_new(_store: &crate::wasm_store_t) -> Box<wasm_foreign_t> {
    abort("wasm_foreign_new")
}

/// C-API representation of `anyref`.
///
/// This represented differently in the C API from the header to handle how
/// this is dispatched internally. Null anyref values are represented with a
/// `store_id` of zero, and otherwise the `rooted` field is valid.
///
/// Note that this relies on the Wasmtime definition of `OwnedRooted` to have
/// a 64-bit store_id first.
macro_rules! ref_wrapper {
    ($wasmtime:ident => $c:ident) => {
        pub struct $c {
            store_id: u64,
            a: u32,
            b: u32,
            c: *const (),
        }

        impl $c {
            pub unsafe fn as_wasmtime(&self) -> Option<OwnedRooted<$wasmtime>> {
                let store_id = NonZeroU64::new(self.store_id)?;
                Some(OwnedRooted::from_borrowed_raw_parts_for_c_api(
                    store_id, self.a, self.b, self.c,
                ))
            }

            pub unsafe fn into_wasmtime(self) -> Option<OwnedRooted<$wasmtime>> {
                ManuallyDrop::new(self).to_owned()
            }

            unsafe fn to_owned(&self) -> Option<OwnedRooted<$wasmtime>> {
                let store_id = NonZeroU64::new(self.store_id)?;
                Some(OwnedRooted::from_owned_raw_parts_for_c_api(
                    store_id, self.a, self.b, self.c,
                ))
            }
        }

        impl Drop for $c {
            fn drop(&mut self) {
                unsafe {
                    let _ = self.to_owned();
                }
            }
        }

        impl From<Option<OwnedRooted<$wasmtime>>> for $c {
            fn from(rooted: Option<OwnedRooted<$wasmtime>>) -> $c {
                let mut ret = $c {
                    store_id: 0,
                    a: 0,
                    b: 0,
                    c: core::ptr::null(),
                };
                if let Some(rooted) = rooted {
                    let (store_id, a, b, c) = rooted.into_parts_for_c_api();
                    ret.store_id = store_id.get();
                    ret.a = a;
                    ret.b = b;
                    ret.c = c;
                }
                ret
            }
        }

        impl From<OwnedRooted<$wasmtime>> for $c {
            fn from(rooted: OwnedRooted<$wasmtime>) -> $c {
                Self::from(Some(rooted))
            }
        }

        // SAFETY: The `*const ()` comes from (and is converted back
        // into) an `Arc<()>`, and is only accessed as such, so this
        // type is both Send and Sync. These constraints are necessary
        // in the async machinery in this crate.
        unsafe impl Send for $c {}
        unsafe impl Sync for $c {}
    };
}

ref_wrapper!(AnyRef => wasmtime_anyref_t);
ref_wrapper!(ExternRef => wasmtime_externref_t);
ref_wrapper!(EqRef => wasmtime_eqref_t);
ref_wrapper!(StructRef => wasmtime_structref_t);
ref_wrapper!(ExnRef => wasmtime_exnref_t);

// Opaque types for struct type and struct ref pre-allocator
pub struct wasmtime_struct_type_t {
    ty: StructType,
}
wasmtime_c_api_macros::declare_own!(wasmtime_struct_type_t);

pub struct wasmtime_struct_ref_pre_t {
    pre: StructRefPre,
}
wasmtime_c_api_macros::declare_own!(wasmtime_struct_ref_pre_t);

ref_wrapper!(ArrayRef => wasmtime_arrayref_t);

pub struct wasmtime_array_type_t {
    ty: ArrayType,
}
wasmtime_c_api_macros::declare_own!(wasmtime_array_type_t);

pub struct wasmtime_array_ref_pre_t {
    pre: ArrayRefPre,
}
wasmtime_c_api_macros::declare_own!(wasmtime_array_ref_pre_t);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_clone(
    anyref: Option<&wasmtime_anyref_t>,
    out: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let anyref = anyref.and_then(|a| a.as_wasmtime());
    crate::initialize(out, anyref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_unroot(val: Option<&mut ManuallyDrop<wasmtime_anyref_t>>) {
    if let Some(val) = val {
        unsafe {
            ManuallyDrop::drop(val);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_to_raw(
    cx: WasmtimeStoreContextMut<'_>,
    val: Option<&wasmtime_anyref_t>,
) -> u32 {
    val.and_then(|v| v.as_wasmtime())
        .and_then(|e| e.to_raw(cx).ok())
        .unwrap_or_default()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_from_raw(
    cx: WasmtimeStoreContextMut<'_>,
    raw: u32,
    val: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let mut scope = RootScope::new(cx);
    let anyref =
        AnyRef::from_raw(&mut scope, raw).map(|a| a.to_owned_rooted(&mut scope).expect("in scope"));
    crate::initialize(val, anyref.into());
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_anyref_from_i31(
    cx: WasmtimeStoreContextMut<'_>,
    val: u32,
    out: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let mut scope = RootScope::new(cx);
    let anyref = AnyRef::from_i31(&mut scope, I31::wrapping_u32(val));
    let anyref = anyref.to_owned_rooted(&mut scope).expect("in scope");
    crate::initialize(out, Some(anyref).into())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_is_i31(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) => anyref.is_i31(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_i31_get_u(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    dst: &mut MaybeUninit<u32>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) if anyref.is_i31(&cx).expect("OwnedRooted always in scope") => {
            let val = anyref
                .unwrap_i31(&cx)
                .expect("OwnedRooted always in scope")
                .get_u32();
            crate::initialize(dst, val);
            true
        }
        _ => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_i31_get_s(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    dst: &mut MaybeUninit<i32>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) if anyref.is_i31(&cx).expect("OwnedRooted always in scope") => {
            let val = anyref
                .unwrap_i31(&cx)
                .expect("OwnedRooted always in scope")
                .get_i32();
            crate::initialize(dst, val);
            true
        }
        _ => false,
    }
}

#[unsafe(no_mangle)]
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
    let e = e.to_owned_rooted(&mut scope).expect("in scope");
    crate::initialize(out, Some(e).into());
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_externref_data(
    cx: WasmtimeStoreContextMut<'_>,
    externref: Option<&wasmtime_externref_t>,
) -> *mut c_void {
    externref
        .and_then(|e| e.as_wasmtime())
        .and_then(|e| {
            let data = e.data(cx).ok()??;
            Some(data.downcast_ref::<crate::ForeignData>().unwrap().data)
        })
        .unwrap_or(core::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_externref_clone(
    externref: Option<&wasmtime_externref_t>,
    out: &mut MaybeUninit<wasmtime_externref_t>,
) {
    let externref = externref.and_then(|e| e.as_wasmtime());
    crate::initialize(out, externref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_externref_unroot(
    val: Option<&mut ManuallyDrop<wasmtime_externref_t>>,
) {
    if let Some(val) = val {
        unsafe {
            ManuallyDrop::drop(val);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_externref_to_raw(
    cx: WasmtimeStoreContextMut<'_>,
    val: Option<&wasmtime_externref_t>,
) -> u32 {
    val.and_then(|e| e.as_wasmtime())
        .and_then(|e| e.to_raw(cx).ok())
        .unwrap_or_default()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_externref_from_raw(
    cx: WasmtimeStoreContextMut<'_>,
    raw: u32,
    val: &mut MaybeUninit<wasmtime_externref_t>,
) {
    let mut scope = RootScope::new(cx);
    let rooted = ExternRef::from_raw(&mut scope, raw)
        .map(|e| e.to_owned_rooted(&mut scope).expect("in scope"));
    crate::initialize(val, rooted.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_exnref_clone(
    exnref: Option<&wasmtime_exnref_t>,
    out: &mut MaybeUninit<wasmtime_exnref_t>,
) {
    let exnref = exnref.and_then(|e| e.as_wasmtime());
    crate::initialize(out, exnref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_exnref_unroot(val: Option<&mut ManuallyDrop<wasmtime_exnref_t>>) {
    if let Some(val) = val {
        unsafe {
            ManuallyDrop::drop(val);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_clone(
    eqref: Option<&wasmtime_eqref_t>,
    out: &mut MaybeUninit<wasmtime_eqref_t>,
) {
    let eqref = eqref.and_then(|e| e.as_wasmtime());
    crate::initialize(out, eqref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_unroot(val: Option<&mut ManuallyDrop<wasmtime_eqref_t>>) {
    if let Some(val) = val {
        unsafe {
            ManuallyDrop::drop(val);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_to_anyref(
    eqref: Option<&wasmtime_eqref_t>,
    out: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let anyref = eqref.and_then(|e| e.as_wasmtime()).map(|e| e.to_anyref());
    crate::initialize(out, anyref.into());
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_eqref_from_i31(
    cx: WasmtimeStoreContextMut<'_>,
    val: u32,
    out: &mut MaybeUninit<wasmtime_eqref_t>,
) {
    let mut scope = RootScope::new(cx);
    let eqref = EqRef::from_i31(&mut scope, I31::wrapping_u32(val));
    let eqref = eqref.to_owned_rooted(&mut scope).expect("in scope");
    crate::initialize(out, Some(eqref).into())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_is_i31(
    cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
) -> bool {
    match eqref.and_then(|e| e.as_wasmtime()) {
        Some(eqref) => eqref.is_i31(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_i31_get_u(
    cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
    dst: &mut MaybeUninit<u32>,
) -> bool {
    let mut scope = RootScope::new(cx);
    if let Some(eqref) = eqref.and_then(|e| e.as_wasmtime()) {
        if let Some(val) = eqref.as_i31(&mut scope).expect("in scope") {
            crate::initialize(dst, val.get_u32());
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_i31_get_s(
    cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
    dst: &mut MaybeUninit<i32>,
) -> bool {
    let mut scope = RootScope::new(cx);
    if let Some(eqref) = eqref.and_then(|e| e.as_wasmtime()) {
        if let Some(val) = eqref.as_i31(&mut scope).expect("in scope") {
            crate::initialize(dst, val.get_i32());
            return true;
        }
    }
    false
}

pub type wasmtime_storage_kind_t = u8;
pub const WASMTIME_STORAGE_KIND_I8: wasmtime_storage_kind_t = 9;
pub const WASMTIME_STORAGE_KIND_I16: wasmtime_storage_kind_t = 10;

#[repr(C)]
pub struct wasmtime_field_type_t {
    pub kind: wasmtime_storage_kind_t,
    pub mutable_: bool,
}

fn field_type_from_c(ft: &wasmtime_field_type_t) -> FieldType {
    let mutability = if ft.mutable_ {
        Mutability::Var
    } else {
        Mutability::Const
    };
    let storage = match ft.kind {
        WASMTIME_STORAGE_KIND_I8 => StorageType::I8,
        WASMTIME_STORAGE_KIND_I16 => StorageType::I16,
        crate::WASMTIME_I32 => StorageType::ValType(ValType::I32),
        crate::WASMTIME_I64 => StorageType::ValType(ValType::I64),
        crate::WASMTIME_F32 => StorageType::ValType(ValType::F32),
        crate::WASMTIME_F64 => StorageType::ValType(ValType::F64),
        crate::WASMTIME_V128 => StorageType::ValType(ValType::V128),
        crate::WASMTIME_FUNCREF => StorageType::ValType(ValType::FUNCREF),
        crate::WASMTIME_EXTERNREF => StorageType::ValType(ValType::EXTERNREF),
        crate::WASMTIME_ANYREF => StorageType::ValType(ValType::ANYREF),
        crate::WASMTIME_EXNREF => StorageType::ValType(ValType::EXNREF),
        other => panic!("unknown wasmtime_storage_kind_t: {other}"),
    };
    FieldType::new(mutability, storage)
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_struct_type_new(
    engine: &crate::wasm_engine_t,
    fields: *const wasmtime_field_type_t,
    nfields: usize,
) -> Box<wasmtime_struct_type_t> {
    let fields = if nfields == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(fields, nfields) }
    };
    let field_types: Vec<FieldType> = fields.iter().map(field_type_from_c).collect();
    let ty = StructType::new(&engine.engine, field_types).expect("failed to create struct type");
    Box::new(wasmtime_struct_type_t { ty })
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_struct_ref_pre_new(
    cx: WasmtimeStoreContextMut<'_>,
    ty: &wasmtime_struct_type_t,
) -> Box<wasmtime_struct_ref_pre_t> {
    let pre = StructRefPre::new(cx, ty.ty.clone());
    Box::new(wasmtime_struct_ref_pre_t { pre })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_structref_new(
    mut cx: WasmtimeStoreContextMut<'_>,
    pre: &wasmtime_struct_ref_pre_t,
    fields: *const crate::wasmtime_val_t,
    nfields: usize,
    out: &mut MaybeUninit<wasmtime_structref_t>,
) -> Option<Box<crate::wasmtime_error_t>> {
    let c_fields = if nfields == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(fields, nfields)
    };
    let mut scope = RootScope::new(&mut cx);
    let vals: Vec<Val> = c_fields.iter().map(|v| v.to_val(&mut scope)).collect();
    match StructRef::new(&mut scope, &pre.pre, &vals) {
        Ok(structref) => {
            let owned = structref
                .to_owned_rooted(&mut scope)
                .expect("just allocated");
            crate::initialize(out, Some(owned).into());
            None
        }
        Err(e) => {
            crate::initialize(out, None::<OwnedRooted<StructRef>>.into());
            Some(Box::new(e.into()))
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_structref_clone(
    structref: Option<&wasmtime_structref_t>,
    out: &mut MaybeUninit<wasmtime_structref_t>,
) {
    let structref = structref.and_then(|s| s.as_wasmtime());
    crate::initialize(out, structref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_structref_unroot(
    structref: Option<&mut ManuallyDrop<wasmtime_structref_t>>,
) {
    if let Some(structref) = structref {
        ManuallyDrop::drop(structref);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_structref_to_anyref(
    structref: Option<&wasmtime_structref_t>,
    out: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let anyref = structref
        .and_then(|s| s.as_wasmtime())
        .map(|s| s.to_anyref());
    crate::initialize(out, anyref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_structref_to_eqref(
    structref: Option<&wasmtime_structref_t>,
    out: &mut MaybeUninit<wasmtime_eqref_t>,
) {
    let eqref = structref
        .and_then(|s| s.as_wasmtime())
        .map(|s| s.to_eqref());
    crate::initialize(out, eqref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_structref_field(
    mut cx: WasmtimeStoreContextMut<'_>,
    structref: Option<&wasmtime_structref_t>,
    index: usize,
    out: &mut MaybeUninit<crate::wasmtime_val_t>,
) -> Option<Box<crate::wasmtime_error_t>> {
    let structref = structref
        .and_then(|s| s.as_wasmtime())
        .expect("non-null structref required");
    let mut scope = RootScope::new(&mut cx);
    let rooted = structref.to_rooted(&mut scope);
    match rooted.field(&mut scope, index) {
        Ok(val) => {
            let c_val = crate::wasmtime_val_t::from_val(&mut scope, val);
            crate::initialize(out, c_val);
            None
        }
        Err(e) => Some(Box::new(e.into())),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_structref_set_field(
    mut cx: WasmtimeStoreContextMut<'_>,
    structref: Option<&wasmtime_structref_t>,
    index: usize,
    val: &crate::wasmtime_val_t,
) -> Option<Box<crate::wasmtime_error_t>> {
    let structref = structref
        .and_then(|s| s.as_wasmtime())
        .expect("non-null structref required");
    let mut scope = RootScope::new(&mut cx);
    let rooted = structref.to_rooted(&mut scope);
    let rust_val = val.to_val(&mut scope);
    match rooted.set_field(&mut scope, index, rust_val) {
        Ok(()) => None,
        Err(e) => Some(Box::new(e.into())),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_is_struct(
    cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
) -> bool {
    match eqref.and_then(|e| e.as_wasmtime()) {
        Some(eqref) => eqref.is_struct(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_as_struct(
    mut cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
    out: &mut MaybeUninit<wasmtime_structref_t>,
) -> bool {
    if let Some(eqref) = eqref.and_then(|e| e.as_wasmtime()) {
        let mut scope = RootScope::new(&mut cx);
        let rooted = eqref.to_rooted(&mut scope);
        if let Ok(Some(structref)) = rooted.as_struct(&scope) {
            let owned = structref.to_owned_rooted(&mut scope).expect("in scope");
            crate::initialize(out, Some(owned).into());
            return true;
        }
    }
    crate::initialize(out, None::<OwnedRooted<StructRef>>.into());
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_array_type_new(
    engine: &crate::wasm_engine_t,
    field: &wasmtime_field_type_t,
) -> Box<wasmtime_array_type_t> {
    let ft = field_type_from_c(field);
    let ty = ArrayType::new(&engine.engine, ft);
    Box::new(wasmtime_array_type_t { ty })
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_array_ref_pre_new(
    cx: WasmtimeStoreContextMut<'_>,
    ty: &wasmtime_array_type_t,
) -> Box<wasmtime_array_ref_pre_t> {
    let pre = ArrayRefPre::new(cx, ty.ty.clone());
    Box::new(wasmtime_array_ref_pre_t { pre })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_new(
    mut cx: WasmtimeStoreContextMut<'_>,
    pre: &wasmtime_array_ref_pre_t,
    elem: &crate::wasmtime_val_t,
    len: u32,
    out: &mut MaybeUninit<wasmtime_arrayref_t>,
) -> Option<Box<crate::wasmtime_error_t>> {
    let mut scope = RootScope::new(&mut cx);
    let val = elem.to_val(&mut scope);
    match ArrayRef::new(&mut scope, &pre.pre, &val, len) {
        Ok(arrayref) => {
            let owned = arrayref
                .to_owned_rooted(&mut scope)
                .expect("just allocated");
            crate::initialize(out, Some(owned).into());
            None
        }
        Err(e) => {
            crate::initialize(out, None::<OwnedRooted<ArrayRef>>.into());
            Some(Box::new(e.into()))
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_clone(
    arrayref: Option<&wasmtime_arrayref_t>,
    out: &mut MaybeUninit<wasmtime_arrayref_t>,
) {
    let arrayref = arrayref.and_then(|a| a.as_wasmtime());
    crate::initialize(out, arrayref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_unroot(
    arrayref: Option<&mut ManuallyDrop<wasmtime_arrayref_t>>,
) {
    if let Some(arrayref) = arrayref {
        ManuallyDrop::drop(arrayref);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_to_anyref(
    arrayref: Option<&wasmtime_arrayref_t>,
    out: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let anyref = arrayref
        .and_then(|a| a.as_wasmtime())
        .map(|a| a.to_anyref());
    crate::initialize(out, anyref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_to_eqref(
    arrayref: Option<&wasmtime_arrayref_t>,
    out: &mut MaybeUninit<wasmtime_eqref_t>,
) {
    let eqref = arrayref.and_then(|a| a.as_wasmtime()).map(|a| a.to_eqref());
    crate::initialize(out, eqref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_len(
    cx: WasmtimeStoreContextMut<'_>,
    arrayref: Option<&wasmtime_arrayref_t>,
    out: &mut MaybeUninit<u32>,
) -> Option<Box<crate::wasmtime_error_t>> {
    let arrayref = arrayref
        .and_then(|a| a.as_wasmtime())
        .expect("non-null arrayref required");
    match arrayref.len(&cx) {
        Ok(len) => {
            crate::initialize(out, len);
            None
        }
        Err(e) => Some(Box::new(e.into())),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_get(
    mut cx: WasmtimeStoreContextMut<'_>,
    arrayref: Option<&wasmtime_arrayref_t>,
    index: u32,
    out: &mut MaybeUninit<crate::wasmtime_val_t>,
) -> Option<Box<crate::wasmtime_error_t>> {
    let arrayref = arrayref
        .and_then(|a| a.as_wasmtime())
        .expect("non-null arrayref required");
    let mut scope = RootScope::new(&mut cx);
    let rooted = arrayref.to_rooted(&mut scope);
    match rooted.get(&mut scope, index) {
        Ok(val) => {
            let c_val = crate::wasmtime_val_t::from_val(&mut scope, val);
            crate::initialize(out, c_val);
            None
        }
        Err(e) => Some(Box::new(e.into())),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_set(
    mut cx: WasmtimeStoreContextMut<'_>,
    arrayref: Option<&wasmtime_arrayref_t>,
    index: u32,
    val: &crate::wasmtime_val_t,
) -> Option<Box<crate::wasmtime_error_t>> {
    let arrayref = arrayref
        .and_then(|a| a.as_wasmtime())
        .expect("non-null arrayref required");
    let mut scope = RootScope::new(&mut cx);
    let rooted = arrayref.to_rooted(&mut scope);
    let rust_val = val.to_val(&mut scope);
    match rooted.set(&mut scope, index, rust_val) {
        Ok(()) => None,
        Err(e) => Some(Box::new(e.into())),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_is_array(
    cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
) -> bool {
    match eqref.and_then(|e| e.as_wasmtime()) {
        Some(eqref) => eqref.is_array(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_as_array(
    mut cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
    out: &mut MaybeUninit<wasmtime_arrayref_t>,
) -> bool {
    if let Some(eqref) = eqref.and_then(|e| e.as_wasmtime()) {
        let mut scope = RootScope::new(&mut cx);
        let rooted = eqref.to_rooted(&mut scope);
        if let Ok(Some(arrayref)) = rooted.as_array(&scope) {
            let owned = arrayref.to_owned_rooted(&mut scope).expect("just created");
            crate::initialize(out, Some(owned).into());
            return true;
        }
    }
    crate::initialize(out, None::<OwnedRooted<ArrayRef>>.into());
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_is_eqref(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) => anyref.is_eqref(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_as_eqref(
    mut cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    out: &mut MaybeUninit<wasmtime_eqref_t>,
) -> bool {
    if let Some(anyref) = anyref.and_then(|a| a.as_wasmtime()) {
        let mut scope = RootScope::new(&mut cx);
        let rooted = anyref.to_rooted(&mut scope);
        if let Ok(Some(eqref)) = rooted.as_eqref(&mut scope) {
            let owned = eqref.to_owned_rooted(&mut scope).expect("in scope");
            crate::initialize(out, Some(owned).into());
            return true;
        }
    }
    crate::initialize(out, None::<OwnedRooted<EqRef>>.into());
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_is_struct(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) => anyref.is_struct(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_as_struct(
    mut cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    out: &mut MaybeUninit<wasmtime_structref_t>,
) -> bool {
    if let Some(anyref) = anyref.and_then(|a| a.as_wasmtime()) {
        let mut scope = RootScope::new(&mut cx);
        let rooted = anyref.to_rooted(&mut scope);
        if let Ok(Some(structref)) = rooted.as_struct(&scope) {
            let owned = structref.to_owned_rooted(&mut scope).expect("in scope");
            crate::initialize(out, Some(owned).into());
            return true;
        }
    }
    crate::initialize(out, None::<OwnedRooted<StructRef>>.into());
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_is_array(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) => anyref.is_array(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_as_array(
    mut cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    out: &mut MaybeUninit<wasmtime_arrayref_t>,
) -> bool {
    if let Some(anyref) = anyref.and_then(|a| a.as_wasmtime()) {
        let mut scope = RootScope::new(&mut cx);
        let rooted = anyref.to_rooted(&mut scope);
        if let Ok(Some(arrayref)) = rooted.as_array(&scope) {
            let owned = arrayref.to_owned_rooted(&mut scope).expect("in scope");
            crate::initialize(out, Some(owned).into());
            return true;
        }
    }
    crate::initialize(out, None::<OwnedRooted<ArrayRef>>.into());
    false
}
