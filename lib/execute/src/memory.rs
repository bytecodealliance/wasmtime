use memmap;
use std::fmt;

const PAGE_SIZE: u32 = 65536;
const MAX_PAGES: u32 = 65536;

/// A linear memory instance.
///
/// This linear memory has a stable base address and at the same time allows
/// for dynamical growing.
pub struct LinearMemory {
    mmap: memmap::MmapMut,
    current: u32,
    maximum: u32,
}

impl LinearMemory {
    /// Create a new linear memory instance with specified initial and maximum number of pages.
    ///
    /// `maximum` cannot be set to more than `65536` pages. If `maximum` is `None` then it
    /// will be treated as `65336`.
    pub fn new(initial: u32, maximum: Option<u32>) -> Self {
        let maximum = maximum.unwrap_or(MAX_PAGES);

        assert!(initial <= MAX_PAGES);
        assert!(maximum <= MAX_PAGES);

        let len = maximum.saturating_mul(MAX_PAGES);
        let mmap = memmap::MmapMut::map_anon(len as usize).unwrap();
        Self {
            mmap,
            current: initial,
            maximum,
        }
    }

    /// Returns an base address of this linear memory.
    pub fn base_addr(&mut self) -> *mut u8 {
        self.mmap.as_mut_ptr()
    }

    /// Returns a number of allocated wasm pages.
    pub fn current_size(&self) -> u32 {
        self.current
    }

    /// Grow memory by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
    pub fn grow(&mut self, add_pages: u32) -> Option<u32> {
        let new_pages = match self.current.checked_add(add_pages) {
            Some(new_pages) => new_pages,
            None => return None,
        };

        let prev_pages = self.current;
        self.current = new_pages;

        // Ensure that newly allocated area is zeroed.
        let new_start_offset = (prev_pages * PAGE_SIZE) as usize;
        let new_end_offset = (new_pages * PAGE_SIZE) as usize;
        for i in new_start_offset..new_end_offset - 1 {
            self.mmap[i] = 0;
        }

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
        &self.mmap
    }
}

impl AsMut<[u8]> for LinearMemory {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.mmap
    }
}
