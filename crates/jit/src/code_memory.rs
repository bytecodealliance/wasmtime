//! Memory management for executable code.

use crate::unwind::UnwindRegistry;
use region;
use std::mem::ManuallyDrop;
use std::{cmp, mem};
use wasmtime_environ::{
    isa::{unwind::UnwindInfo, TargetIsa},
    Compilation, CompiledFunction, Relocation, Relocations,
};
use wasmtime_runtime::{Mmap, VMFunctionBody};

type CodeMemoryRelocations = Vec<(u32, Vec<Relocation>)>;

struct CodeMemoryEntry {
    mmap: ManuallyDrop<Mmap>,
    registry: ManuallyDrop<UnwindRegistry>,
    relocs: CodeMemoryRelocations,
}

impl CodeMemoryEntry {
    fn with_capacity(cap: usize) -> Result<Self, String> {
        let mmap = ManuallyDrop::new(Mmap::with_at_least(cap)?);
        let registry = ManuallyDrop::new(UnwindRegistry::new(mmap.as_ptr() as usize));
        Ok(Self {
            mmap,
            registry,
            relocs: vec![],
        })
    }

    fn range(&self) -> (usize, usize) {
        let start = self.mmap.as_ptr() as usize;
        let end = start + self.mmap.len();
        (start, end)
    }
}

impl Drop for CodeMemoryEntry {
    fn drop(&mut self) {
        unsafe {
            // The registry needs to be dropped before the mmap
            ManuallyDrop::drop(&mut self.registry);
            ManuallyDrop::drop(&mut self.mmap);
        }
    }
}

