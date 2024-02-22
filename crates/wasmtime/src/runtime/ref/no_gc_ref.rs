//! The dummy `ExternRef` type used when the `gc` cargo feature is disabled.
//!
//! Providing a dummy type means that downstream users need to do fewer
//! `#[cfg(...)]`s versus if this type or its methods simply didn't exist. The
//! only methods that are left missing are constructors.

#![allow(missing_docs)]

use crate::runtime::Uninhabited;
use crate::AsContextMut;
use std::any::Any;
use std::ffi::c_void;
use wasmtime_runtime::VMExternRef;

/// Represents an opaque reference to any data within WebAssembly.
///
/// Due to compilation configuration, this is an uninhabited type: enable the
/// `gc` cargo feature to properly use this type.
#[derive(Clone, Debug)]
pub struct ExternRef {
    pub(crate) _inner: Uninhabited,
}

impl ExternRef {
    pub(crate) fn from_vm_extern_ref(_inner: VMExternRef) -> Self {
        unreachable!()
    }

    pub(crate) fn into_vm_extern_ref(self) -> VMExternRef {
        match self._inner {}
    }

    pub fn data(&self) -> &dyn Any {
        match self._inner {}
    }

    pub fn strong_count(&self) -> usize {
        match self._inner {}
    }

    pub fn ptr_eq(&self, _other: &ExternRef) -> bool {
        match self._inner {}
    }

    pub unsafe fn from_raw(raw: *mut c_void) -> Option<ExternRef> {
        assert!(raw.is_null());
        None
    }

    pub unsafe fn to_raw(&self, mut store: impl AsContextMut) -> *mut c_void {
        let _ = &mut store;
        match self._inner {}
    }
}

impl std::fmt::Pointer for ExternRef {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self._inner {}
    }
}
