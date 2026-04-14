//! The copying collector.
//!
//! This is a skeleton implementation that is not yet functional. All methods
//! bail with "not yet implemented" errors.

use crate::{Engine, prelude::*, vm::GcRuntime};
use wasmtime_environ::{GcTypeLayouts, copying::CopyingTypeLayouts};

/// The copying collector.
#[derive(Default)]
pub struct CopyingCollector {
    layouts: CopyingTypeLayouts,
}

unsafe impl GcRuntime for CopyingCollector {
    fn layouts(&self) -> &dyn GcTypeLayouts {
        &self.layouts
    }

    fn new_gc_heap(&self, _: &Engine) -> Result<Box<dyn crate::vm::GcHeap>> {
        bail!("copying collector is not yet implemented")
    }
}
