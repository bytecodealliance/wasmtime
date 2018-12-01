//! Memory management for linear memory.

use cast;
use mmap::Mmap;
use region;
use std::fmt;
use std::string::String;
use wasmtime_environ::{MemoryPlan, MemoryStyle, WASM_MAX_PAGES, WASM_PAGE_SIZE};

/// A linear memory instance.
///
/// This linear memory has a stable base address and at the same time allows
/// for dynamical growing.
pub struct LinearMemory {
    mmap: Mmap,
    current: u32,
    maximum: Option<u32>,
    offset_guard_size: usize,
}

impl LinearMemory {
    /// Create a new linear memory instance with specified minimum and maximum number of pages.
    pub fn new(plan: &MemoryPlan) -> Result<Self, String> {
        // `maximum` cannot be set to more than `65536` pages.
        assert!(plan.memory.minimum <= WASM_MAX_PAGES);
        assert!(plan.memory.maximum.is_none() || plan.memory.maximum.unwrap() <= WASM_MAX_PAGES);

        let offset_guard_bytes = plan.offset_guard_size as usize;

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
        unsafe {
            region::protect(
                mmap.as_ptr().add(mapped_bytes),
                inaccessible_bytes,
                region::Protection::None,
            ).expect("unable to make memory inaccessible");
        }

        Ok(Self {
            mmap,
            current: plan.memory.minimum,
            maximum: plan.memory.maximum,
            offset_guard_size: offset_guard_bytes,
        })
    }

    /// Returns an base address of this linear memory.
    pub fn base_addr(&mut self) -> *mut u8 {
        self.mmap.as_mut_ptr()
    }

    /// Returns a number of allocated wasm pages.
    pub fn current_size(&self) -> u32 {
        assert_eq!(self.mmap.len() % WASM_PAGE_SIZE as usize, 0);
        let num_pages = self.mmap.len() / WASM_PAGE_SIZE as usize;
        cast::u32(num_pages).unwrap()
    }

    /// Grow memory by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
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

        if new_bytes > self.mmap.len() {
            // If we have no maximum, this is a "dynamic" heap, and it's allowed to move.
            assert!(self.maximum.is_none());
            let mapped_pages = self.current as usize;
            let mapped_bytes = mapped_pages * WASM_PAGE_SIZE as usize;
            let guard_bytes = self.offset_guard_size;

            let mut new_mmap = Mmap::with_size(new_bytes).ok()?;

            // Make the offset-guard pages inaccessible.
            unsafe {
                region::protect(
                    new_mmap.as_ptr().add(mapped_bytes),
                    guard_bytes,
                    region::Protection::Read,
                ).expect("unable to make memory readonly");
            }

            new_mmap
                .as_mut_slice()
                .copy_from_slice(self.mmap.as_slice());

            self.mmap = new_mmap;
        }

        self.current = new_pages;

        Some(prev_pages)
    }
}

impl fmt::Debug for LinearMemory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LinearMemory")
            .field("current", &self.current)
            .field("maximum", &self.maximum)
            .finish()
    }
}

impl AsRef<[u8]> for LinearMemory {
    fn as_ref(&self) -> &[u8] {
        self.mmap.as_slice()
    }
}

impl AsMut<[u8]> for LinearMemory {
    fn as_mut(&mut self) -> &mut [u8] {
        self.mmap.as_mut_slice()
    }
}
