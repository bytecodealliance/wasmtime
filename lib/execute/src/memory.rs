use memmap;
use std::fmt;
use std::ops::{Deref, DerefMut};

const PAGE_SIZE: u32 = 65536;
const MAX_PAGES: u32 = 65536;

pub struct LinearMemory {
    mmap: memmap::MmapMut,
    current: u32,
    maximum: u32,
}

impl LinearMemory {
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

    pub fn base_addr(&self) -> *mut u8 {
        self.mmap.as_ptr() as *mut u8
    }

    pub fn current_size(&self) -> u32 {
        self.current
    }

    pub fn grow(&mut self, add_pages: u32) -> Option<u32> {
        let new_pages = self
            .current
            .checked_add(add_pages)
            .filter(|&new_pages| new_pages <= self.maximum)?;

        let prev_pages = self.current;
        self.current = new_pages;

        // Ensure that newly allocated area is zeroed.
        let new_start_offset = (prev_pages * PAGE_SIZE) as usize;
        let new_end_offset = (new_pages * PAGE_SIZE) as usize;
        for i in new_start_offset..new_end_offset - 1 {
            self[i] = 0;
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

impl Deref for LinearMemory {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.mmap
    }
}

impl DerefMut for LinearMemory {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.mmap
    }
}
