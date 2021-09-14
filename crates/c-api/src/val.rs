use crate::r#ref::{ref_to_val, WasmRefInner};
use crate::{
    from_valtype, into_valtype, wasm_ref_t, wasm_valkind_t, wasmtime_valkind_t, CStoreContextMut,
    WASM_I32,
};
use std::ffi::c_void;
use std::mem::{self, ManuallyDrop, MaybeUninit};
use std::ptr;
use wasmtime::{ExternRef, Func, Val, ValType};

#[repr(C)]
pub struct wasm_val_t {
    pub kind: wasm_valkind_t,
    pub of: wasm_val_union,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union wasm_val_union {
    pub i32: i32,
    pub i64: i64,
    pub u32: u32,
    pub u64: u64,
    pub f32: f32,
    pub f64: f64,
    pub ref_: *mut wasm_ref_t,
}

impl Drop for wasm_val_t {
    fn drop(&mut self) {
        match into_valtype(self.kind) {
            ValType::FuncRef | ValType::ExternRef => unsafe {
                if !self.of.ref_.is_null() {
                    drop(Box::from_raw(self.of.ref_));
                }
            },
            _ => {}
        }
    }
}

impl Clone for wasm_val_t {
    fn clone(&self) -> Self {
        let mut ret = wasm_val_t {
            kind: self.kind,
            of: self.of,
        };
        unsafe {
            match into_valtype(self.kind) {
                ValType::ExternRef | ValType::FuncRef if !self.of.ref_.is_null() => {
                    ret.of.ref_ = Box::into_raw(Box::new((*self.of.ref_).clone()));
                }
                _ => {}
            }
        }
        return ret;
    }
}

impl Default for wasm_val_t {
    fn default() -> Self {
        wasm_val_t {
            kind: WASM_I32,
            of: wasm_val_union { i32: 0 },
        }
    }
}

impl wasm_val_t {
    pub fn from_val(val: Val) -> wasm_val_t {
        match val {
            Val::I32(i) => wasm_val_t {
                kind: from_valtype(&ValType::I32),
                of: wasm_val_union { i32: i },
            },
            Val::I64(i) => wasm_val_t {
                kind: from_valtype(&ValType::I64),
                of: wasm_val_union { i64: i },
            },
            Val::F32(f) => wasm_val_t {
                kind: from_valtype(&ValType::F32),
                of: wasm_val_union { u32: f },
            },
            Val::F64(f) => wasm_val_t {
                kind: from_valtype(&ValType::F64),
                of: wasm_val_union { u64: f },
            },
            Val::ExternRef(None) => wasm_val_t {
                kind: from_valtype(&ValType::ExternRef),
                of: wasm_val_union {
                    ref_: ptr::null_mut(),
                },
            },
            Val::ExternRef(Some(r)) => wasm_val_t {
                kind: from_valtype(&ValType::ExternRef),
                of: wasm_val_union {
                    ref_: Box::into_raw(Box::new(wasm_ref_t {
                        r: WasmRefInner::ExternRef(r),
                    })),
                },
            },
            Val::FuncRef(None) => wasm_val_t {
                kind: from_valtype(&ValType::FuncRef),
                of: wasm_val_union {
                    ref_: ptr::null_mut(),
                },
            },
            Val::FuncRef(Some(f)) => wasm_val_t {
                kind: from_valtype(&ValType::FuncRef),
                of: wasm_val_union {
                    ref_: Box::into_raw(Box::new(wasm_ref_t {
                        r: WasmRefInner::FuncRef(f),
                    })),
                },
            },
            _ => unimplemented!("wasm_val_t::from_val {:?}", val),
        }
    }

