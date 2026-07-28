//! `exnref` implementation stubs when GC is disabled.

use crate::{
    AsContextMut, GcRefImpl, Result, Rooted, Tag, Val,
    store::{AutoAssertNoGc, StoreContextMut},
    vm::VMGcRef,
};

/// Support for `ExnRefPre` disabled at compile time because the `gc`
/// cargo feature was not enabled.
pub enum ExnRefPre {}

/// Support for `exnref` disabled at compile time because the `gc`
/// cargo feature was not enabled.
pub enum ExnRef {}

impl GcRefImpl for ExnRef {}

impl ExnRef {
    pub(crate) fn from_cloned_gc_ref(
        _store: &mut AutoAssertNoGc<'_>,
        _gc_ref: VMGcRef,
    ) -> Rooted<Self> {
        unimplemented!()
    }

    pub fn from_raw(_store: impl AsContextMut, _raw: u32) -> Option<Rooted<Self>> {
        None
    }

    pub(crate) fn _from_raw(_store: &mut AutoAssertNoGc, _raw: u32) -> Option<Rooted<Self>> {
        None
    }

    pub fn to_raw(&self, _store: impl AsContextMut) -> Result<u32> {
        Ok(0)
    }

    pub(crate) fn _to_raw(&self, _store: &mut AutoAssertNoGc<'_>) -> Result<u32> {
        Ok(0)
    }

    pub fn tag(&self, _store: impl AsContextMut) -> Result<Tag> {
        match *self {}
    }

    pub fn fields<'a, T: 'static>(
        &self,
        _store: impl Into<StoreContextMut<'a, T>>,
    ) -> Result<impl ExactSizeIterator<Item = Val> + 'a + '_> {
        match *self {}
        Ok([].into_iter())
    }

    pub fn field(&self, _store: impl AsContextMut, _index: usize) -> Result<Val> {
        match *self {}
    }
}
