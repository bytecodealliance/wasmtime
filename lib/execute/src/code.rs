//! Memory management for executable code.

use mmap::Mmap;
use region;
use std::cmp;
use std::mem;
use std::slice;
use std::string::String;
use std::vec::Vec;

/// Memory manager for executable code.
pub struct Code {
    current: Mmap,
    mmaps: Vec<Mmap>,
    position: usize,
    published: usize,
}

impl Code {
    /// Create a new `Code` instance.
    pub fn new() -> Self {
        Self {
            current: Mmap::new(),
            mmaps: Vec::new(),
            position: 0,
            published: 0,
        }
    }

    /// Allocate `size` bytes of memory which can be made executable later by
    /// calling `publish()`. Note that we allocate the memory as writeable so
    /// that it can be written to and patched, though we make it readonly before
    /// actually executing from it.
    ///
    /// TODO: Add an alignment flag.
    fn allocate(&mut self, size: usize) -> Result<*mut u8, String> {
        if self.current.len() - self.position < size {
            self.mmaps.push(mem::replace(
                &mut self.current,
                Mmap::with_size(cmp::max(0x10000, size.next_power_of_two()))?,
            ));
            self.position = 0;
        }
        let old_position = self.position;
        self.position += size;
        Ok(self.current.as_mut_slice()[old_position..self.position].as_mut_ptr())
    }

    /// Allocate enough memory to hold a copy of `slice` and copy the data into it.
    /// TODO: Reorganize the code that calls this to emit code directly into the
    /// mmap region rather than into a Vec that we need to copy in.
    pub fn allocate_copy_of_slice(&mut self, slice: &[u8]) -> Result<&mut [u8], String> {
        let ptr = self.allocate(slice.len())?;
        let new = unsafe { slice::from_raw_parts_mut(ptr, slice.len()) };
        new.copy_from_slice(slice);
        Ok(new)
    }

    /// Make all allocated memory executable.
    pub fn publish(&mut self) {
        self.mmaps
            .push(mem::replace(&mut self.current, Mmap::new()));
        self.position = 0;

        for m in &mut self.mmaps[self.published..] {
            if !m.as_ptr().is_null() {
                unsafe {
                    region::protect(m.as_mut_ptr(), m.len(), region::Protection::ReadExecute)
                }
                .expect("unable to make memory readonly and executable");
            }
        }
        self.published = self.mmaps.len();
    }
}
