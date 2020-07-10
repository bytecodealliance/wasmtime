use crate::r#ref::{ref_to_val, WasmRefInner};
use crate::{from_valtype, into_valtype, wasm_ref_t, wasm_valkind_t, WASM_I32};
use std::mem::MaybeUninit;
use std::ptr;
use wasmtime::{Val, ValType};

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
            ValType::ExternRef => unsafe {
                drop(Box::from_raw(self.of.ref_));
            },
            _ => {}
        }
    }
}

impl Clone for wasm_val_t {
    fn clone(&self) -> Self {
        match into_valtype(self.kind) {
            ValType::ExternRef => wasm_val_t {
                kind: self.kind,
                of: wasm_val_union {
                    ref_: unsafe { Box::into_raw(Box::new((*self.of.ref_).clone())) },
                },
            },
            _ => wasm_val_t {
                kind: self.kind,
                of: self.of,
            },
        }
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
            ValType::ExternRef | ValType::FuncRef => ref_to_val(unsafe { &*self.of.ref_ }),
            _ => unimplemented!("wasm_val_t::val {:?}", self.kind),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_val_copy(out: &mut MaybeUninit<wasm_val_t>, source: &wasm_val_t) {
    ptr::write(
        out.as_mut_ptr(),
        match into_valtype(source.kind) {
            ValType::I32
            | ValType::I64
            | ValType::F32
            | ValType::F64
            | ValType::ExternRef
            | ValType::FuncRef => source.clone(),
            _ => unimplemented!("wasm_val_copy arg"),
        },
    );
}

#[no_mangle]
pub unsafe extern "C" fn wasm_val_delete(val: *mut wasm_val_t) {
    ptr::drop_in_place(val);
}
