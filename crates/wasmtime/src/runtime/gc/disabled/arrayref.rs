use crate::runtime::vm::VMGcRef;
use crate::{
    store::{AutoAssertNoGc, StoreContextMut, StoreOpaque},
    ArrayType, AsContext, AsContextMut, GcRefImpl, Result, Rooted, Val, I31,
};

/// Support for `ArrayRefPre` disabled at compile time because the `gc` cargo
/// feature was not enabled.
pub enum ArrayRefPre {}

/// Support for `arrayref` disabled at compile time because the `gc` cargo feature
/// was not enabled.
pub enum ArrayRef {}

impl GcRefImpl for ArrayRef {}

impl ArrayRef {
    pub(crate) fn from_cloned_gc_ref(
        _store: &mut AutoAssertNoGc<'_>,
        _gc_ref: VMGcRef,
    ) -> Rooted<Self> {
        unreachable!()
    }

    pub fn ty(&self, _store: impl AsContext) -> Result<ArrayType> {
        match *self {}
    }

    pub fn matches_ty(&self, _store: impl AsContext, _ty: &ArrayType) -> Result<bool> {
        match *self {}
    }

    pub(crate) fn _matches_ty(&self, _store: &StoreOpaque, _ty: &ArrayType) -> Result<bool> {
        match *self {}
    }

    pub fn len(&self, _store: impl AsContext) -> Result<u32> {
        match *self {}
    }

    pub fn elems<'a, T: 'a>(
        &self,
        _store: impl Into<StoreContextMut<'a, T>>,
    ) -> Result<impl ExactSizeIterator<Item = Val> + 'a> {
        match *self {}
        Ok([].into_iter())
    }

    pub fn get(&self, _store: impl AsContextMut, _index: usize) -> Result<Val> {
        match *self {}
    }

    pub fn set(&self, _store: impl AsContextMut, _index: usize, _value: Val) -> Result<()> {
        match *self {}
    }
}
