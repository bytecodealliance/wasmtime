//! The dummy `VMExternRef` for when the `gc` cargo feature is disabled.
//!
//! To reduce `#[cfg(...)]`s, this provides all the same methods as the real
//! `VMExternRef` except for constructors.

#![allow(missing_docs)]

use crate::{ModuleInfoLookup, VMRuntimeLimits};
use core::any::Any;
use core::cmp;
use core::fmt;
use core::hash::Hasher;
use core::ops::Deref;

#[derive(Clone)]
enum Uninhabited {}

#[derive(Clone)]
pub struct VMExternRef(Uninhabited);

impl fmt::Pointer for VMExternRef {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {}
    }
}

impl Drop for VMExternRef {
    fn drop(&mut self) {
        match self.0 {}
    }
}

impl VMExternRef {
    /// This method is only available when the `gc` cargo feature is *disabled*.
    pub fn assert_unreachable<T>(&self) -> T {
        match self.0 {}
    }

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

pub struct VMExternRefActivationsTable {
    _priv: (),
}

impl VMExternRefActivationsTable {
    pub fn new() -> Self {
        Self { _priv: () }
    }

    pub fn bump_capacity_remaining(&self) -> usize {
        usize::MAX
    }

    pub fn try_insert(&mut self, externref: VMExternRef) -> Result<(), VMExternRef> {
        match externref.0 {}
    }

    pub unsafe fn insert_with_gc(
        &mut self,
        _limits: *const VMRuntimeLimits,
        externref: VMExternRef,
        _module_info_lookup: &dyn ModuleInfoLookup,
    ) {
        match externref.0 {}
    }

    pub fn insert_without_gc(&mut self, externref: VMExternRef) {
        match externref.0 {}
    }

    pub fn set_gc_okay(&mut self, _okay: bool) -> bool {
        true
    }
}

pub unsafe fn gc(
    _limits: *const VMRuntimeLimits,
    _module_info_lookup: &dyn ModuleInfoLookup,
    _externref_activations_table: &mut VMExternRefActivationsTable,
) {
    // Nothing to do.
}
