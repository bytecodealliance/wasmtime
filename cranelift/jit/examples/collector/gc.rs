//! Garbage collector implementation.
//!
//! This implementation is not fast and it does not scale. It is meant to
//! show a functional, yet simple, example implementation which can be used
//! as a first version.

use std::alloc::{Layout, alloc, dealloc};
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

/// Immutable, thread-transportable pointer type.
#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FunctionPtr(*const u8);

impl FunctionPtr {
    #[inline]
    pub fn new(ptr: *const u8) -> Self {
        FunctionPtr(ptr)
    }

    #[inline]
    pub fn ptr(self) -> *const u8 {
        self.0
    }
}

unsafe impl Send for FunctionPtr {}
unsafe impl Sync for FunctionPtr {}

const POINTER_ALIGNMENT: usize = std::mem::align_of::<*const ()>();

/// List of all managed allocations, which we need before we can deallocate
/// any allocations again.
///
/// While this isn't directly necessary, we need to know the layout of each
/// allocation, so that we can pass it to [`dealloc`]. This does limit
/// the time complexity of the garbage collector to at least `O(n)`.
pub static ALLOCATIONS: LazyLock<RwLock<HashMap<FunctionPtr, Layout>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Allocates a new object with the given size, in bytes.
///
/// The memory block created from the function is managed by the
/// runtime, allowing the garbage collector to deallocate it if it
/// determines that is is no longer in use.
pub(crate) fn allocate_object(size: u64) -> *mut u8 {
    let layout = Layout::from_size_align(size as usize, POINTER_ALIGNMENT).unwrap();
    let ptr = unsafe { alloc(layout) };

    ALLOCATIONS
        .try_write()
        .unwrap()
        .insert(FunctionPtr::new(ptr), layout);

    ptr
}

/// Triggers a garbage collection at the first applicable frame. If
/// no viable frame is found, returns without collecting.
///
/// This will inspect the current stack maps to find live objects
/// and deallocate any allocations which don't exist in the stack maps.
pub(crate) fn trigger_collection() {
    let Some(frame) = crate::frame::find_current_stack_map() else {
        return;
    };

    let allocations = ALLOCATIONS.try_read().unwrap();
    let live_objects = frame.stack_value_locations().collect::<Vec<_>>();

    // Find all the allocations which don't exist in the stack maps - i.e.
    // all the objects which are unreferenced / dead.
    let dead_objects = allocations
        .iter()
        .filter(|(alloc_ptr, _)| {
            !live_objects
                .iter()
                .any(|(_, live_ptr)| alloc_ptr.ptr() == *live_ptr)
        })
        .collect::<Vec<_>>();

    for (_stack_ptr, _obj_ptr) in live_objects {
        // If you want to implement a compacting- or generational garbage collector,
        // you can move the allocation, then write the new pointer to the `stack_ptr`
        // pointer.
    }

    for (obj_ptr, layout) in dead_objects {
        unsafe {
            dealloc(obj_ptr.ptr().cast_mut(), *layout);
        }
    }
}
