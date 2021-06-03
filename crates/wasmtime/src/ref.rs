#![allow(missing_docs)]

use std::any::Any;
use wasmtime_runtime::VMExternRef;

/// Represents an opaque reference to any data within WebAssembly.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct ExternRef {
    pub(crate) inner: VMExternRef,
}

impl ExternRef {
    /// Creates a new instance of `ExternRef` wrapping the given value.
    pub fn new<T>(value: T) -> ExternRef
    where
        T: 'static + Any + Send + Sync,
    {
        let inner = VMExternRef::new(value);
        ExternRef { inner }
    }

    /// Get the underlying data for this `ExternRef`.
    pub fn data(&self) -> &dyn Any {
        &*self.inner
    }

    /// Get the strong reference count for this `ExternRef`.
    ///
    /// Note that this loads the reference count with a `SeqCst` ordering to
    /// synchronize with other threads.
    pub fn strong_count(&self) -> usize {
        self.inner.strong_count()
    }

    /// Does this `ExternRef` point to the same inner value as `other`?
    ///
    /// This is *only* pointer equality, and does *not* run any inner value's
    /// `Eq` implementation.
    pub fn ptr_eq(&self, other: &ExternRef) -> bool {
        VMExternRef::eq(&self.inner, &other.inner)
    }
}