/// Memory manager for executable code.
pub struct CodeMemory {
    current: Option<CodeMemoryEntry>,
    entries: Vec<CodeMemoryEntry>,
    position: usize,
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
            current: None,
            entries: Vec::new(),
            position: 0,
            published: 0,
        }
    }

    /// Allocate a continuous memory block for a single compiled function.
    /// TODO: Reorganize the code that calls this to emit code directly into the
    /// mmap region rather than into a Vec that we need to copy in.
    pub fn allocate_for_function<'a>(
        &mut self,
        func: &'a CompiledFunction,
        relocs: impl Iterator<Item = &'a Relocation>,
    ) -> Result<&mut [VMFunctionBody], String> {
        let size = Self::function_allocation_size(func);

        let (buf, registry, start, m_relocs) = self.allocate(size)?;

        let (_, _, vmfunc) = Self::copy_function(func, start as u32, buf, registry);

        Self::copy_relocs(m_relocs, start as u32, relocs);

        Ok(vmfunc)
    }

    /// Allocate a continuous memory block for a compilation.
    pub fn allocate_for_compilation(
        &mut self,
        compilation: &Compilation,
        relocations: &Relocations,
    ) -> Result<Box<[&mut [VMFunctionBody]]>, String> {
        let total_len = compilation
            .into_iter()
            .fold(0, |acc, func| acc + Self::function_allocation_size(func));

        let (mut buf, registry, start, m_relocs) = self.allocate(total_len)?;
        let mut result = Vec::with_capacity(compilation.len());
        let mut start = start as u32;

        for (func, relocs) in compilation.into_iter().zip(relocations.values()) {
            let (next_start, next_buf, vmfunc) = Self::copy_function(func, start, buf, registry);

            result.push(vmfunc);

            Self::copy_relocs(m_relocs, start, relocs.iter());

            start = next_start;
            buf = next_buf;
        }

        Ok(result.into_boxed_slice())
    }

    /// Make all allocated memory executable.
    pub fn publish(&mut self, isa: &dyn TargetIsa) {
        self.push_current(0)
            .expect("failed to push current memory map");

        for CodeMemoryEntry {
            mmap: m,
            registry: r,
            relocs,
        } in &mut self.entries[self.published..]
        {
            // Remove write access to the pages due to the relocation fixups.
            r.publish(isa)
                .expect("failed to publish function unwind registry");

            if !m.is_empty() {
                unsafe {
                    region::protect(m.as_mut_ptr(), m.len(), region::Protection::ReadExecute)
                }
                .expect("unable to make memory readonly and executable");
            }

            // Relocs data in not needed anymore -- clearing.
            // TODO use relocs to serialize the published code.
            relocs.clear();
        }

        self.published = self.entries.len();
    }

    /// Allocate `size` bytes of memory which can be made executable later by
    /// calling `publish()`. Note that we allocate the memory as writeable so
    /// that it can be written to and patched, though we make it readonly before
    /// actually executing from it.
    ///
    /// A few values are returned:
    ///
    /// * A mutable slice which references the allocated memory
    /// * A function table instance where unwind information is registered
    /// * The offset within the current mmap that the slice starts at
    ///
    /// TODO: Add an alignment flag.
    fn allocate(
        &mut self,
        size: usize,
    ) -> Result<
        (
            &mut [u8],
            &mut UnwindRegistry,
            usize,
            &mut CodeMemoryRelocations,
        ),
        String,
    > {
        assert!(size > 0);

        if match &self.current {
            Some(e) => e.mmap.len() - self.position < size,
            None => true,
        } {
            self.push_current(cmp::max(0x10000, size))?;
        }

        let old_position = self.position;
        self.position += size;

        let e = self.current.as_mut().unwrap();

        Ok((
            &mut e.mmap.as_mut_slice()[old_position..self.position],
            &mut e.registry,
            old_position,
            &mut e.relocs,
        ))
    }

    /// Calculates the allocation size of the given compiled function.
    fn function_allocation_size(func: &CompiledFunction) -> usize {
        match &func.unwind_info {
            Some(UnwindInfo::WindowsX64(info)) => {
                // Windows unwind information is required to be emitted into code memory
                // This is because it must be a positive relative offset from the start of the memory
                // Account for necessary unwind information alignment padding (32-bit alignment)
                ((func.body.len() + 3) & !3) + info.emit_size()
            }
            _ => func.body.len(),
        }
    }

    fn copy_relocs<'a>(
        entry_relocs: &'_ mut CodeMemoryRelocations,
        start: u32,
        relocs: impl Iterator<Item = &'a Relocation>,
    ) {
        entry_relocs.push((start, relocs.cloned().collect()));
    }

    /// Copies the data of the compiled function to the given buffer.
    ///
    /// This will also add the function to the current unwind registry.
    fn copy_function<'a>(
        func: &CompiledFunction,
        func_start: u32,
        buf: &'a mut [u8],
        registry: &mut UnwindRegistry,
    ) -> (u32, &'a mut [u8], &'a mut [VMFunctionBody]) {
        let func_len = func.body.len();
        let mut func_end = func_start + (func_len as u32);

        let (body, mut remainder) = buf.split_at_mut(func_len);
        body.copy_from_slice(&func.body);
        let vmfunc = Self::view_as_mut_vmfunc_slice(body);

        if let Some(UnwindInfo::WindowsX64(info)) = &func.unwind_info {
            // Windows unwind information is written following the function body
            // Keep unwind information 32-bit aligned (round up to the nearest 4 byte boundary)
            let unwind_start = (func_end + 3) & !3;
            let unwind_size = info.emit_size();
            let padding = (unwind_start - func_end) as usize;

            let (slice, r) = remainder.split_at_mut(padding + unwind_size);

            info.emit(&mut slice[padding..]);

            func_end = unwind_start + (unwind_size as u32);
            remainder = r;
        }

        if let Some(info) = &func.unwind_info {
            registry
                .register(func_start, func_len as u32, info)
                .expect("failed to register unwind information");
        }

        (func_end, remainder, vmfunc)
    }

    /// Convert mut a slice from u8 to VMFunctionBody.
    fn view_as_mut_vmfunc_slice(slice: &mut [u8]) -> &mut [VMFunctionBody] {
        let byte_ptr: *mut [u8] = slice;
        let body_ptr = byte_ptr as *mut [VMFunctionBody];
        unsafe { &mut *body_ptr }
    }

    /// Pushes the current entry and allocates a new one with the given size.
    fn push_current(&mut self, new_size: usize) -> Result<(), String> {
        let previous = mem::replace(
            &mut self.current,
            if new_size == 0 {
                None
            } else {
                Some(CodeMemoryEntry::with_capacity(cmp::max(0x10000, new_size))?)
            },
        );

        if let Some(e) = previous {
            self.entries.push(e);
        }

        self.position = 0;

        Ok(())
    }

    /// Returns all published segment ranges.
    pub fn published_ranges<'a>(&'a self) -> impl Iterator<Item = (usize, usize)> + 'a {
        self.entries[..self.published]
            .iter()
            .map(|entry| entry.range())
    }

    /// Returns all relocations for the unpublished memory.
    pub fn unpublished_relocations<'a>(
        &'a self,
    ) -> impl Iterator<Item = (*const u8, &'a Relocation)> + 'a {
        self.entries[self.published..]
            .iter()
            .chain(self.current.iter())
            .flat_map(|entry| {
                entry.relocs.iter().flat_map(move |(start, relocs)| {
                    let base_ptr = unsafe { entry.mmap.as_ptr().add(*start as usize) };
                    relocs.iter().map(move |r| (base_ptr, r))
                })
            })
    }
}
