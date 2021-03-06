//! Memory management for linear memories.
//!
//! `RuntimeLinearMemory` is to WebAssembly linear memories what `Table` is to WebAssembly tables.

use crate::mmap::Mmap;
use crate::vmcontext::VMMemoryDefinition;
use anyhow::Result;
use more_asserts::{assert_ge, assert_le};
use std::cell::{Cell, RefCell};
use std::cmp::min;
use std::convert::TryFrom;
use std::ptr;
use wasmtime_environ::{MemoryPlan, MemoryStyle, WASM_MAX_PAGES, WASM_PAGE_SIZE};

/// A memory allocator
pub trait RuntimeMemoryCreator: Send + Sync {
    /// Create new RuntimeLinearMemory
    fn new_memory(&self, plan: &MemoryPlan) -> Result<Box<dyn RuntimeLinearMemory>>;
}

/// A default memory allocator used by Wasmtime
pub struct DefaultMemoryCreator;

impl RuntimeMemoryCreator for DefaultMemoryCreator {
    /// Create new MmapMemory
    fn new_memory(&self, plan: &MemoryPlan) -> Result<Box<dyn RuntimeLinearMemory>> {
        Ok(Box::new(MmapMemory::new(plan)?) as _)
    }
}

/// A linear memory
pub trait RuntimeLinearMemory {
    /// Returns the number of allocated wasm pages.
    fn size(&self) -> u32;

    /// Grow memory by the specified amount of wasm pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of wasm pages.
    fn grow(&self, delta: u32) -> Option<u32>;

    /// Return a `VMMemoryDefinition` for exposing the memory to compiled wasm code.
    fn vmmemory(&self) -> VMMemoryDefinition;
}

/// A linear memory instance.
#[derive(Debug)]
pub struct MmapMemory {
    // The underlying allocation.
    mmap: RefCell<WasmMmap>,

    // The optional maximum size in wasm pages of this linear memory.
    maximum: Option<u32>,

    // Size in bytes of extra guard pages after the end to optimize loads and stores with
    // constant offsets.
    offset_guard_size: usize,
}

#[derive(Debug)]
struct WasmMmap {
    // Our OS allocation of mmap'd memory.
    alloc: Mmap,
    // The current logical size in wasm pages of this linear memory.
    size: u32,
}

impl MmapMemory {
    /// Create a new linear memory instance with specified minimum and maximum number of wasm pages.
    pub fn new(plan: &MemoryPlan) -> Result<Self> {
        // `maximum` cannot be set to more than `65536` pages.
        assert_le!(plan.memory.minimum, WASM_MAX_PAGES);
        assert!(plan.memory.maximum.is_none() || plan.memory.maximum.unwrap() <= WASM_MAX_PAGES);

        let offset_guard_bytes = plan.offset_guard_size as usize;

        let minimum_pages = match plan.style {
            MemoryStyle::Dynamic => plan.memory.minimum,
            MemoryStyle::Static { bound } => {
                assert_ge!(bound, plan.memory.minimum);
                bound
            }
        } as usize;
        let minimum_bytes = minimum_pages.checked_mul(WASM_PAGE_SIZE as usize).unwrap();
        let request_bytes = minimum_bytes.checked_add(offset_guard_bytes).unwrap();
        let mapped_pages = plan.memory.minimum as usize;
        let mapped_bytes = mapped_pages * WASM_PAGE_SIZE as usize;

        let mmap = WasmMmap {
            alloc: Mmap::accessible_reserved(mapped_bytes, request_bytes)?,
            size: plan.memory.minimum,
        };

        Ok(Self {
            mmap: mmap.into(),
            maximum: plan.memory.maximum,
            offset_guard_size: offset_guard_bytes,
        })
    }
}

impl RuntimeLinearMemory for MmapMemory {
    /// Returns the number of allocated wasm pages.
    fn size(&self) -> u32 {
        self.mmap.borrow().size
    }

    /// Grow memory by the specified amount of wasm pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of wasm pages.
    fn grow(&self, delta: u32) -> Option<u32> {
        // Optimization of memory.grow 0 calls.
        let mut mmap = self.mmap.borrow_mut();
        if delta == 0 {
            return Some(mmap.size);
        }

        let new_pages = match mmap.size.checked_add(delta) {
            Some(new_pages) => new_pages,
            // Linear memory size overflow.
            None => return None,
        };
        let prev_pages = mmap.size;

        if let Some(maximum) = self.maximum {
            if new_pages > maximum {
                // Linear memory size would exceed the declared maximum.
                return None;
            }
        }

        // Wasm linear memories are never allowed to grow beyond what is
        // indexable. If the memory has no maximum, enforce the greatest
        // limit here.
        if new_pages >= WASM_MAX_PAGES {
            // Linear memory size would exceed the index range.
            return None;
        }

        let delta_bytes = usize::try_from(delta).unwrap() * WASM_PAGE_SIZE as usize;
        let prev_bytes = usize::try_from(prev_pages).unwrap() * WASM_PAGE_SIZE as usize;
        let new_bytes = usize::try_from(new_pages).unwrap() * WASM_PAGE_SIZE as usize;

        if new_bytes > mmap.alloc.len() - self.offset_guard_size {
            // If the new size is within the declared maximum, but needs more memory than we
            // have on hand, it's a dynamic heap and it can move.
            let guard_bytes = self.offset_guard_size;
            let request_bytes = new_bytes.checked_add(guard_bytes)?;

            let mut new_mmap = Mmap::accessible_reserved(new_bytes, request_bytes).ok()?;

            let copy_len = mmap.alloc.len() - self.offset_guard_size;
            new_mmap.as_mut_slice()[..copy_len].copy_from_slice(&mmap.alloc.as_slice()[..copy_len]);

            mmap.alloc = new_mmap;
        } else if delta_bytes > 0 {
            // Make the newly allocated pages accessible.
            mmap.alloc.make_accessible(prev_bytes, delta_bytes).ok()?;
        }

        mmap.size = new_pages;

        Some(prev_pages)
    }

