//! Memory management for linear memories.
//!
//! `LinearMemory` is to WebAssembly linear memories what `Table` is to WebAssembly tables.

use mmap::Mmap;
use region;
use std::string::String;
use vmcontext::VMMemoryDefinition;
use wasmtime_environ::{MemoryPlan, MemoryStyle, WASM_MAX_PAGES, WASM_PAGE_SIZE};

/// A linear memory instance.
#[derive(Debug)]
pub struct LinearMemory {
    // The underlying allocation.
    mmap: Mmap,

    // The current logical size in wasm pages of this linear memory.
    current: u32,

    // The optional maximum size in wasm pages of this linear memory.
    maximum: Option<u32>,

    // Size in bytes of extra guard pages after the end to optimize loads and stores with
    // constant offsets.
    offset_guard_size: usize,

    // Records whether we're using a bounds-checking strategy which requires
    // handlers to catch trapping accesses.
    pub(crate) needs_signal_handlers: bool,
}

impl LinearMemory {
    /// Create a new linear memory instance with specified minimum and maximum number of wasm pages.
    pub fn new(plan: &MemoryPlan) -> Result<Self, String> {
        // `maximum` cannot be set to more than `65536` pages.
        assert!(plan.memory.minimum <= WASM_MAX_PAGES);
        assert!(plan.memory.maximum.is_none() || plan.memory.maximum.unwrap() <= WASM_MAX_PAGES);

        let offset_guard_bytes = plan.offset_guard_size as usize;

        // If we have an offset guard, or if we're doing the static memory
        // allocation strategy, we need signal handlers to catch out of bounds
        // acceses.
        let needs_signal_handlers = offset_guard_bytes > 0
            || match plan.style {
                MemoryStyle::Dynamic => false,
                MemoryStyle::Static { .. } => true,
            };

        let minimum_pages = match plan.style {
            MemoryStyle::Dynamic => plan.memory.minimum,
            MemoryStyle::Static { bound } => {
                assert!(bound >= plan.memory.minimum);
                bound
            }
        } as usize;
        let minimum_bytes = minimum_pages.checked_mul(WASM_PAGE_SIZE as usize).unwrap();
        let request_bytes = minimum_bytes.checked_add(offset_guard_bytes).unwrap();
        let mapped_pages = plan.memory.minimum as usize;
        let mapped_bytes = mapped_pages * WASM_PAGE_SIZE as usize;
        let unmapped_pages = minimum_pages - mapped_pages;
        let unmapped_bytes = unmapped_pages * WASM_PAGE_SIZE as usize;
        let inaccessible_bytes = unmapped_bytes + offset_guard_bytes;

        let mmap = Mmap::with_size(request_bytes)?;

        // Make the unmapped and offset-guard pages inaccessible.
        if request_bytes != 0 {
            unsafe {
                region::protect(
                    mmap.as_ptr().add(mapped_bytes),
                    inaccessible_bytes,
                    region::Protection::None,
                )
            }
            .expect("unable to make memory inaccessible");
        }

        Ok(Self {
            mmap,
            current: plan.memory.minimum,
            maximum: plan.memory.maximum,
            offset_guard_size: offset_guard_bytes,
            needs_signal_handlers,
        })
    }

    /// Returns the number of allocated wasm pages.
    pub fn size(&self) -> u32 {
        self.current
    }

    /// Grow memory by the specified amount of wasm pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of wasm pages.
    pub fn grow(&mut self, delta: u32) -> Option<u32> {
        let new_pages = match self.current.checked_add(delta) {
            Some(new_pages) => new_pages,
            // Linear memory size overflow.
            None => return None,
        };
        let prev_pages = self.current;

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

        let new_bytes = new_pages as usize * WASM_PAGE_SIZE as usize;

        if new_bytes > self.mmap.len() - self.offset_guard_size {
            // If we have no maximum, this is a "dynamic" heap, and it's allowed to move.
            let guard_bytes = self.offset_guard_size;
            let request_bytes = new_bytes.checked_add(guard_bytes)?;

            let mut new_mmap = Mmap::with_size(request_bytes).ok()?;

            // Make the offset-guard pages inaccessible.
            unsafe {
                region::protect(
                    new_mmap.as_ptr().add(new_bytes),
                    guard_bytes,
                    region::Protection::None,
                )
            }
            .expect("unable to make memory inaccessible");

            let copy_len = self.mmap.len() - self.offset_guard_size;
            new_mmap.as_mut_slice()[..copy_len].copy_from_slice(&self.mmap.as_slice()[..copy_len]);

            self.mmap = new_mmap;
        }

        self.current = new_pages;

        Some(prev_pages)
    }

    /// Return a `VMMemoryDefinition` for exposing the memory to compiled wasm code.
    pub fn vmmemory(&mut self) -> VMMemoryDefinition {
        VMMemoryDefinition {
            base: self.mmap.as_mut_ptr(),
            current_length: self.current as usize * WASM_PAGE_SIZE as usize,
        }
    }
}
