//! Memory management for executable code.
// Copy of wasmtime's wasmtime-jit/src/code_memory.rs

use core::{cmp, mem};
use region;
use std::string::String;
use std::vec::Vec;
use wasmtime_runtime::{Mmap, VMFunctionBody};

/// Memory manager for executable code.
pub(crate) struct CodeMemory {
    current: Mmap,
    mmaps: Vec<Mmap>,
    position: usize,
    published: usize,
}

impl CodeMemory {
    /// Create a new `CodeMemory` instance.
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
    fn allocate(&mut self, size: usize) -> Result<&mut [u8], String> {
        if self.current.len() - self.position < size {
            self.mmaps.push(mem::replace(
                &mut self.current,
                Mmap::with_at_least(cmp::max(0x10000, size))?,
            ));
            self.position = 0;
        }
        let old_position = self.position;
        self.position += size;
        Ok(&mut self.current.as_mut_slice()[old_position..self.position])
    }

    /// Convert mut a slice from u8 to VMFunctionBody.
    fn view_as_mut_vmfunc_slice(slice: &mut [u8]) -> &mut [VMFunctionBody] {
        let byte_ptr: *mut [u8] = slice;
        let body_ptr = byte_ptr as *mut [VMFunctionBody];
        unsafe { &mut *body_ptr }
    }

    /// Allocate enough memory to hold a copy of `slice` and copy the data into it.
    /// TODO: Reorganize the code that calls this to emit code directly into the
    /// mmap region rather than into a Vec that we need to copy in.
    pub fn allocate_copy_of_byte_slice(
        &mut self,
        slice: &[u8],
    ) -> Result<&mut [VMFunctionBody], String> {
        let new = self.allocate(slice.len())?;
        new.copy_from_slice(slice);
        Ok(Self::view_as_mut_vmfunc_slice(new))
    }

    /// Make all allocated memory executable.
    pub fn publish(&mut self) {
        self.mmaps
            .push(mem::replace(&mut self.current, Mmap::new()));
        self.position = 0;

        for m in &mut self.mmaps[self.published..] {
            if m.len() != 0 {
                unsafe {
                    region::protect(m.as_mut_ptr(), m.len(), region::Protection::ReadExecute)
                }
                .expect("unable to make memory readonly and executable");
            }
        }
        self.published = self.mmaps.len();
    }
}
