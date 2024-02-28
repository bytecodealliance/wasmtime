//! The dummy `VMExternRef` for when the `gc` cargo feature is disabled.
//!
//! To reduce `#[cfg(...)]`s, this provides all the same methods as the real
//! `VMExternRef` except for constructors.

#![allow(missing_docs)]

use crate::{GcRuntime, ModuleInfoLookup, VMRuntimeLimits};
use std::any::Any;
use std::cmp;
use std::hash::Hasher;
use std::ops::Deref;

pub fn default_gc_runtime() -> impl GcRuntime {
    DisabledCollector
}

struct DisabledCollector;

unsafe impl GcRuntime for DisabledCollector {
    fn new_gc_heap(&self) -> Box<dyn GcHeap> {
        unreachable!()
    }
}
