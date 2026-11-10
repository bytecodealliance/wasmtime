use crate::prelude::*;
use crate::runtime::vm::VMGcRef;
use crate::store::{AutoAssertNoGc, StoreOpaque};
use crate::{
    AnyRef, ArrayRef, ArrayType, AsContext, AsContextMut, EqRef, ExnRef, ExnType, ExternRef, GcRef,
    HeapType, I31, OwnedRooted, Result, Rooted, StoreContext, StructRef, StructType, Tag, Val,
};
use core::any::Any;
use core::convert::Infallible;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker;

/// This type has been disabled because the `gc` cargo feature was not enabled
/// at compile time.
pub struct Unrooted<T: GcRef> {
    pub(crate) inner: Infallible,
    _phantom: marker::PhantomData<fn() -> T>,
}

impl<T: GcRef> Clone for Unrooted<T> {
    fn clone(&self) -> Self {
        match self.inner {}
    }
}

impl<T: GcRef> Copy for Unrooted<T> {}

impl<T: GcRef> fmt::Debug for Unrooted<T> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner {}
    }
}

impl<T: GcRef> PartialEq for Unrooted<T> {
    fn eq(&self, _other: &Self) -> bool {
        match self.inner {}
    }
}

impl<T: GcRef> Eq for Unrooted<T> {}

impl<T: GcRef> Hash for Unrooted<T> {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        match self.inner {}
    }
}

impl<T: GcRef> Unrooted<T> {
    pub(crate) fn new(_store: &StoreOpaque, _gc_ref: &VMGcRef) -> Self {
        unreachable!("GC references without the `gc` feature")
    }

    pub(crate) fn from_raw_u32(_store: &StoreOpaque, raw: u32) -> Option<Self> {
        assert_eq!(raw, 0);
        None
    }

    pub(crate) fn gc_ref<'a>(&'a self, _store: &'a StoreOpaque) -> Result<&'a VMGcRef> {
        match self.inner {}
    }

    pub(crate) fn comes_from_same_store(&self, _store: &StoreOpaque) -> bool {
        match self.inner {}
    }

    pub(crate) fn to_rooted_prevalidated(&self, _store: &mut AutoAssertNoGc<'_>) -> Rooted<T> {
        match self.inner {}
    }

    pub fn is_valid(&self, _store: impl AsContext) -> bool {
        match self.inner {}
    }

    pub fn to_rooted(&self, _store: impl AsContextMut) -> Result<Rooted<T>> {
        match self.inner {}
    }

    pub(crate) fn _to_rooted(&self, _store: &mut StoreOpaque) -> Result<Rooted<T>> {
        match self.inner {}
    }

    pub(crate) fn _to_owned_rooted(&self, _store: &mut StoreOpaque) -> Result<OwnedRooted<T>> {
        match self.inner {}
    }

    pub fn to_owned_rooted(&self, _store: impl AsContextMut) -> Result<OwnedRooted<T>> {
        match self.inner {}
    }
}

impl Unrooted<AnyRef> {
    pub(crate) fn _ty(&self, _store: &StoreOpaque) -> Result<HeapType> {
        match self.inner {}
    }

    pub fn ty(&self, _store: impl AsContext) -> Result<HeapType> {
        match self.inner {}
    }

    pub fn as_i31(&self, _store: impl AsContext) -> Result<Option<I31>> {
        match self.inner {}
    }

    pub fn as_struct(&self, _store: impl AsContext) -> Result<Option<Unrooted<StructRef>>> {
        match self.inner {}
    }

    pub fn as_array(&self, _store: impl AsContext) -> Result<Option<Unrooted<ArrayRef>>> {
        match self.inner {}
    }
}

impl From<Unrooted<EqRef>> for Unrooted<AnyRef> {
    fn from(r: Unrooted<EqRef>) -> Self {
        match r.inner {}
    }
}

impl From<Unrooted<StructRef>> for Unrooted<AnyRef> {
    fn from(r: Unrooted<StructRef>) -> Self {
        match r.inner {}
    }
}

impl From<Unrooted<ArrayRef>> for Unrooted<AnyRef> {
    fn from(r: Unrooted<ArrayRef>) -> Self {
        match r.inner {}
    }
}

impl Unrooted<StructRef> {
    pub fn ty(&self, _store: impl AsContext) -> Result<StructType> {
        match self.inner {}
    }

    pub fn field(&self, _store: impl AsContext, _index: usize) -> Result<Val> {
        match self.inner {}
    }

    pub fn fields(&self, _store: impl AsContext) -> Result<Vec<Val>> {
        match self.inner {}
    }
}

impl Unrooted<ArrayRef> {
    pub fn ty(&self, _store: impl AsContext) -> Result<ArrayType> {
        match self.inner {}
    }

    pub fn len(&self, _store: impl AsContext) -> Result<u32> {
        match self.inner {}
    }

    pub fn get(&self, _store: impl AsContext, _index: u32) -> Result<Val> {
        match self.inner {}
    }
}

impl Unrooted<ExnRef> {
    pub fn ty(&self, _store: impl AsContext) -> Result<ExnType> {
        match self.inner {}
    }

    pub(crate) fn _ty(&self, _store: &StoreOpaque) -> Result<ExnType> {
        match self.inner {}
    }

    pub fn tag(&self, _store: impl AsContext) -> Result<Tag> {
        match self.inner {}
    }

    pub fn field(&self, _store: impl AsContext, _index: usize) -> Result<Val> {
        match self.inner {}
    }

    pub fn fields(&self, _store: impl AsContext) -> Result<Vec<Val>> {
        match self.inner {}
    }
}

impl Unrooted<ExternRef> {
    pub fn data<'a, T: 'static>(
        &self,
        _store: impl Into<StoreContext<'a, T>>,
    ) -> Result<Option<&'a (dyn Any + Send + Sync)>> {
        match self.inner {}
    }
}
