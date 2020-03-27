use crate::{from_valtype, into_valtype, wasm_ref_t, wasm_valkind_t, WASM_I32};
use wasmtime::{Val, ValType};

#[repr(C)]
#[derive(Copy, Clone)]
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

impl Default for wasm_val_t {
    fn default() -> Self {
        wasm_val_t {
            kind: WASM_I32,
            of: wasm_val_union { i32: 0 },
        }
    }
}

impl wasm_val_t {
    pub fn from_val(val: &Val) -> wasm_val_t {
        match val {
            Val::I32(i) => wasm_val_t {
                kind: from_valtype(&ValType::I32),
                of: wasm_val_union { i32: *i },
            },
            Val::I64(i) => wasm_val_t {
                kind: from_valtype(&ValType::I64),
                of: wasm_val_union { i64: *i },
            },
            Val::F32(f) => wasm_val_t {
                kind: from_valtype(&ValType::F32),
                of: wasm_val_union { u32: *f },
            },
            Val::F64(f) => wasm_val_t {
                kind: from_valtype(&ValType::F64),
                of: wasm_val_union { u64: *f },
            },
            _ => unimplemented!("wasm_val_t::from_val {:?}", val),
        }
    }

    pub fn set(&mut self, val: Val) {
        match val {
            Val::I32(i) => {
                self.kind = from_valtype(&ValType::I32);
                self.of = wasm_val_union { i32: i };
            }
            Val::I64(i) => {
                self.kind = from_valtype(&ValType::I64);
                self.of = wasm_val_union { i64: i };
            }
            Val::F32(f) => {
                self.kind = from_valtype(&ValType::F32);
                self.of = wasm_val_union { u32: f };
            }
            Val::F64(f) => {
                self.kind = from_valtype(&ValType::F64);
                self.of = wasm_val_union { u64: f };
            }
            _ => unimplemented!("wasm_val_t::from_val {:?}", val),
        }
    }

    pub fn val(&self) -> Val {
        match into_valtype(self.kind) {
            ValType::I32 => Val::from(unsafe { self.of.i32 }),
            ValType::I64 => Val::from(unsafe { self.of.i64 }),
            ValType::F32 => Val::from(unsafe { self.of.f32 }),
            ValType::F64 => Val::from(unsafe { self.of.f64 }),
            _ => unimplemented!("wasm_val_t::val {:?}", self.kind),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_val_copy(out: *mut wasm_val_t, source: &wasm_val_t) {
    *out = match into_valtype(source.kind) {
        ValType::I32 | ValType::I64 | ValType::F32 | ValType::F64 => *source,
        _ => unimplemented!("wasm_val_copy arg"),
    };
}

#[no_mangle]
pub extern "C" fn wasm_val_delete(_val: &mut wasm_val_t) {
    // currently we only support integers/floats which need no deletion
}
