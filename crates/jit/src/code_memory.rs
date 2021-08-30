//! Memory management for executable code.

use crate::unwind::UnwindRegistration;
use anyhow::{Context, Result};
use object::read::{File as ObjectFile, Object, ObjectSection};
use std::mem::ManuallyDrop;
use wasmtime_runtime::Mmap;

struct CodeMemoryEntry {
    mmap: ManuallyDrop<Mmap>,
    unwind_registration: ManuallyDrop<Option<UnwindRegistration>>,
    text_len: usize,
    unwind_info_len: usize,
}

impl CodeMemoryEntry {
    fn new(text_len: usize, unwind_info_len: usize) -> Result<Self> {
        let mmap = ManuallyDrop::new(Mmap::with_at_least(text_len + unwind_info_len)?);
        Ok(Self {
            mmap,
            unwind_registration: ManuallyDrop::new(None),
            text_len,
            unwind_info_len,
        })
    }
}

impl Drop for CodeMemoryEntry {
    fn drop(&mut self) {
        unsafe {
            // The registry needs to be dropped before the mmap
            ManuallyDrop::drop(&mut self.unwind_registration);
            ManuallyDrop::drop(&mut self.mmap);
        }
    }
}

/// Memory manager for executable code.
pub struct CodeMemory {
    entries: Vec<CodeMemoryEntry>,
    published: usize,
}

fn _assert() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<CodeMemory>();
}

impl CodeMemory {
    /// Create a new `CodeMemory` instance.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            published: 0,
        }
    }

    /// Make all allocated memory executable.
    pub fn publish(&mut self) {
        for entry in &mut self.entries[self.published..] {
            assert!(!entry.mmap.is_empty());

            unsafe {
                // Switch the executable portion from read/write to
                // read/execute, notably not using read/write/execute to prevent
                // modifications.
                region::protect(
                    entry.mmap.as_mut_ptr(),
                    entry.text_len,
                    region::Protection::READ_EXECUTE,
                )
                .expect("unable to make memory readonly and executable");

                if entry.unwind_info_len == 0 {
                    continue;
                }

                // With all our memory setup use the platform-specific
                // `UnwindRegistration` implementation to inform the general
                // runtime that there's unwinding information available for all
                // our just-published JIT functions.
                *entry.unwind_registration = Some(
                    UnwindRegistration::new(
                        entry.mmap.as_mut_ptr(),
                        entry.mmap.as_mut_ptr().add(entry.text_len),
                        entry.unwind_info_len,
                    )
                    .expect("failed to create unwind info registration"),
                );
            }
        }

        self.published = self.entries.len();
    }

    /// Alternative to `allocate_for_object`, but when the object file isn't
    /// already parsed.
    pub fn allocate_for_object_unparsed<'a, 'b>(
        &'a mut self,
        obj: &'b [u8],
    ) -> Result<(&'a mut [u8], ObjectFile<'b>)> {
        let obj = ObjectFile::parse(obj)?;
        Ok((self.allocate_for_object(&obj)?, obj))
    }

    /// Allocates and copies the ELF image code section into CodeMemory.
    /// Returns references to functions and trampolines defined there.
    pub fn allocate_for_object(&mut self, obj: &ObjectFile) -> Result<&mut [u8]> {
        let text_section = obj.section_by_name(".text").unwrap();
        let text_section_size = text_section.size() as usize;

        if text_section_size == 0 {
            // No code in the image.
            return Ok(&mut []);
        }

        // Find the platform-specific unwind section, if present, which contains
        // unwinding tables that will be used to load unwinding information
        // dynamically at runtime.
        let unwind_section = obj.section_by_name(UnwindRegistration::section_name());
        let unwind_section_size = unwind_section
            .as_ref()
            .map(|s| s.size() as usize)
            .unwrap_or(0);

        // Allocate memory for the text section and unwinding information if it
        // is present. Then we can copy in all of the code and unwinding memory
        // over.
        let entry = CodeMemoryEntry::new(text_section_size, unwind_section_size)?;
        self.entries.push(entry);
        let entry = self.entries.last_mut().unwrap();
        entry.mmap.as_mut_slice()[..text_section_size].copy_from_slice(
            text_section
                .data()
                .with_context(|| "cannot read text section data")?,
        );
        if let Some(section) = unwind_section {
            entry.mmap.as_mut_slice()[text_section_size..][..unwind_section_size].copy_from_slice(
                section
                    .data()
                    .with_context(|| "cannot read unwind section data")?,
            );
        }

        Ok(&mut entry.mmap.as_mut_slice()[..text_section_size])
    }
}
