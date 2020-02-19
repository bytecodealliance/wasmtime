use crate::r#ref::AnyRef;
use crate::{Func, Store, ValType};
use anyhow::{bail, Result};
use std::ptr;
use wasmtime_environ::ir;

/// Possible runtime values that a WebAssembly module can either consume or
/// produce.
#[derive(Debug, Clone)]
pub enum Val {
    /// A 32-bit integer
    I32(i32),

    /// A 64-bit integer
    I64(i64),

    /// A 32-bit float.
    ///
    /// Note that the raw bits of the float are stored here, and you can use
    /// `f32::from_bits` to create an `f32` value.
    F32(u32),

    /// A 64-bit float.
    ///
    /// Note that the raw bits of the float are stored here, and you can use
    /// `f64::from_bits` to create an `f64` value.
    F64(u64),

    /// An `anyref` value which can hold opaque data to the wasm instance itself.
    ///
    /// Note that this is a nullable value as well.
    AnyRef(AnyRef),

    /// A first-class reference to a WebAssembly function.
    FuncRef(Func),

    /// A 128-bit number
    V128(u128),
}

macro_rules! accessors {
    ($bind:ident $(($variant:ident($ty:ty) $get:ident $unwrap:ident $cvt:expr))*) => ($(
        /// Attempt to access the underlying value of this `Val`, returning
        /// `None` if it is not the correct type.
        pub fn $get(&self) -> Option<$ty> {
            if let Val::$variant($bind) = self {
                Some($cvt)
            } else {
                None
            }
        }

        /// Returns the underlying value of this `Val`, panicking if it's the
        /// wrong type.
        ///
        /// # Panics
        ///
        /// Panics if `self` is not of the right type.
        pub fn $unwrap(&self) -> $ty {
            self.$get().expect(concat!("expected ", stringify!($ty)))
        }
    )*)
}

impl Val {
    /// Returns a null `anyref` value.
    pub fn null() -> Val {
        Val::AnyRef(AnyRef::null())
    }

    /// Returns the corresponding [`ValType`] for this `Val`.
    pub fn ty(&self) -> ValType {
        match self {
            Val::I32(_) => ValType::I32,
            Val::I64(_) => ValType::I64,
            Val::F32(_) => ValType::F32,
            Val::F64(_) => ValType::F64,
            Val::AnyRef(_) => ValType::AnyRef,
            Val::FuncRef(_) => ValType::FuncRef,
            Val::V128(_) => ValType::V128,
        }
    }

    pub(crate) unsafe fn write_value_to(&self, p: *mut i128) {
        match self {
            Val::I32(i) => ptr::write(p as *mut i32, *i),
            Val::I64(i) => ptr::write(p as *mut i64, *i),
            Val::F32(u) => ptr::write(p as *mut u32, *u),
            Val::F64(u) => ptr::write(p as *mut u64, *u),
            Val::V128(b) => ptr::write(p as *mut u128, *b),
            _ => unimplemented!("Val::write_value_to"),
        }
    }

    pub(crate) unsafe fn read_value_from(p: *const i128, ty: ir::Type) -> Val {
        match ty {
            ir::types::I32 => Val::I32(ptr::read(p as *const i32)),
            ir::types::I64 => Val::I64(ptr::read(p as *const i64)),
            ir::types::F32 => Val::F32(ptr::read(p as *const u32)),
            ir::types::F64 => Val::F64(ptr::read(p as *const u64)),
            ir::types::I8X16 => Val::V128(ptr::read(p as *const u128)),
            _ => unimplemented!("Val::read_value_from"),
        }
    }

    accessors! {
        e
        (I32(i32) i32 unwrap_i32 *e)
        (I64(i64) i64 unwrap_i64 *e)
        (F32(f32) f32 unwrap_f32 f32::from_bits(*e))
        (F64(f64) f64 unwrap_f64 f64::from_bits(*e))
        (FuncRef(&Func) funcref unwrap_funcref e)
        (V128(u128) v128 unwrap_v128 *e)
    }

    /// Attempt to access the underlying value of this `Val`, returning
    /// `None` if it is not the correct type.
    ///
    /// This will return `Some` for both the `AnyRef` and `FuncRef` types.
    pub fn anyref(&self) -> Option<AnyRef> {
        match self {
            Val::AnyRef(e) => Some(e.clone()),
            _ => None,
        }
    }

    /// Returns the underlying value of this `Val`, panicking if it's the
    /// wrong type.
    ///
    /// # Panics
    ///
    /// Panics if `self` is not of the right type.
    pub fn unwrap_anyref(&self) -> AnyRef {
        self.anyref().expect("expected anyref")
    }

    pub(crate) fn comes_from_same_store(&self, store: &Store) -> bool {
        match self {
            Val::FuncRef(f) => Store::same(store, f.store()),

            // TODO: need to implement this once we actually finalize what
            // `anyref` will look like and it's actually implemented to pass it
            // to compiled wasm as well.
            Val::AnyRef(AnyRef::Ref(_)) | Val::AnyRef(AnyRef::Other(_)) => false,
            Val::AnyRef(AnyRef::Null) => true,

            // Integers have no association with any particular store, so
            // they're always considered as "yes I came from that store",
            Val::I32(_) | Val::I64(_) | Val::F32(_) | Val::F64(_) | Val::V128(_) => true,
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

impl From<AnyRef> for Val {
    fn from(val: AnyRef) -> Val {
        Val::AnyRef(val)
    }
}

impl From<Func> for Val {
    fn from(val: Func) -> Val {
        Val::FuncRef(val)
    }
}

pub(crate) fn into_checked_anyfunc(
    val: Val,
    store: &Store,
) -> Result<wasmtime_runtime::VMCallerCheckedAnyfunc> {
    if !val.comes_from_same_store(store) {
        bail!("cross-`Store` values are not supported");
    }
    Ok(match val {
        Val::AnyRef(AnyRef::Null) => wasmtime_runtime::VMCallerCheckedAnyfunc {
            func_ptr: ptr::null(),
            type_index: wasmtime_runtime::VMSharedSignatureIndex::default(),
            vmctx: ptr::null_mut(),
        },
        Val::FuncRef(f) => {
            let f = f.wasmtime_function();
            wasmtime_runtime::VMCallerCheckedAnyfunc {
                func_ptr: f.address,
                type_index: f.signature,
                vmctx: f.vmctx,
            }
        }
        _ => bail!("val is not funcref"),
    })
}

pub(crate) fn from_checked_anyfunc(
    item: wasmtime_runtime::VMCallerCheckedAnyfunc,
    store: &Store,
) -> Val {
    if item.type_index == wasmtime_runtime::VMSharedSignatureIndex::default() {
        Val::AnyRef(AnyRef::Null);
    }
    let instance_handle = unsafe { wasmtime_runtime::InstanceHandle::from_vmctx(item.vmctx) };
    let export = wasmtime_runtime::ExportFunction {
        address: item.func_ptr,
        signature: item.type_index,
        vmctx: item.vmctx,
    };
    let f = Func::from_wasmtime_function(export, store, instance_handle);
    Val::FuncRef(f)
}
