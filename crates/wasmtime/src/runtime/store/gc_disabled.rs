use crate::store::{StoreOpaque, StoreResourceLimiter};
use crate::vm::GcStore;
use crate::{Result, bail};

impl StoreOpaque {
    #[inline]
    pub(crate) async fn ensure_gc_store(
        &mut self,
        _limiter: Option<&mut StoreResourceLimiter<'_>>,
    ) -> Result<&mut GcStore> {
        bail!("cannot allocate a GC store: the `gc` feature was disabled at compile time")
    }

    pub(crate) fn has_pending_exception(&self) -> bool {
        false
    }

    pub(crate) fn require_gc_store_mut(&mut self) -> Result<&mut GcStore> {
        bail!("GC is disabled")
    }

    #[inline]
    pub(crate) fn enter_gc_lifo_scope(&self) -> usize {
        0
    }

    #[inline]
    pub(crate) fn exit_gc_lifo_scope(&mut self, _scope: usize) {}
}
