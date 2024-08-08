//! Dummy GC types for when the `gc` cargo feature is disabled.
//!
//! To reduce `#[cfg(...)]`s, this provides all the same methods as the real
//! `VMExternRef` except for constructors.

#![allow(missing_docs)]

use crate::prelude::*;
use crate::runtime::vm::{GcArrayLayout, GcHeap, GcRuntime, GcStructLayout};
use wasmtime_environ::{WasmArrayType, WasmStructType};

pub fn default_gc_runtime() -> impl GcRuntime {
    DisabledCollector
}

struct DisabledCollector;

unsafe impl GcRuntime for DisabledCollector {
    fn new_gc_heap(&self) -> Result<Box<dyn GcHeap>> {
        unreachable!()
    }

    fn array_layout(&self, _ty: &WasmArrayType) -> GcArrayLayout {
        unreachable!()
    }

    fn struct_layout(&self, _ty: &WasmStructType) -> GcStructLayout {
        unreachable!()
    }
}

pub enum VMExternRef {}

pub enum VMStructRef {}

pub enum VMArrayRef {}

pub struct VMGcObjectDataMut<'a> {
    inner: VMStructRef,
    _phantom: core::marker::PhantomData<&'a mut ()>,
}
