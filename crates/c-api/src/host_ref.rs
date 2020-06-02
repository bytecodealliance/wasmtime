use std::any::Any;
use std::cell::{self, RefCell};
use std::convert::TryFrom;
use std::marker::PhantomData;
use wasmtime::{ExternRef, Store};

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
    pub fn new(store: &Store, item: T) -> HostRef<T> {
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
            .data()
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
        if externref.data().is::<RefCell<T>>() {
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
