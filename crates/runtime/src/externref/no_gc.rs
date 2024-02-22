//! The dummy `VMExternRef` for when the `gc` cargo feature is disabled.
//!
//! To reduce `#[cfg(...)]`s, this provides all the same methods as the real
//! `VMExternRef` except for constructors.

#![allow(missing_docs)]

use crate::{ModuleInfoLookup, VMRuntimeLimits};
use std::any::Any;
use std::cmp;
use std::hash::Hasher;
use std::ops::Deref;

#[derive(Clone)]
enum Uninhabited {}

#[derive(Clone)]
pub struct VMExternRef(Uninhabited);

impl std::fmt::Pointer for VMExternRef {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {}
    }
}

impl Drop for VMExternRef {
    fn drop(&mut self) {
        match self.0 {}
    }
}

impl VMExternRef {
    pub fn as_raw(&self) -> *mut u8 {
        match self.0 {}
    }

    pub unsafe fn into_raw(self) -> *mut u8 {
        match self.0 {}
    }

    pub unsafe fn from_raw(ptr: *mut u8) -> Option<Self> {
        assert!(ptr.is_null());
        None
    }

    pub unsafe fn clone_from_raw(ptr: *mut u8) -> Option<Self> {
        assert!(ptr.is_null());
        None
    }

    pub fn strong_count(&self) -> usize {
        match self.0 {}
    }

    pub fn eq(a: &Self, _b: &Self) -> bool {
        match a.0 {}
    }

    pub fn hash<H>(externref: &Self, _hasher: &mut H)
    where
        H: Hasher,
    {
        match externref.0 {}
    }

    pub fn cmp(a: &Self, _b: &Self) -> cmp::Ordering {
        match a.0 {}
    }
}

impl Deref for VMExternRef {
    type Target = dyn Any;

    fn deref(&self) -> &dyn Any {
        match self.0 {}
    }
}

pub struct VMExternRefActivationsTable(Uninhabited);

impl VMExternRefActivationsTable {
    pub fn bump_capacity_remaining(&self) -> usize {
        match self.0 {}
    }

    pub fn try_insert(&mut self, _externref: VMExternRef) -> Result<(), VMExternRef> {
        match self.0 {}
    }

    pub unsafe fn insert_with_gc(
        &mut self,
        _limits: *const VMRuntimeLimits,
        _externref: VMExternRef,
        _module_info_lookup: &dyn ModuleInfoLookup,
    ) {
        match self.0 {}
    }

    pub fn insert_without_gc(&mut self, _externref: VMExternRef) {
        match self.0 {}
    }

    pub fn set_gc_okay(&mut self, _okay: bool) -> bool {
        match self.0 {}
    }
}

pub unsafe fn gc(
    _limits: *const VMRuntimeLimits,
    _module_info_lookup: &dyn ModuleInfoLookup,
    externref_activations_table: &mut VMExternRefActivationsTable,
) {
    match externref_activations_table.0 {}
}
