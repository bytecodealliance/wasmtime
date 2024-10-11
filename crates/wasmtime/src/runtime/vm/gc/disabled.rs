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


pub struct VMGcObjectDataMut<'a> {
    inner: VMStructRef,
    _phantom: core::marker::PhantomData<&'a mut ()>,
}
