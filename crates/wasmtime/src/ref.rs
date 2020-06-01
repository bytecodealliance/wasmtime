#![allow(missing_docs)]

use std::any::Any;
use std::cell::RefCell;
use std::fmt;
use std::rc::{Rc, Weak};
use wasmtime_runtime::VMExternRef;

/// Represents an opaque reference to any data within WebAssembly.
#[derive(Clone)]
pub struct ExternRef {
    pub(crate) inner: VMExternRef,
    pub(crate) store: Weak<crate::runtime::StoreInner>,
}

impl ExternRef {
    /// Creates a new instance of `ExternRef` wrapping the given value.
    pub fn new<T>(store: &crate::Store, value: T) -> ExternRef
    where
        T: 'static + Any,
    {
        let inner = VMExternRef::new(value);
        let store = store.weak();
        ExternRef { inner, store }
    }

    /// Get this reference's store.
    ///
    /// Returns `None` if this reference outlived its store.
    pub fn store(&self) -> Option<crate::runtime::Store> {
        crate::runtime::Store::upgrade(&self.store)
    }

    /// Get the underlying data for this `ExternRef`.
    pub fn data(&self) -> &dyn Any {
        &*self.inner
    }

    /// Does this `ExternRef` point to the same inner value as `other`?0
    ///
    /// This is *only* pointer equality, and does *not* run any inner value's
    /// `Eq` implementation.
    pub fn ptr_eq(&self, other: &ExternRef) -> bool {
        VMExternRef::eq(&self.inner, &other.inner)
    }

    /// Returns the host information for this `externref`, if previously created
    /// with `set_host_info`.
    pub fn host_info(&self) -> Option<Rc<RefCell<dyn Any>>> {
        let store = crate::Store::upgrade(&self.store)?;
        store.host_info(self)
    }

    /// Set the host information for this `externref`, returning the old host
    /// information if it was previously set.
    pub fn set_host_info<T>(&self, info: T) -> Option<Rc<RefCell<dyn Any>>>
    where
        T: 'static + Any,
    {
        let store = crate::Store::upgrade(&self.store)?;
        store.set_host_info(self, Some(Rc::new(RefCell::new(info))))
    }

    /// Remove the host information for this `externref`, returning the old host
    /// information if it was previously set.
    pub fn remove_host_info(&self) -> Option<Rc<RefCell<dyn Any>>> {
        let store = crate::Store::upgrade(&self.store)?;
        store.set_host_info(self, None)
    }
}

impl fmt::Debug for ExternRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ExternRef { inner, store: _ } = self;
        let store = self.store();
        f.debug_struct("ExternRef")
            .field("inner", &inner)
            .field("store", &store)
            .finish()
    }
}
