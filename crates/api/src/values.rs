use crate::externals::Func;
use crate::r#ref::{AnyRef, HostRef};
use crate::runtime::Store;
use crate::types::ValType;
use core::ptr;
use cranelift_codegen::ir;
use wasmtime_jit::RuntimeValue;

#[derive(Debug, Clone)]
pub enum Val {
    I32(i32),
    I64(i64),
    F32(u32),
    F64(u64),
    AnyRef(AnyRef),
    FuncRef(HostRef<Func>),
}

impl Val {
    pub fn default() -> Val {
        Val::AnyRef(AnyRef::null())
    }

    pub fn r#type(&self) -> ValType {
        match self {
            Val::I32(_) => ValType::I32,
            Val::I64(_) => ValType::I64,
            Val::F32(_) => ValType::F32,
            Val::F64(_) => ValType::F64,
            Val::AnyRef(_) => ValType::AnyRef,
            Val::FuncRef(_) => ValType::FuncRef,
        }
    }

    pub(crate) unsafe fn write_value_to(&self, p: *mut i64) {
        match self {
            Val::I32(i) => ptr::write(p as *mut i32, *i),
            Val::I64(i) => ptr::write(p as *mut i64, *i),
            Val::F32(u) => ptr::write(p as *mut u32, *u),
            Val::F64(u) => ptr::write(p as *mut u64, *u),
            _ => unimplemented!("Val::write_value_to"),
        }
    }

    pub(crate) unsafe fn read_value_from(p: *const i64, ty: ir::Type) -> Val {
        match ty {
            ir::types::I32 => Val::I32(ptr::read(p as *const i32)),
            ir::types::I64 => Val::I64(ptr::read(p as *const i64)),
            ir::types::F32 => Val::F32(ptr::read(p as *const u32)),
            ir::types::F64 => Val::F64(ptr::read(p as *const u64)),
            _ => unimplemented!("Val::read_value_from"),
        }
    }

    pub fn from_f32_bits(v: u32) -> Val {
        Val::F32(v)
    }

    pub fn from_f64_bits(v: u64) -> Val {
        Val::F64(v)
    }

    pub fn i32(&self) -> i32 {
        if let Val::I32(i) = self {
            *i
        } else {
            panic!("Invalid conversion of {:?} to i32.", self);
        }
    }

    pub fn i64(&self) -> i64 {
        if let Val::I64(i) = self {
            *i
        } else {
            panic!("Invalid conversion of {:?} to i64.", self);
        }
    }

    pub fn f32(&self) -> f32 {
        RuntimeValue::F32(self.f32_bits()).unwrap_f32()
    }

    pub fn f64(&self) -> f64 {
        RuntimeValue::F64(self.f64_bits()).unwrap_f64()
    }

    pub fn f32_bits(&self) -> u32 {
        if let Val::F32(i) = self {
            *i
        } else {
            panic!("Invalid conversion of {:?} to f32.", self);
        }
    }

    pub fn f64_bits(&self) -> u64 {
        if let Val::F64(i) = self {
            *i
        } else {
            panic!("Invalid conversion of {:?} to f64.", self);
        }
    }
}

impl From<i32> for Val {
    fn from(val: i32) -> Val {
        Val::I32(val)
    }
}

impl From<i64> for Val {
    fn from(val: i64) -> Val {
        Val::I64(val)
    }
}

impl From<f32> for Val {
    fn from(val: f32) -> Val {
        Val::F32(val.to_bits())
    }
}

impl From<f64> for Val {
    fn from(val: f64) -> Val {
        Val::F64(val.to_bits())
    }
}

impl Into<i32> for Val {
    fn into(self) -> i32 {
        self.i32()
    }
}

impl Into<i64> for Val {
    fn into(self) -> i64 {
        self.i64()
    }
}

impl Into<f32> for Val {
    fn into(self) -> f32 {
        self.f32()
    }
}

impl Into<f64> for Val {
    fn into(self) -> f64 {
        self.f64()
    }
}

impl From<AnyRef> for Val {
    fn from(val: AnyRef) -> Val {
        match &val {
            AnyRef::Ref(r) => {
                if r.is_ref::<Func>() {
                    Val::FuncRef(r.get_ref())
                } else {
                    Val::AnyRef(val)
                }
            }
            _ => unimplemented!("AnyRef::Other"),
        }
    }
}

impl From<HostRef<Func>> for Val {
    fn from(val: HostRef<Func>) -> Val {
        Val::FuncRef(val)
    }
}

impl Into<AnyRef> for Val {
    fn into(self) -> AnyRef {
        match self {
            Val::AnyRef(r) => r,
            Val::FuncRef(f) => f.anyref(),
            _ => panic!("Invalid conversion of {:?} to anyref.", self),
        }
    }
}

impl From<RuntimeValue> for Val {
    fn from(rv: RuntimeValue) -> Self {
        match rv {
            RuntimeValue::I32(i) => Val::I32(i),
            RuntimeValue::I64(i) => Val::I64(i),
            RuntimeValue::F32(u) => Val::F32(u),
            RuntimeValue::F64(u) => Val::F64(u),
            x => {
                panic!("unsupported {:?}", x);
            }
        }
    }
}

impl Into<RuntimeValue> for Val {
    fn into(self) -> RuntimeValue {
        match self {
            Val::I32(i) => RuntimeValue::I32(i),
            Val::I64(i) => RuntimeValue::I64(i),
            Val::F32(u) => RuntimeValue::F32(u),
            Val::F64(u) => RuntimeValue::F64(u),
            x => {
                panic!("unsupported {:?}", x);
            }
        }
    }
}

pub(crate) fn into_checked_anyfunc(
    val: Val,
    store: &HostRef<Store>,
) -> wasmtime_runtime::VMCallerCheckedAnyfunc {
    match val {
        Val::AnyRef(AnyRef::Null) => wasmtime_runtime::VMCallerCheckedAnyfunc {
            func_ptr: ptr::null(),
            type_index: wasmtime_runtime::VMSharedSignatureIndex::default(),
            vmctx: ptr::null_mut(),
        },
        Val::FuncRef(f) => {
            let f = f.borrow();
            let (vmctx, func_ptr, signature) = match f.wasmtime_export() {
                wasmtime_runtime::Export::Function {
                    vmctx,
                    address,
                    signature,
                } => (*vmctx, *address, signature),
                _ => panic!("expected function export"),
            };
            let type_index = store.borrow_mut().register_cranelift_signature(signature);
            wasmtime_runtime::VMCallerCheckedAnyfunc {
                func_ptr,
                type_index,
                vmctx,
            }
        }
        _ => panic!("val is not funcref"),
    }
}

pub(crate) fn from_checked_anyfunc(
    item: &wasmtime_runtime::VMCallerCheckedAnyfunc,
    store: &HostRef<Store>,
) -> Val {
    if item.type_index == wasmtime_runtime::VMSharedSignatureIndex::default() {
        return Val::AnyRef(AnyRef::Null);
    }
    let signature = store
        .borrow()
        .lookup_cranelift_signature(item.type_index)
        .expect("signature")
        .clone();
    let instance_handle = unsafe { wasmtime_runtime::InstanceHandle::from_vmctx(item.vmctx) };
    let export = wasmtime_runtime::Export::Function {
        address: item.func_ptr,
        signature,
        vmctx: item.vmctx,
    };
    let f = Func::from_wasmtime_function(export, store, instance_handle);
    Val::FuncRef(HostRef::new(f))
}
