use super::FunctionTableReloc;

/// Represents a runtime function table.
///
/// This is used to register JIT code with the operating system to enable stack walking and unwinding.
pub(crate) struct FunctionTable {
    functions: Vec<winapi::um::winnt::RUNTIME_FUNCTION>,
    published: bool,
}

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
    pub fn add_function(
        &mut self,
        start: u32,
        end: u32,
        unwind: u32,
        _relocs: &[FunctionTableReloc],
    ) {
        assert_eq!(_relocs.len(), 0);
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
