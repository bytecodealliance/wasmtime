#![allow(missing_docs)]

use std::any::Any;
use std::cell::{self, RefCell};
use std::convert::TryFrom;
use std::fmt;
use std::marker::PhantomData;
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
        f.debug_struct("ExternRef")
            .field("inner", &inner)
            .field("store", &"..")
            .finish()
    }
}

/// Represents a piece of data located in the host environment.
#[derive(Debug)]
pub struct HostRef<T>
where
    T: 'static + Any,
{
    externref: ExternRef,
    _phantom: PhantomData<T>,
}

impl<T> HostRef<T>
where
    T: 'static + Any,
{
    /// Creates a new `HostRef<T>` from `T`.
    pub fn new(store: &crate::Store, item: T) -> HostRef<T> {
        HostRef {
            externref: ExternRef::new(store, RefCell::new(item)),
            _phantom: PhantomData,
        }
    }

    /// Immutably borrows the wrapped data.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently mutably borrowed.
    pub fn borrow(&self) -> cell::Ref<T> {
        self.inner().borrow()
    }

    /// Mutably borrows the wrapped data.
    ///
    /// # Panics
    ///
    /// Panics if the `HostRef<T>` is already borrowed.
    pub fn borrow_mut(&self) -> cell::RefMut<T> {
        self.inner().borrow_mut()
    }

    /// Returns true if the two `HostRef<T>`'s point to the same value (not just
    /// values that compare as equal).
    pub fn ptr_eq(&self, other: &HostRef<T>) -> bool {
        self.externref.ptr_eq(&other.externref)
    }

    fn inner(&self) -> &RefCell<T> {
        self.externref
            .inner
            .downcast_ref::<RefCell<T>>()
            .expect("`HostRef<T>`s always wrap an `ExternRef` of `RefCell<T>`")
    }
}

impl<T> AsRef<ExternRef> for HostRef<T> {
    fn as_ref(&self) -> &ExternRef {
        &self.externref
    }
}

impl<T> From<HostRef<T>> for ExternRef
where
    T: 'static + Any,
{
    fn from(host: HostRef<T>) -> ExternRef {
        host.externref
    }
}

impl<T> TryFrom<ExternRef> for HostRef<T>
where
    T: 'static + Any,
{
    type Error = ExternRef;

    fn try_from(externref: ExternRef) -> Result<Self, ExternRef> {
        if externref.inner.is::<RefCell<T>>() {
            Ok(HostRef {
                externref,
                _phantom: PhantomData,
            })
        } else {
            Err(externref)
        }
    }
}

impl<T> Clone for HostRef<T> {
    fn clone(&self) -> HostRef<T> {
        HostRef {
            externref: self.externref.clone(),
            _phantom: PhantomData,
        }
    }
}
