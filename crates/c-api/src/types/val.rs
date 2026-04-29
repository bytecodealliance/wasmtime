use crate::{
    wasm_engine_t, wasm_functype_t, wasmtime_array_type_t, wasmtime_exn_type_t,
    wasmtime_struct_type_t,
};
use std::mem::{ManuallyDrop, MaybeUninit};
use wasmtime::{HeapType, RefType, ValType};

pub type wasm_valtype_t = ValType;

wasmtime_c_api_macros::declare_ty!(wasm_valtype_t);

pub type wasm_valkind_t = u8;
pub const WASM_I32: wasm_valkind_t = 0;
pub const WASM_I64: wasm_valkind_t = 1;
pub const WASM_F32: wasm_valkind_t = 2;
pub const WASM_F64: wasm_valkind_t = 3;
pub const WASM_EXTERNREF: wasm_valkind_t = 128;
pub const WASM_FUNCREF: wasm_valkind_t = 129;

#[unsafe(no_mangle)]
pub extern "C" fn wasm_valtype_new(kind: wasm_valkind_t) -> Box<wasm_valtype_t> {
    Box::new(match kind {
        WASM_I32 => ValType::I32,
        WASM_I64 => ValType::I64,
        WASM_F32 => ValType::F32,
        WASM_F64 => ValType::F64,
        WASM_EXTERNREF => ValType::EXTERNREF,
        WASM_FUNCREF => ValType::FUNCREF,
        _ => crate::abort("unexpected value type kind"),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_valtype_kind(vt: &wasm_valtype_t) -> wasm_valkind_t {
    match vt {
        ValType::I32 => WASM_I32,
        ValType::I64 => WASM_I64,
        ValType::F32 => WASM_F32,
        ValType::F64 => WASM_F64,
        // TODO
        ValType::V128 => crate::abort("support for v128 "),
        ValType::Ref(r) => match (r.is_nullable(), r.heap_type()) {
            (true, HeapType::Extern) => WASM_EXTERNREF,
            (true, HeapType::Func) => WASM_FUNCREF,
            // TODO
            _ => crate::abort("support for non-externref and non-funcref references"),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_wasm_valtype_v128() -> Box<wasm_valtype_t> {
    Box::new(ValType::V128)
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_wasm_valtype_equal(a: &wasm_valtype_t, b: &wasm_valtype_t) -> bool {
    ValType::eq(a, b)
}

#[repr(C, u8)]
#[derive(Clone)]
pub enum wasmtime_valtype_t {
    I32,
    I64,
    F32,
    F64,
    V128,
    Ref(wasmtime_reftype_t),
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_valtype_new(
    ty: &wasm_valtype_t,
    ret: &mut MaybeUninit<wasmtime_valtype_t>,
) {
    ret.write(match ty {
        ValType::I32 => wasmtime_valtype_t::I32,
        ValType::I64 => wasmtime_valtype_t::I64,
        ValType::F32 => wasmtime_valtype_t::F32,
        ValType::F64 => wasmtime_valtype_t::F64,
        ValType::V128 => wasmtime_valtype_t::V128,
        ValType::Ref(r) => wasmtime_valtype_t::Ref(wasmtime_reftype_t::from(r)),
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_valtype_clone(
    ty: &wasmtime_valtype_t,
    ret: &mut MaybeUninit<wasmtime_valtype_t>,
) {
    ret.write(ty.clone());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_valtype_delete(
    ret: Option<&mut ManuallyDrop<wasmtime_valtype_t>>,
) {
    if let Some(ret) = ret {
        unsafe {
            ManuallyDrop::drop(ret);
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_valtype_to_wasm(
    engine: Option<&wasm_engine_t>,
    r: &wasmtime_valtype_t,
) -> Box<wasm_valtype_t> {
    let r = match r {
        wasmtime_valtype_t::I32 => return Box::new(ValType::I32),
        wasmtime_valtype_t::I64 => return Box::new(ValType::I64),
        wasmtime_valtype_t::F32 => return Box::new(ValType::F32),
        wasmtime_valtype_t::F64 => return Box::new(ValType::F64),
        wasmtime_valtype_t::V128 => return Box::new(ValType::V128),
        wasmtime_valtype_t::Ref(r) => r,
    };
    let heap_type = match &r.hty {
        wasmtime_heaptype_t::Extern => HeapType::Extern,
        wasmtime_heaptype_t::NoExtern => HeapType::NoExtern,
        wasmtime_heaptype_t::Func => HeapType::Func,
        wasmtime_heaptype_t::ConcreteFunc(f) => {
            HeapType::ConcreteFunc(f.ty().ty(&engine.unwrap().engine).clone())
        }
        wasmtime_heaptype_t::NoFunc => HeapType::NoFunc,
        wasmtime_heaptype_t::Any => HeapType::Any,
        wasmtime_heaptype_t::None => HeapType::None,
        wasmtime_heaptype_t::Eq => HeapType::Eq,
        wasmtime_heaptype_t::I31 => HeapType::I31,
        wasmtime_heaptype_t::Array => HeapType::Array,
        wasmtime_heaptype_t::ConcreteArray(a) => HeapType::ConcreteArray(a.ty.clone()),
        wasmtime_heaptype_t::Struct => HeapType::Struct,
        wasmtime_heaptype_t::ConcreteStruct(s) => HeapType::ConcreteStruct(s.ty.clone()),
        wasmtime_heaptype_t::Exn => HeapType::Exn,
        wasmtime_heaptype_t::ConcreteExn(e) => HeapType::ConcreteExn(e.ty.clone()),
        wasmtime_heaptype_t::NoExn => HeapType::NoExn,
    };
    Box::new(ValType::Ref(RefType::new(r.nullable, heap_type)))
}

#[repr(C, u8)]
#[derive(Clone)]
pub enum wasmtime_heaptype_t {
    Extern,
    NoExtern,
    Func,
    ConcreteFunc(Box<wasm_functype_t>),
    NoFunc,
    Any,
    None,
    Eq,
    I31,
    Array,
    ConcreteArray(Box<wasmtime_array_type_t>),
    Struct,
    ConcreteStruct(Box<wasmtime_struct_type_t>),
    Exn,
    ConcreteExn(Box<wasmtime_exn_type_t>),
    NoExn,
}

impl From<&HeapType> for wasmtime_heaptype_t {
    fn from(r: &HeapType) -> Self {
        match r {
            HeapType::Extern => Self::Extern,
            HeapType::NoExtern => Self::NoExtern,
            HeapType::Func => Self::Func,
            HeapType::ConcreteFunc(f) => {
                Self::ConcreteFunc(Box::new(wasm_functype_t::new(f.clone())))
            }
            HeapType::NoFunc => Self::NoFunc,
            HeapType::Any => Self::Any,
            HeapType::None => Self::None,
            HeapType::Eq => Self::Eq,
            HeapType::I31 => Self::I31,
            HeapType::Array => Self::Array,
            HeapType::ConcreteArray(a) => {
                Self::ConcreteArray(Box::new(wasmtime_array_type_t { ty: a.clone() }))
            }
            HeapType::Struct => Self::Struct,
            HeapType::ConcreteStruct(s) => {
                Self::ConcreteStruct(Box::new(wasmtime_struct_type_t { ty: s.clone() }))
            }
            HeapType::Exn => Self::Exn,
            HeapType::ConcreteExn(e) => {
                Self::ConcreteExn(Box::new(wasmtime_exn_type_t { ty: e.clone() }))
            }
            HeapType::NoExn => Self::NoExn,
            HeapType::Cont | HeapType::ConcreteCont(_) | HeapType::NoCont => {
                crate::abort("missing support for contref")
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_heaptype_clone(
    ty: &wasmtime_heaptype_t,
    ret: &mut MaybeUninit<wasmtime_heaptype_t>,
) {
    ret.write(ty.clone());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_heaptype_delete(
    ret: Option<&mut ManuallyDrop<wasmtime_heaptype_t>>,
) {
    if let Some(ret) = ret {
        unsafe {
            ManuallyDrop::drop(ret);
        }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct wasmtime_reftype_t {
    pub nullable: bool,
    pub hty: wasmtime_heaptype_t,
}

impl From<&RefType> for wasmtime_reftype_t {
    fn from(r: &RefType) -> Self {
        Self {
            nullable: r.is_nullable(),
            hty: r.heap_type().into(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_reftype_clone(
    ty: &wasmtime_reftype_t,
    ret: &mut MaybeUninit<wasmtime_reftype_t>,
) {
    ret.write(ty.clone());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_reftype_delete(
    ret: Option<&mut ManuallyDrop<wasmtime_reftype_t>>,
) {
    if let Some(ret) = ret {
        unsafe {
            ManuallyDrop::drop(ret);
        }
    }
}
