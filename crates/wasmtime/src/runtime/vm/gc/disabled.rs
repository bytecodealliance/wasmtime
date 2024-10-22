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

pub enum VMExternRef {}

pub enum VMEqRef {}

pub enum VMStructRef {}

pub enum VMArrayRef {}

pub struct VMGcObjectDataMut<'a> {
    inner: VMStructRef,
    _phantom: core::marker::PhantomData<&'a mut ()>,
}

impl VMGcObjectDataMut<'_> {
    pub fn new(_data: &mut [u8]) -> Self {
        unreachable!()
    }
}
