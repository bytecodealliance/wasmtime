//! Runtime function table.
//!
//! This module is primarily used to track JIT functions on Windows for stack walking and unwind.

/// Represents a runtime function table.
///
/// The runtime function table is not implemented for non-Windows target platforms.
#[cfg(not(target_os = "windows"))]
pub(crate) struct FunctionTable;

#[cfg(not(target_os = "windows"))]
impl FunctionTable {
    /// Creates a new function table.
    pub fn new() -> Self {
        Self
    }

    /// Returns the number of functions in the table, also referred to as its 'length'.
    ///
    /// For non-Windows platforms, the table will always be empty.
    pub fn len(&self) -> usize {
        0
    }

    /// Adds a function to the table based off of the start offset, end offset, and unwind offset.
    ///
    /// The offsets are from the "module base", which is provided when the table is published.
    ///
    /// For non-Windows platforms, this is a no-op.
    pub fn add_function(&mut self, _start: u32, _end: u32, _unwind: u32) {}

    /// Publishes the function table using the given base address.
    ///
    /// A published function table will automatically be deleted when it is dropped.
    ///
    /// For non-Windows platforms, this is a no-op.
    pub fn publish(&mut self, _base_address: u64) -> Result<(), String> {
        Ok(())
    }
}

/// Represents a runtime function table.
///
/// This is used to register JIT code with the operating system to enable stack walking and unwinding.
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub(crate) struct FunctionTable {
    functions: Vec<winapi::um::winnt::RUNTIME_FUNCTION>,
    published: bool,
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
impl FunctionTable {
    /// Creates a new function table.
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            published: false,
        }
    }

    /// Returns the number of functions in the table, also referred to as its 'length'.
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Adds a function to the table based off of the start offset, end offset, and unwind offset.
    ///
    /// The offsets are from the "module base", which is provided when the table is published.
    pub fn add_function(&mut self, start: u32, end: u32, unwind: u32) {
        use winapi::um::winnt;

        assert!(!self.published, "table has already been published");

        let mut entry = winnt::RUNTIME_FUNCTION::default();

        entry.BeginAddress = start;
        entry.EndAddress = end;

        unsafe {
            *entry.u.UnwindInfoAddress_mut() = unwind;
        }

        self.functions.push(entry);
    }

    /// Publishes the function table using the given base address.
    ///
    /// A published function table will automatically be deleted when it is dropped.
    pub fn publish(&mut self, base_address: u64) -> Result<(), String> {
        use winapi::um::winnt;

        if self.published {
            return Err("function table was already published".into());
        }

        self.published = true;

        if self.functions.is_empty() {
            return Ok(());
        }

        unsafe {
            // Windows heap allocations are 32-bit aligned, but assert just in case
            assert_eq!(
                (self.functions.as_mut_ptr() as u64) % 4,
                0,
                "function table allocation was not aligned"
            );

            if winnt::RtlAddFunctionTable(
                self.functions.as_mut_ptr(),
                self.functions.len() as u32,
                base_address,
            ) == 0
            {
                return Err("failed to add function table".into());
            }
        }

        Ok(())
    }
}

#[cfg(target_os = "windows")]
impl Drop for FunctionTable {
    fn drop(&mut self) {
        use winapi::um::winnt;

        if self.published {
            unsafe {
                winnt::RtlDeleteFunctionTable(self.functions.as_mut_ptr());
            }
        }
    }
}
