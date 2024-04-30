use crate::runtime::vm::VMGcRef;
use crate::{
    store::AutoAssertNoGc, AsContextMut, GcRefImpl, Result, Rooted, StoreContext, StoreContextMut,
};
use std::any::Any;

/// Support for `externref` disabled at compile time because the `gc` cargo
/// feature was not enabled.
pub enum ExternRef {}

impl GcRefImpl for ExternRef {}

#[allow(missing_docs)]
impl ExternRef {
    pub(crate) fn from_cloned_gc_ref(
        _store: &mut AutoAssertNoGc<'_>,
        _gc_ref: VMGcRef,
    ) -> Rooted<Self> {
        unreachable!()
    }

    pub fn data<'a, T>(
        &self,
        _store: impl Into<StoreContext<'a, T>>,
    ) -> Result<&'a (dyn Any + Send + Sync)>
    where
        T: 'a,
    {
        match *self {}
    }

    pub fn data_mut<'a, T>(
        &self,
        _store: impl Into<StoreContextMut<'a, T>>,
    ) -> Result<&'a mut (dyn Any + Send + Sync)>
    where
        T: 'a,
    {
        match *self {}
    }

    pub unsafe fn from_raw(_store: impl AsContextMut, raw: u32) -> Option<Rooted<Self>> {
        assert_eq!(raw, 0);
        None
    }

    pub unsafe fn to_raw(&self, _store: impl AsContextMut) -> Result<u32> {
        match *self {}
    }
}