    pub fn val(&self) -> Val {
        match into_valtype(self.kind) {
            ValType::I32 => Val::from(unsafe { self.of.i32 }),
            ValType::I64 => Val::from(unsafe { self.of.i64 }),
            ValType::F32 => Val::from(unsafe { self.of.f32 }),
            ValType::F64 => Val::from(unsafe { self.of.f64 }),
            ValType::ExternRef => unsafe {
                if self.of.ref_.is_null() {
                    Val::ExternRef(None)
                } else {
                    ref_to_val(&*self.of.ref_)
                }
            },
            ValType::FuncRef => unsafe {
                if self.of.ref_.is_null() {
                    Val::FuncRef(None)
                } else {
                    ref_to_val(&*self.of.ref_)
                }
            },
            _ => unimplemented!("wasm_val_t::val {:?}", self.kind),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_val_copy(out: &mut MaybeUninit<wasm_val_t>, source: &wasm_val_t) {
    crate::initialize(out, source.clone());
}

#[no_mangle]
pub unsafe extern "C" fn wasm_val_delete(val: *mut wasm_val_t) {
    ptr::drop_in_place(val);
}

#[repr(C)]
pub struct wasmtime_val_t {
    pub kind: wasmtime_valkind_t,
    pub of: wasmtime_val_union,
}

#[repr(C)]
pub union wasmtime_val_union {
    pub i32: i32,
    pub i64: i64,
    pub f32: u32,
    pub f64: u64,
    pub funcref: wasmtime_func_t,
    pub externref: ManuallyDrop<Option<ExternRef>>,
    pub v128: [u8; 16],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct wasmtime_func_t {
    pub store_id: u64,
    pub index: usize,
}

impl wasmtime_val_t {
    pub fn from_val(val: Val) -> wasmtime_val_t {
        match val {
            Val::I32(i) => wasmtime_val_t {
                kind: crate::WASMTIME_I32,
                of: wasmtime_val_union { i32: i },
            },
            Val::I64(i) => wasmtime_val_t {
                kind: crate::WASMTIME_I64,
                of: wasmtime_val_union { i64: i },
            },
            Val::F32(i) => wasmtime_val_t {
                kind: crate::WASMTIME_F32,
                of: wasmtime_val_union { f32: i },
            },
            Val::F64(i) => wasmtime_val_t {
                kind: crate::WASMTIME_F64,
                of: wasmtime_val_union { f64: i },
            },
            Val::ExternRef(i) => wasmtime_val_t {
                kind: crate::WASMTIME_EXTERNREF,
                of: wasmtime_val_union {
                    externref: ManuallyDrop::new(i),
                },
            },
            Val::FuncRef(i) => wasmtime_val_t {
                kind: crate::WASMTIME_FUNCREF,
                of: wasmtime_val_union {
                    funcref: match i {
                        Some(func) => unsafe { mem::transmute::<Func, wasmtime_func_t>(func) },
                        None => wasmtime_func_t {
                            store_id: 0,
                            index: 0,
                        },
                    },
                },
            },
            Val::V128(val) => wasmtime_val_t {
                kind: crate::WASMTIME_V128,
                of: wasmtime_val_union {
                    v128: val.to_le_bytes(),
                },
            },
        }
    }

    pub unsafe fn to_val(&self) -> Val {
        match self.kind {
            crate::WASMTIME_I32 => Val::I32(self.of.i32),
            crate::WASMTIME_I64 => Val::I64(self.of.i64),
            crate::WASMTIME_F32 => Val::F32(self.of.f32),
            crate::WASMTIME_F64 => Val::F64(self.of.f64),
            crate::WASMTIME_V128 => Val::V128(u128::from_le_bytes(self.of.v128)),
            crate::WASMTIME_FUNCREF => {
                let store = self.of.funcref.store_id;
                let index = self.of.funcref.index;
                Val::FuncRef(if store == 0 && index == 0 {
                    None
                } else {
                    Some(mem::transmute::<wasmtime_func_t, Func>(self.of.funcref))
                })
            }
            crate::WASMTIME_EXTERNREF => Val::ExternRef((*self.of.externref).clone()),
            other => panic!("unknown wasmtime_valkind_t: {}", other),
        }
    }
}

impl Drop for wasmtime_val_t {
    fn drop(&mut self) {
        if self.kind == crate::WASMTIME_EXTERNREF {
            unsafe {
                ManuallyDrop::drop(&mut self.of.externref);
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_val_delete(val: &mut ManuallyDrop<wasmtime_val_t>) {
    ManuallyDrop::drop(val)
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_val_copy(
    dst: &mut MaybeUninit<wasmtime_val_t>,
    src: &wasmtime_val_t,
) {
    crate::initialize(dst, wasmtime_val_t::from_val(src.to_val()))
}

#[no_mangle]
pub extern "C" fn wasmtime_externref_new(
    data: *mut c_void,
    finalizer: Option<extern "C" fn(*mut c_void)>,
) -> ExternRef {
    ExternRef::new(crate::ForeignData { data, finalizer })
}

#[no_mangle]
pub extern "C" fn wasmtime_externref_data(externref: ManuallyDrop<ExternRef>) -> *mut c_void {
    externref
        .data()
        .downcast_ref::<crate::ForeignData>()
        .unwrap()
        .data
}

#[no_mangle]
pub extern "C" fn wasmtime_externref_clone(externref: ManuallyDrop<ExternRef>) -> ExternRef {
    (*externref).clone()
}

#[no_mangle]
pub extern "C" fn wasmtime_externref_delete(_val: Option<ExternRef>) {}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_externref_to_raw(
    cx: CStoreContextMut<'_>,
    val: Option<ManuallyDrop<ExternRef>>,
) -> usize {
    match val {
        Some(ptr) => ptr.to_raw(cx),
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_externref_from_raw(
    _cx: CStoreContextMut<'_>,
    val: usize,
) -> Option<ExternRef> {
    ExternRef::from_raw(val)
}
