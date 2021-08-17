//! Memory management for executable code.

use crate::unwind::UnwindRegistration;
use anyhow::{Context, Result};
use object::read::{File as ObjectFile, Object, ObjectSection, ObjectSymbol};
use std::collections::BTreeMap;
use std::mem::ManuallyDrop;
use wasmtime_environ::obj::{try_parse_func_name, try_parse_trampoline_name};
use wasmtime_environ::{FuncIndex, SignatureIndex};
use wasmtime_runtime::{Mmap, VMFunctionBody};

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

    // Note that this intentionally excludes any unwinding information, if
    // present, since consumers largely are only interested in code memory
    // itself.
    fn range(&self) -> (usize, usize) {
        let start = self.mmap.as_ptr() as usize;
        let end = start + self.text_len;
        (start, end)
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

pub struct CodeMemoryObjectAllocation<'a, 'b> {
    pub code_range: &'a mut [u8],
    funcs: BTreeMap<FuncIndex, (usize, usize)>,
    trampolines: BTreeMap<SignatureIndex, (usize, usize)>,
    pub obj: ObjectFile<'b>,
}

impl<'a> CodeMemoryObjectAllocation<'a, '_> {
    pub fn funcs_len(&self) -> usize {
        self.funcs.len()
    }

    pub fn trampolines_len(&self) -> usize {
        self.trampolines.len()
    }

    pub fn funcs(&'a self) -> impl Iterator<Item = (FuncIndex, &'a mut [VMFunctionBody])> + 'a {
        let buf = self.code_range as *const _ as *mut [u8];
        self.funcs.iter().map(move |(i, (start, len))| {
            (*i, unsafe {
                CodeMemory::view_as_mut_vmfunc_slice(&mut (*buf)[*start..*start + *len])
            })
        })
    }

    pub fn trampolines(
        &'a self,
    ) -> impl Iterator<Item = (SignatureIndex, &'a mut [VMFunctionBody])> + 'a {
        let buf = self.code_range as *const _ as *mut [u8];
        self.trampolines.iter().map(move |(i, (start, len))| {
            (*i, unsafe {
                CodeMemory::view_as_mut_vmfunc_slice(&mut (*buf)[*start..*start + *len])
            })
        })
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

    /// Convert mut a slice from u8 to VMFunctionBody.
    fn view_as_mut_vmfunc_slice(slice: &mut [u8]) -> &mut [VMFunctionBody] {
        let byte_ptr: *mut [u8] = slice;
        let body_ptr = byte_ptr as *mut [VMFunctionBody];
        unsafe { &mut *body_ptr }
    }

    /// Returns all published segment ranges.
    pub fn published_ranges<'a>(&'a self) -> impl Iterator<Item = (usize, usize)> + 'a {
        self.entries[..self.published]
            .iter()
            .map(|entry| entry.range())
    }

    /// Allocates and copies the ELF image code section into CodeMemory.
    /// Returns references to functions and trampolines defined there.
    pub fn allocate_for_object<'a, 'b>(
        &'a mut self,
        obj: &'b [u8],
    ) -> Result<CodeMemoryObjectAllocation<'a, 'b>> {
        let obj = ObjectFile::parse(obj)
            .with_context(|| "failed to parse internal ELF compilation artifact")?;
        let text_section = obj.section_by_name(".text").unwrap();
        let text_section_size = text_section.size() as usize;

        if text_section_size == 0 {
            // No code in the image.
            return Ok(CodeMemoryObjectAllocation {
                code_range: &mut [],
                funcs: BTreeMap::new(),
                trampolines: BTreeMap::new(),
                obj,
            });
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

        // Track locations of all defined functions and trampolines.
        let mut funcs = BTreeMap::new();
        let mut trampolines = BTreeMap::new();
        for sym in obj.symbols() {
            match sym.name() {
                Ok(name) => {
                    if let Some(index) = try_parse_func_name(name) {
                        let is_import = sym.section_index().is_none();
                        if !is_import {
                            funcs.insert(index, (sym.address() as usize, sym.size() as usize));
                        }
                    } else if let Some(index) = try_parse_trampoline_name(name) {
                        trampolines.insert(index, (sym.address() as usize, sym.size() as usize));
                    }
                }
                Err(_) => (),
            }
        }

        Ok(CodeMemoryObjectAllocation {
            code_range: &mut entry.mmap.as_mut_slice()[..text_section_size],
            funcs,
            trampolines,
            obj,
        })
    }
}
