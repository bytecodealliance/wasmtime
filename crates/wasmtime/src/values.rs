use crate::r#ref::ExternRef;
use crate::store::StoreOpaque;
use crate::{Func, ValType};
use anyhow::{bail, Result};
use std::ptr;
use wasmtime_runtime::{self as runtime, VMExternRef};

/// Possible runtime values that a WebAssembly module can either consume or
/// produce.
#[derive(Debug, Clone)]
pub enum Val {
    // NB: the ordering here is intended to match the ordering in
    // `ValType` to improve codegen when learning the type of a value.
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

    /// A 128-bit number
    V128(u128),

    /// A first-class reference to a WebAssembly function.
    ///
    /// `FuncRef(None)` is the null function reference, created by `ref.null
    /// func` in Wasm.
    FuncRef(Option<Func>),

    /// An `externref` value which can hold opaque data to the Wasm instance
    /// itself.
    ///
    /// `ExternRef(None)` is the null external reference, created by `ref.null
    /// extern` in Wasm.
    ExternRef(Option<ExternRef>),
}

macro_rules! accessors {
    ($bind:ident $(($variant:ident($ty:ty) $get:ident $unwrap:ident $cvt:expr))*) => ($(
        /// Attempt to access the underlying value of this `Val`, returning
        /// `None` if it is not the correct type.
        #[inline]
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
        #[inline]
        pub fn $unwrap(&self) -> $ty {
            self.$get().expect(concat!("expected ", stringify!($ty)))
        }
    )*)
}

impl Val {
    /// Returns a null `externref` value.
    #[inline]
    pub fn null() -> Val {
        Val::ExternRef(None)
    }

    /// Returns the corresponding [`ValType`] for this `Val`.
    #[inline]
    pub fn ty(&self) -> ValType {
        match self {
            Val::I32(_) => ValType::I32,
            Val::I64(_) => ValType::I64,
            Val::F32(_) => ValType::F32,
            Val::F64(_) => ValType::F64,
            Val::ExternRef(_) => ValType::ExternRef,
            Val::FuncRef(_) => ValType::FuncRef,
            Val::V128(_) => ValType::V128,
        }
    }

    pub(crate) unsafe fn write_value_to(self, store: &mut StoreOpaque, p: *mut u128) {
        match self {
            Val::I32(i) => ptr::write(p as *mut i32, i),
            Val::I64(i) => ptr::write(p as *mut i64, i),
            Val::F32(u) => ptr::write(p as *mut u32, u),
            Val::F64(u) => ptr::write(p as *mut u64, u),
            Val::V128(b) => ptr::write(p as *mut u128, b),
            Val::ExternRef(None) => ptr::write(p, 0),
            Val::ExternRef(Some(x)) => {
                let externref_ptr = x.inner.as_raw();
                store.insert_vmexternref(x.inner);
                ptr::write(p as *mut *mut u8, externref_ptr)
            }
            Val::FuncRef(f) => ptr::write(
                p as *mut *mut runtime::VMCallerCheckedAnyfunc,
                if let Some(f) = f {
                    f.caller_checked_anyfunc(store).as_ptr()
                } else {
                    ptr::null_mut()
                },
            ),
        }
    }

    pub(crate) unsafe fn read_value_from(
        store: &mut StoreOpaque,
        p: *const u128,
        ty: ValType,
    ) -> Val {
        match ty {
            ValType::I32 => Val::I32(ptr::read(p as *const i32)),
            ValType::I64 => Val::I64(ptr::read(p as *const i64)),
            ValType::F32 => Val::F32(ptr::read(p as *const u32)),
            ValType::F64 => Val::F64(ptr::read(p as *const u64)),
            ValType::V128 => Val::V128(ptr::read(p as *const u128)),
            ValType::ExternRef => {
                let raw = ptr::read(p as *const *mut u8);
                if raw.is_null() {
                    Val::ExternRef(None)
                } else {
                    Val::ExternRef(Some(ExternRef {
                        inner: VMExternRef::clone_from_raw(raw),
                    }))
                }
            }
            ValType::FuncRef => {
                let func = ptr::read(p as *const *mut runtime::VMCallerCheckedAnyfunc);
                from_checked_anyfunc(func, store)
            }
        }
    }

    accessors! {
        e
        (I32(i32) i32 unwrap_i32 *e)
        (I64(i64) i64 unwrap_i64 *e)
        (F32(f32) f32 unwrap_f32 f32::from_bits(*e))
        (F64(f64) f64 unwrap_f64 f64::from_bits(*e))
        (FuncRef(Option<&Func>) funcref unwrap_funcref e.as_ref())
        (V128(u128) v128 unwrap_v128 *e)
    }