    /// Return a `VMMemoryDefinition` for exposing the memory to compiled wasm code.
    fn vmmemory(&self) -> VMMemoryDefinition {
        let mmap = self.mmap.borrow();
        VMMemoryDefinition {
            base: mmap.alloc.as_mut_ptr(),
            current_length: mmap.size as usize * WASM_PAGE_SIZE as usize,
        }
    }
}

enum MemoryStorage {
    Static {
        base: *mut u8,
        size: Cell<u32>,
        maximum: u32,
        make_accessible: fn(*mut u8, usize) -> Result<()>,
    },
    Dynamic(Box<dyn RuntimeLinearMemory>),
}

/// Represents an instantiation of a WebAssembly memory.
pub struct Memory(MemoryStorage);

impl Memory {
    /// Create a new dynamic (movable) memory instance for the specified plan.
    pub fn new_dynamic(plan: &MemoryPlan, creator: &dyn RuntimeMemoryCreator) -> Result<Self> {
        Ok(Self(MemoryStorage::Dynamic(creator.new_memory(plan)?)))
    }

    /// Create a new static (immovable) memory instance for the specified plan.
    pub fn new_static(
        plan: &MemoryPlan,
        base: *mut u8,
        maximum: u32,
        make_accessible: fn(*mut u8, usize) -> Result<()>,
    ) -> Result<Self> {
        if plan.memory.minimum > 0 {
            make_accessible(base, plan.memory.minimum as usize * WASM_PAGE_SIZE as usize)?;
        }

        Ok(Self(MemoryStorage::Static {
            base,
            size: Cell::new(plan.memory.minimum),
            maximum: min(plan.memory.maximum.unwrap_or(maximum), maximum),
            make_accessible,
        }))
    }

    /// Returns the number of allocated wasm pages.
    pub fn size(&self) -> u32 {
        match &self.0 {
            MemoryStorage::Static { size, .. } => size.get(),
            MemoryStorage::Dynamic(mem) => mem.size(),
        }
    }

    /// Returns whether or not the underlying storage of the memory is "static".
    pub(crate) fn is_static(&self) -> bool {
        if let MemoryStorage::Static { .. } = &self.0 {
            true
        } else {
            false
        }
    }

    /// Grow memory by the specified amount of wasm pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of wasm pages.
    pub fn grow(&self, delta: u32) -> Option<u32> {
        match &self.0 {
            MemoryStorage::Static {
                base,
                size,
                maximum,
                make_accessible,
                ..
            } => {
                let old_size = size.get();
                if delta == 0 {
                    return Some(old_size);
                }

                let new_size = old_size.checked_add(delta)?;

                if new_size > *maximum || new_size >= WASM_MAX_PAGES {
                    return None;
                }

                let start = usize::try_from(old_size).unwrap() * WASM_PAGE_SIZE as usize;
                let len = usize::try_from(delta).unwrap() * WASM_PAGE_SIZE as usize;

                make_accessible(unsafe { base.add(start) }, len).ok()?;

                size.set(new_size);

                Some(old_size)
            }
            MemoryStorage::Dynamic(mem) => mem.grow(delta),
        }
    }

    /// Return a `VMMemoryDefinition` for exposing the memory to compiled wasm code.
    pub fn vmmemory(&self) -> VMMemoryDefinition {
        match &self.0 {
            MemoryStorage::Static { base, size, .. } => VMMemoryDefinition {
                base: *base,
                current_length: size.get() as usize * WASM_PAGE_SIZE as usize,
            },
            MemoryStorage::Dynamic(mem) => mem.vmmemory(),
        }
    }
}

// The default memory representation is an empty memory that cannot grow.
impl Default for Memory {
    fn default() -> Self {
        fn make_accessible(_ptr: *mut u8, _len: usize) -> Result<()> {
            unreachable!()
        }

        Self(MemoryStorage::Static {
            base: ptr::null_mut(),
            size: Cell::new(0),
            maximum: 0,
            make_accessible,
        })
    }
}
