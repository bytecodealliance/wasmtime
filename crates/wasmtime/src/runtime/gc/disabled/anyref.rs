use crate::runtime::vm::VMGcRef;
use crate::{
    store::{AutoAssertNoGc, StoreOpaque},
    AsContext, AsContextMut, GcRefImpl, Result, Rooted, I31,
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
}
