//! Dummy GC types for when the `gc` cargo feature is disabled.
//!
//! To reduce `#[cfg(...)]`s, this provides all the same methods as the real
//! `VMExternRef` except for constructors.

#![allow(missing_docs)]

use crate::prelude::*;
use crate::runtime::vm::{GcHeap, GcRuntime};
use wasmtime_environ::{
    GcArrayLayout, GcStructLayout, GcTypeLayouts, WasmArrayType, WasmStructType,
};

pub fn default_gc_runtime() -> impl GcRuntime {
    DisabledCollector
}

struct DisabledCollector;

unsafe impl GcRuntime for DisabledCollector {
    fn new_gc_heap(&self) -> Result<Box<dyn GcHeap>> {
        unreachable!()
    }

    fn layouts(&self) -> &dyn GcTypeLayouts {
        &DisabledLayouts
    }
}

struct DisabledLayouts;
impl GcTypeLayouts for DisabledLayouts {
    fn array_length_field_offset(&self) -> u32 {
        unreachable!()
    }

    fn array_layout(&self, _: &WasmArrayType) -> GcArrayLayout {
        unreachable!()
    }

    fn struct_layout(&self, _: &WasmStructType) -> GcStructLayout {
        unreachable!()
    }
}

pub enum VMExternRef {}

pub enum VMEqRef {}

pub enum VMStructRef {}

pub enum VMArrayRef {}

pub struct VMGcObjectDataMut<'a> {
    inner: VMStructRef,
    _phantom: core::marker::PhantomData<&'a mut ()>,
}
