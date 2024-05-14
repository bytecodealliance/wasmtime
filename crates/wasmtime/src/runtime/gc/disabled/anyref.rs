use crate::runtime::vm::VMGcRef;
use crate::{
    store::{AutoAssertNoGc, StoreOpaque},
    AsContext, AsContextMut, GcRefImpl, HeapType, Result, Rooted, StructRef, I31,
};

/// Support for `anyref` disabled at compile time because the `gc` cargo feature
/// was not enabled.
pub enum AnyRef {}

impl GcRefImpl for AnyRef {}

#[allow(missing_docs)]
impl AnyRef {
    pub(crate) fn from_cloned_gc_ref(
        _store: &mut AutoAssertNoGc<'_>,
        _gc_ref: VMGcRef,
    ) -> Rooted<Self> {
        unreachable!()
    }

    pub unsafe fn from_raw(_store: impl AsContextMut, raw: u32) -> Option<Rooted<Self>> {
        assert_eq!(raw, 0);
        None
    }

    pub unsafe fn to_raw(&self, _store: impl AsContextMut) -> Result<u32> {
        match *self {}
    }

    pub fn ty(&self, _store: impl AsContext) -> Result<HeapType> {
        match *self {}
    }

    pub(crate) fn _ty(&self, _store: &StoreOpaque) -> Result<HeapType> {
        match *self {}
    }

    pub fn matches_ty(&self, _store: impl AsContext, _ty: &HeapType) -> Result<bool> {
        match *self {}
    }

    pub fn is_i31(&self, _store: impl AsContext) -> Result<bool> {
        match *self {}
    }

    pub(crate) fn _is_i31(&self, _store: &StoreOpaque) -> Result<bool> {
        match *self {}
    }

    pub fn as_i31(&self, _store: impl AsContext) -> Result<Option<I31>> {
        match *self {}
    }

    pub fn unwrap_i31(&self, _store: impl AsContext) -> Result<I31> {
        match *self {}
    }

    pub fn is_struct(&self, _store: impl AsContext) -> Result<bool> {
        match *self {}
    }

    pub(crate) fn _is_struct(&self, _store: &StoreOpaque) -> Result<bool> {
        match *self {}
    }

    pub fn as_struct(&self, _store: impl AsContext) -> Result<Option<StructRef>> {
        match *self {}
    }

    pub(crate) fn _as_struct(&self, _store: &StoreOpaque) -> Result<Option<StructRef>> {
        match *self {}
    }

    pub fn unwrap_struct(&self, _store: impl AsContext) -> Result<StructRef> {
        match *self {}
    }
}
