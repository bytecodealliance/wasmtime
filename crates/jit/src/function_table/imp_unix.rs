use super::FunctionTableReloc;

cfg_if::cfg_if! {
    if #[cfg(target_arch="arm")] {
        // there are no such exports on arm.
        #[no_mangle]
        unsafe extern "C" fn __register_frame(_fde: *const u8) {
            core::hint::unreachable_unchecked()
        }
        #[no_mangle]
        unsafe extern "C" fn __deregister_frame(_fde: *const u8) {
            core::hint::unreachable_unchecked()
        }
    } else {
        extern "C" {
            // libunwind imports
            fn __register_frame(fde: *const u8);
            fn __deregister_frame(fde: *const u8);
        }
    }
}

/// Represents a runtime function table.
///
/// This is used to register JIT code with the operating system to enable stack walking and unwinding.
pub(crate) struct FunctionTable {
    functions: Vec<u32>,
    relocs: Vec<FunctionTableReloc>,
    published: Option<Vec<usize>>,
}

impl FunctionTable {
    /// Creates a new function table.
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            relocs: Vec::new(),
            published: None,
        }
    }

    /// Returns the number of functions in the table, also referred to as its 'length'.
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Adds a function to the table based off of the start offset, end offset, and unwind offset.
    ///
    /// The offsets are from the "module base", which is provided when the table is published.
    pub fn add_function(
        &mut self,
        _start: u32,
        _end: u32,
        unwind: u32,
        relocs: &[FunctionTableReloc],
    ) {
        assert!(self.published.is_none(), "table has already been published");
        self.functions.push(unwind);
        self.relocs.extend_from_slice(relocs);
    }

    /// Publishes the function table using the given base address.
    ///
    /// A published function table will automatically be deleted when it is dropped.
    pub fn publish(&mut self, base_address: u64) -> Result<(), String> {
        if self.published.is_some() {
            return Err("function table was already published".into());
        }

        if self.functions.is_empty() {
            assert_eq!(self.relocs.len(), 0);
            self.published = Some(vec![]);
            return Ok(());
        }

        for reloc in self.relocs.iter() {
            let addr = base_address + (reloc.offset as u64);
            let target = base_address + (reloc.addend as u64);
            unsafe {
                std::ptr::write(addr as *mut u64, target);
            }
        }

        let mut fdes = Vec::with_capacity(self.functions.len());
        for unwind_offset in self.functions.iter() {
            let addr = base_address + (*unwind_offset as u64);
            let off = unsafe { std::ptr::read::<u32>(addr as *const u32) } as usize + 4;

            let fde = (addr + off as u64) as usize;
            unsafe {
                __register_frame(fde as *const _);
            }
            fdes.push(fde);
        }

        self.published = Some(fdes);
        Ok(())
    }
}

impl Drop for FunctionTable {
    fn drop(&mut self) {
        if let Some(published) = &self.published {
            unsafe {
                // I'm not really sure why, but it appears to be way faster to
                // unregister frames in reverse order rather than in-order. This
                // way we're deregistering in LIFO order, and maybe there's some
                // vec shifting or something like that in libgcc?
                //
                // Locally on Ubuntu 18.04 a wasm module with 40k empty
                // functions takes 0.1s to compile and drop with reverse
                // iteration. With forward iteration it takes 3s to compile and
                // drop!
                //
                // Poking around libgcc sources seems to indicate that some sort
                // of linked list is being traversed... We may need to figure
                // out something else for backtraces in the future since this
                // API may not be long-lived to keep calling.
                for fde in published.iter().rev() {
                    __deregister_frame(*fde as *const _);
                }
            }
        }
    }
}