    /// Attempt to access the underlying `externref` value of this `Val`.
    ///
    /// If this is not an `externref`, then `None` is returned.
    ///
    /// If this is a null `externref`, then `Some(None)` is returned.
    ///
    /// If this is a non-null `externref`, then `Some(Some(..))` is returned.
    #[inline]
    pub fn externref(&self) -> Option<Option<ExternRef>> {
        match self {
            Val::ExternRef(e) => Some(e.clone()),
            _ => None,
        }
    }

    /// Returns the underlying `externref` value of this `Val`, panicking if it's the
    /// wrong type.
    ///
    /// If this is a null `externref`, then `None` is returned.
    ///
    /// If this is a non-null `externref`, then `Some(..)` is returned.
    ///
    /// # Panics
    ///
    /// Panics if `self` is not a (nullable) `externref`.
    #[inline]
    pub fn unwrap_externref(&self) -> Option<ExternRef> {
        self.externref().expect("expected externref")
    }

    pub(crate) fn into_table_element(
        self,
        store: &mut StoreOpaque,
        ty: ValType,
    ) -> Result<runtime::TableElement> {
        match (self, ty) {
            (Val::FuncRef(Some(f)), ValType::FuncRef) => {
                if !f.comes_from_same_store(store) {
                    bail!("cross-`Store` values are not supported in tables");
                }
                Ok(runtime::TableElement::FuncRef(
                    f.caller_checked_anyfunc(store).as_ptr(),
                ))
            }
            (Val::FuncRef(None), ValType::FuncRef) => {
                Ok(runtime::TableElement::FuncRef(ptr::null_mut()))
            }
            (Val::ExternRef(Some(x)), ValType::ExternRef) => {
                Ok(runtime::TableElement::ExternRef(Some(x.inner)))
            }
            (Val::ExternRef(None), ValType::ExternRef) => {
                Ok(runtime::TableElement::ExternRef(None))
            }
            _ => bail!("value does not match table element type"),
        }
    }

    #[inline]
    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        match self {
            Val::FuncRef(Some(f)) => f.comes_from_same_store(store),
            Val::FuncRef(None) => true,

            // Integers, floats, vectors, and `externref`s have no association
            // with any particular store, so they're always considered as "yes I
            // came from that store",
            Val::I32(_)
            | Val::I64(_)
            | Val::F32(_)
            | Val::F64(_)
            | Val::V128(_)
            | Val::ExternRef(_) => true,
        }
    }
}

impl From<i32> for Val {
    #[inline]
    fn from(val: i32) -> Val {
        Val::I32(val)
    }
}

impl From<i64> for Val {
    #[inline]
    fn from(val: i64) -> Val {
        Val::I64(val)
    }
}

impl From<f32> for Val {
    #[inline]
    fn from(val: f32) -> Val {
        Val::F32(val.to_bits())
    }
}

impl From<f64> for Val {
    #[inline]
    fn from(val: f64) -> Val {
        Val::F64(val.to_bits())
    }
}

impl From<ExternRef> for Val {
    #[inline]
    fn from(val: ExternRef) -> Val {
        Val::ExternRef(Some(val))
    }
}

impl From<Option<ExternRef>> for Val {
    #[inline]
    fn from(val: Option<ExternRef>) -> Val {
        Val::ExternRef(val)
    }
}

impl From<Option<Func>> for Val {
    #[inline]
    fn from(val: Option<Func>) -> Val {
        Val::FuncRef(val)
    }
}

impl From<Func> for Val {
    #[inline]
    fn from(val: Func) -> Val {
        Val::FuncRef(Some(val))
    }
}

impl From<u128> for Val {
    #[inline]
    fn from(val: u128) -> Val {
        Val::V128(val)
    }
}

pub(crate) fn into_checked_anyfunc(
    val: Val,
    store: &mut StoreOpaque,
) -> Result<*mut wasmtime_runtime::VMCallerCheckedAnyfunc> {
    if !val.comes_from_same_store(store) {
        bail!("cross-`Store` values are not supported");
    }
    Ok(match val {
        Val::FuncRef(None) => ptr::null_mut(),
        Val::FuncRef(Some(f)) => f.caller_checked_anyfunc(store).as_ptr(),
        _ => bail!("val is not funcref"),
    })
}

pub(crate) unsafe fn from_checked_anyfunc(
    anyfunc: *mut wasmtime_runtime::VMCallerCheckedAnyfunc,
    store: &mut StoreOpaque,
) -> Val {
    Val::FuncRef(Func::from_caller_checked_anyfunc(store, anyfunc))
}
