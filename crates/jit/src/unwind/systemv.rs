//! Module for System V ABI unwind registry.

use anyhow::Result;

/// Represents a registration of function unwind information for System V ABI.
pub struct UnwindRegistration {
    registrations: Vec<usize>,
}

extern "C" {
    // libunwind import
    fn __register_frame(fde: *const u8);
    fn __deregister_frame(fde: *const u8);
}

impl UnwindRegistration {
    /// Registers precompiled unwinding information with the system.
    ///
    /// The `_base_address` field is ignored here (only used on other
    /// platforms), but the `unwind_info` and `unwind_len` parameters should
    /// describe an in-memory representation of a `.eh_frame` section. This is
    /// typically arranged for by the `wasmtime-obj` crate.
    pub unsafe fn new(
        _base_address: *mut u8,
        unwind_info: *mut u8,
        unwind_len: usize,
    ) -> Result<UnwindRegistration> {
        let mut registrations = Vec::new();
        if cfg!(any(
            all(target_os = "linux", target_env = "gnu"),
            target_os = "freebsd"
        )) {
            // On gnu (libgcc), `__register_frame` will walk the FDEs until an
            // entry of length 0
            __register_frame(unwind_info);
            registrations.push(unwind_info as usize);
        } else {
            // For libunwind, `__register_frame` takes a pointer to a single
            // FDE. Note that we subtract 4 from the length of unwind info since
            // wasmtime-encode .eh_frame sections always have a trailing 32-bit
            // zero for the platforms above.
            let start = unwind_info;
            let end = start.add(unwind_len - 4);
            let mut current = start;

            // Walk all of the entries in the frame table and register them
            while current < end {
                let len = std::ptr::read::<u32>(current as *const u32) as usize;

                // Skip over the CIE
                if current != start {
                    __register_frame(current);
                    registrations.push(current as usize);
                }

                // Move to the next table entry (+4 because the length itself is
                // not inclusive)
                current = current.add(len + 4);
            }
        }

        Ok(UnwindRegistration { registrations })
    }

    pub fn section_name() -> &'static str {
        "_wasmtime_eh_frame"
    }
}

impl Drop for UnwindRegistration {
    fn drop(&mut self) {
        unsafe {
            // libgcc stores the frame entries as a linked list in decreasing
            // sort order based on the PC value of the registered entry.
            //
            // As we store the registrations in increasing order, it would be
            // O(N^2) to deregister in that order.
            //
            // To ensure that we just pop off the first element in the list upon
            // every deregistration, walk our list of registrations backwards.
            for fde in self.registrations.iter().rev() {
                __deregister_frame(*fde as *const _);
            }
        }
    }
}
