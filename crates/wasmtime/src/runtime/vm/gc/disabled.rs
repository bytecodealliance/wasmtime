//! Dummy GC types for when the `gc` cargo feature is disabled.
//!
//! To reduce `#[cfg(...)]`s, this provides all the same methods as the real
//! `VMExternRef` except for constructors.

#![allow(missing_docs)]

use crate::prelude::*;
use crate::runtime::vm::{GcHeap, GcRuntime};
use anyhow::Result;

pub fn default_gc_runtime() -> impl GcRuntime {
    DisabledCollector
}

struct DisabledCollector;

unsafe impl GcRuntime for DisabledCollector {
    fn new_gc_heap(&self) -> Result<Box<dyn GcHeap>> {
        unreachable!()
    }
}

pub enum VMExternRef {}
