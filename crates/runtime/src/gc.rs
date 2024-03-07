#[cfg(feature = "gc")]
mod enabled;
#[cfg(feature = "gc")]
pub use enabled::*;

#[cfg(not(feature = "gc"))]
mod disabled;
#[cfg(not(feature = "gc"))]
pub use disabled::*;

use crate::SendSyncPtr;
use std::ptr::NonNull;
use wasmtime_environ::StackMap;

/// Used by the runtime to lookup information about a module given a
/// program counter value.
pub trait ModuleInfoLookup {
    /// Lookup the module information from a program counter value.
    fn lookup(&self, pc: usize) -> Option<&dyn ModuleInfo>;
}

/// Used by the runtime to query module information.
pub trait ModuleInfo {
    /// Lookup the stack map at a program counter value.
    fn lookup_stack_map(&self, pc: usize) -> Option<&StackMap>;
}

/// A raw, unrooted GC pointer.
///
/// We know that the referent is some kind of GC object, but we don't know
/// exactly which kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VMGcRef(SendSyncPtr<u8>);

impl std::fmt::Pointer for VMGcRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_non_null().fmt(f)
    }
}

impl VMGcRef {
    /// The only type of valid `VMGcRef` is currently `VMExternRef`.
    ///
    /// Assert on this anywhere you are making that assumption, so that we know
    /// all the places to update when it no longer holds true.
    pub const ONLY_EXTERN_REF_IMPLEMENTED_YET: bool = true;

    /// Create a new `VMGcRef`.
    ///
    /// Returns `None` for null pointers.
    ///
    /// # Safety
    ///
    /// The given pointer must point to a valid GC-managed object.
    pub unsafe fn from_ptr(raw: *mut u8) -> Option<Self> {
        let raw = NonNull::new(raw)?;
        Some(Self::from_non_null(raw))
    }

    /// Create a new `VMGcRef`.
    ///
    /// # Safety
    ///
    /// The given pointer must point to a valid GC-managed object.
    pub unsafe fn from_non_null(raw: NonNull<u8>) -> Self {
        VMGcRef(SendSyncPtr::new(raw))
    }

    /// Get this GC reference as a pointer.
    ///
    /// Note that the returned pointer does not point to a valid GC object when
    /// `self.is_i31()`.
    pub fn as_ptr(&self) -> *mut u8 {
        self.0.as_ptr()
    }

    /// Get this GC reference as a non-null pointer.
    pub fn as_non_null(&self) -> NonNull<u8> {
        self.0.as_non_null()
    }
}
