//! Memory management for executable code.

use crate::Engine;
use crate::prelude::*;
use crate::runtime::vm::MmapVec;
use alloc::sync::Arc;
use core::ops::Range;
use object::{
    elf::FileHeader64,
    endian::Endianness,
    read::elf::{FileHeader as _, SectionHeader as _},
};
use wasmtime_environ::{Trap, lookup_trap_code, obj};
use wasmtime_unwinder::ExceptionTable;

/// Management of executable memory within a `MmapVec`
///
/// This type consumes ownership of a region of memory and will manage the
/// executable permissions of the contained JIT code as necessary.
pub struct CodeMemory {
    mmap: MmapVec,
    #[cfg(has_host_compiler_backend)]
    unwind_registration: Option<crate::runtime::vm::UnwindRegistration>,
    #[cfg(feature = "debug-builtins")]
    debug_registration: Option<crate::runtime::vm::GdbJitImageRegistration>,
    published: bool,
    registered: bool,
    enable_branch_protection: bool,
    needs_executable: bool,
    #[cfg(feature = "debug-builtins")]
    has_native_debug_info: bool,
    custom_code_memory: Option<Arc<dyn CustomCodeMemory>>,

    // Ranges within `self.mmap` of where the particular sections lie.
    text: Range<usize>,
    unwind: Range<usize>,
    trap_data: Range<usize>,
    wasm_data: Range<usize>,
    address_map_data: Range<usize>,
    stack_map_data: Range<usize>,
    exception_data: Range<usize>,
    frame_tables_data: Range<usize>,
    func_name_data: Range<usize>,
    info_data: Range<usize>,
    wasm_dwarf: Range<usize>,
}

impl Drop for CodeMemory {
    fn drop(&mut self) {
        // If there is a custom code memory handler, restore the
        // original (non-executable) state of the memory.
        //
        // We do this rather than invoking `unpublish()` because we
        // want to skip the mprotect() if we natively own the mmap and
        // are going to munmap soon anyway.
        if let Some(mem) = self.custom_code_memory.as_ref() {
            if self.published && self.needs_executable {
                let text = self.text();
                mem.unpublish_executable(text.as_ptr(), text.len())
                    .expect("Executable memory unpublish failed");
            }
        }

        // Drop the registrations before `self.mmap` since they (implicitly) refer to it.
        #[cfg(has_host_compiler_backend)]
        let _ = self.unwind_registration.take();
        #[cfg(feature = "debug-builtins")]
        let _ = self.debug_registration.take();
    }
}

fn _assert() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<CodeMemory>();
}

/// Interface implemented by an embedder to provide custom
/// implementations of code-memory protection and execute permissions.
pub trait CustomCodeMemory: Send + Sync {
    /// The minimal alignment granularity for an address region that
    /// can be made executable.
    ///
    /// Wasmtime does not assume the system page size for this because
    /// custom code-memory protection can be used when all other uses
    /// of virtual memory are disabled.
    fn required_alignment(&self) -> usize;

    /// Publish a region of memory as executable.
    ///
    /// This should update permissions from the default RW
    /// (readable/writable but not executable) to RX
    /// (readable/executable but not writable), enforcing W^X
    /// discipline.
    ///
    /// If the platform requires any data/instruction coherence
    /// action, that should be performed as part of this hook as well.
    ///
    /// `ptr` and `ptr.offset(len)` are guaranteed to be aligned as
    /// per `required_alignment()`.
    fn publish_executable(&self, ptr: *const u8, len: usize) -> crate::Result<()>;

    /// Unpublish a region of memory.
    ///
    /// This should perform the opposite effect of `make_executable`,
    /// switching a range of memory back from RX (readable/executable)
    /// to RW (readable/writable). It is guaranteed that no code is
    /// running anymore from this region.
    ///
    /// `ptr` and `ptr.offset(len)` are guaranteed to be aligned as
    /// per `required_alignment()`.
    fn unpublish_executable(&self, ptr: *const u8, len: usize) -> crate::Result<()>;
}

impl CodeMemory {
    /// Creates a new `CodeMemory` by taking ownership of the provided
    /// `MmapVec`.
    ///
    /// The returned `CodeMemory` manages the internal `MmapVec` and the
    /// `publish` method is used to actually make the memory executable.
    pub fn new(engine: &Engine, mmap: MmapVec) -> Result<Self> {
        let mmap_data = &*mmap;
        let header = FileHeader64::<Endianness>::parse(mmap_data)
            .map_err(obj::ObjectCrateErrorWrapper)
            .context("failed to parse precompiled artifact as an ELF")?;
        let endian = header
            .endian()
            .context("failed to parse header endianness")?;

        let section_headers = header
            .section_headers(endian, mmap_data)
            .context("failed to parse section headers")?;
        let strings = header
            .section_strings(endian, mmap_data, section_headers)
            .context("failed to parse strings table")?;
        let sections = header
            .sections(endian, mmap_data)
            .context("failed to parse sections table")?;

        let mut text = 0..0;
        let mut unwind = 0..0;
        let mut enable_branch_protection = None;
        let mut needs_executable = true;
        #[cfg(feature = "debug-builtins")]
        let mut has_native_debug_info = false;
        let mut trap_data = 0..0;
        let mut exception_data = 0..0;
        let mut frame_tables_data = 0..0;
        let mut wasm_data = 0..0;
        let mut address_map_data = 0..0;
        let mut stack_map_data = 0..0;
        let mut func_name_data = 0..0;
        let mut info_data = 0..0;
        let mut wasm_dwarf = 0..0;
        for section_header in sections.iter() {
            let data = section_header
                .data(endian, mmap_data)
                .map_err(obj::ObjectCrateErrorWrapper)?;
            let name = section_header
                .name(endian, strings)
                .map_err(obj::ObjectCrateErrorWrapper)?;
            let Ok(name) = str::from_utf8(name) else {
                log::debug!("ignoring section with invalid UTF-8 name");
                continue;
            };
            let range = subslice_range(data, &mmap);

            // Double-check that sections are all aligned properly.
            let section_align = usize::try_from(section_header.sh_addralign(endian))?;
            if section_align != 0 && data.len() != 0 {
                ensure!(
                    data.as_ptr().addr() % section_align == 0,
                    "section `{name}` isn't aligned to {section_align:#x}",
                );
            }

            // Assert that Cranelift hasn't inserted any calls that need to be
            // relocated. We avoid using things like Cranelift's floor/ceil/etc.
            // operators in the Wasm-to-Cranelift translator specifically to
            // avoid having to do any relocations here. This also ensures that
            // all builtins use the same trampoline mechanism.
            let sh_type = section_header.sh_type(endian);
            assert!(!matches!(
                sh_type,
                object::elf::SHT_REL | object::elf::SHT_RELA | object::elf::SHT_CREL
            ));

            match name {
                obj::ELF_WASM_BTI => match data.len() {
                    1 => enable_branch_protection = Some(data[0] != 0),
                    _ => bail!("invalid `{name}` section"),
                },
                ".text" => {
                    text = range;

                    if section_header.sh_flags(endian) & obj::SH_WASMTIME_NOT_EXECUTED != 0 {
                        needs_executable = false;
                    }
                }
                #[cfg(has_host_compiler_backend)]
                crate::runtime::vm::UnwindRegistration::SECTION_NAME => unwind = range,
                obj::ELF_WASM_DATA => wasm_data = range,
                obj::ELF_WASMTIME_ADDRMAP => address_map_data = range,
                obj::ELF_WASMTIME_STACK_MAP => stack_map_data = range,
                obj::ELF_WASMTIME_TRAPS => trap_data = range,
                obj::ELF_WASMTIME_EXCEPTIONS => exception_data = range,
                obj::ELF_WASMTIME_FRAMES => frame_tables_data = range,
                obj::ELF_NAME_DATA => func_name_data = range,
                obj::ELF_WASMTIME_INFO => info_data = range,
                obj::ELF_WASMTIME_DWARF => wasm_dwarf = range,
                #[cfg(feature = "debug-builtins")]
                ".debug_info" => has_native_debug_info = true,

                _ => log::debug!("ignoring section {name}"),
            }
        }

        // require mutability even when this is turned off
        #[cfg(not(has_host_compiler_backend))]
        let _ = &mut unwind;

        // Ensure that the exception table is well-formed. This parser
        // construction is cheap: it reads the header and validates
        // ranges but nothing else. We do this only in debug-assertion
        // builds because we otherwise require for safety that the
        // compiled artifact is as-produced-by this version of
        // Wasmtime, and we should always produce a correct exception
        // table (i.e., we are not expecting untrusted data here).
        #[cfg(debug_assertions)]
        let _ = ExceptionTable::parse(&mmap[exception_data.clone()])?;

        Ok(Self {
            mmap,
            #[cfg(has_host_compiler_backend)]
            unwind_registration: None,
            #[cfg(feature = "debug-builtins")]
            debug_registration: None,
            published: false,
            registered: false,
            enable_branch_protection: enable_branch_protection
                .ok_or_else(|| format_err!("missing `{}` section", obj::ELF_WASM_BTI))?,
            needs_executable,
            #[cfg(feature = "debug-builtins")]
            has_native_debug_info,
            custom_code_memory: engine.custom_code_memory().cloned(),
            text,
            unwind,
            trap_data,
            address_map_data,
            stack_map_data,
            exception_data,
            frame_tables_data,
            func_name_data,
            wasm_dwarf,
            info_data,
            wasm_data,
        })
    }

    /// Returns a reference to the underlying `MmapVec` this memory owns.
    #[inline]
    pub fn mmap(&self) -> &MmapVec {
        &self.mmap
    }

    /// Returns the contents of the text section of the ELF executable this
    /// represents.
    #[inline]
    pub fn text(&self) -> &[u8] {
        &self.mmap[self.text.clone()]
    }

    /// Returns the contents of the `ELF_WASMTIME_DWARF` section.
    #[inline]
    pub fn wasm_dwarf(&self) -> &[u8] {
        &self.mmap[self.wasm_dwarf.clone()]
    }

    /// Returns the data in the `ELF_NAME_DATA` section.
    #[inline]
    pub fn func_name_data(&self) -> &[u8] {
        &self.mmap[self.func_name_data.clone()]
    }

    /// Returns the concatenated list of all data associated with this wasm
    /// module.
    ///
    /// This is used for initialization of memories and all data ranges stored
    /// in a `Module` are relative to the slice returned here.
    #[inline]
    pub fn wasm_data(&self) -> &[u8] {
        &self.mmap[self.wasm_data.clone()]
    }

    /// Returns the encoded address map section used to pass to
    /// `wasmtime_environ::lookup_file_pos`.
    #[inline]
    pub fn address_map_data(&self) -> &[u8] {
        &self.mmap[self.address_map_data.clone()]
    }

    /// Returns the encoded stack map section used to pass to
    /// `wasmtime_environ::StackMap::lookup`.
    pub fn stack_map_data(&self) -> &[u8] {
        &self.mmap[self.stack_map_data.clone()]
    }

    /// Returns the encoded exception-tables section to pass to
    /// `wasmtime_unwinder::ExceptionTable::parse`.
    pub fn exception_tables(&self) -> &[u8] {
        &self.mmap[self.exception_data.clone()]
    }

    /// Returns the encoded frame-tables section to pass to
    /// `wasmtime_environ::FrameTable::parse`.
    pub fn frame_tables(&self) -> &[u8] {
        &self.mmap[self.frame_tables_data.clone()]
    }

    /// Returns the contents of the `ELF_WASMTIME_INFO` section, or an empty
    /// slice if it wasn't found.
    #[inline]
    pub fn wasmtime_info(&self) -> &[u8] {
        &self.mmap[self.info_data.clone()]
    }

    /// Returns the contents of the `ELF_WASMTIME_TRAPS` section, or an empty
    /// slice if it wasn't found.
    #[inline]
    pub fn trap_data(&self) -> &[u8] {
        &self.mmap[self.trap_data.clone()]
    }

    /// Publishes the internal ELF image to be ready for execution.
    ///
    /// This method can only be when the image is not published (its
    /// default state) and will panic if called when already
    /// published. This will parse the ELF image from the original
    /// `MmapVec` and do everything necessary to get it ready for
    /// execution, including:
    ///
    /// * Change page protections from read/write to read/execute.
    /// * Register unwinding information with the OS
    /// * Register this image with the debugger if native DWARF is present
    ///
    /// After this function executes all JIT code should be ready to execute.
    ///
    /// The action may be reversed by calling [`Self::unpublish`], as long
    /// as that method's safety requirements are upheld.
    pub fn publish(&mut self) -> Result<()> {
        assert!(!self.published);
        self.published = true;

        if self.text().is_empty() {
            return Ok(());
        }

        // The unsafety here comes from a few things:
        //
        // * We're actually updating some page protections to executable memory.
        //
        // * We're registering unwinding information which relies on the
        //   correctness of the information in the first place. This applies to
        //   both the actual unwinding tables as well as the validity of the
        //   pointers we pass in itself.
        unsafe {
            // Next freeze the contents of this image by making all of the
            // memory readonly. Nothing after this point should ever be modified
            // so commit everything. For a compiled-in-memory image this will
            // mean IPIs to evict writable mappings from other cores. For
            // loaded-from-disk images this shouldn't result in IPIs so long as
            // there weren't any relocations because nothing should have
            // otherwise written to the image at any point either.
            //
            // Note that if virtual memory is disabled this is skipped because
            // we aren't able to make it readonly, but this is just a
            // defense-in-depth measure and isn't required for correctness.
            #[cfg(has_virtual_memory)]
            if self.mmap.supports_virtual_memory() {
                self.mmap.make_readonly(0..self.mmap.len())?;
            }

            // Switch the executable portion from readonly to read/execute.
            if self.needs_executable {
                if !self.custom_publish()? {
                    if !self.mmap.supports_virtual_memory() {
                        bail!("this target requires virtual memory to be enabled");
                    }
                    #[cfg(has_virtual_memory)]
                    self.mmap
                        .make_executable(self.text.clone(), self.enable_branch_protection)
                        .context("unable to make memory executable")?;
                }
            }

            if !self.registered {
                // With all our memory set up use the platform-specific
                // `UnwindRegistration` implementation to inform the general
                // runtime that there's unwinding information available for all
                // our just-published JIT functions.
                self.register_unwind_info()?;

                #[cfg(feature = "debug-builtins")]
                self.register_debug_image()?;
                self.registered = true;
            }
        }

        Ok(())
    }

    fn custom_publish(&mut self) -> Result<bool> {
        if let Some(mem) = self.custom_code_memory.as_ref() {
            let text = self.text();
            // The text section should be aligned to
            // `custom_code_memory.required_alignment()` due to a
            // combination of two invariants:
            //
            // - MmapVec aligns its start address, even in owned-Vec mode; and
            // - The text segment inside the ELF image will be aligned according
            //   to the platform's requirements.
            let text_addr = text.as_ptr() as usize;
            assert_eq!(text_addr & (mem.required_alignment() - 1), 0);

            // The custom code memory handler will ensure the
            // memory is executable and also handle icache
            // coherence.
            mem.publish_executable(text.as_ptr(), text.len())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// "Unpublish" code memory (transition it from executable to read/writable).
    ///
    /// This may be used to edit the code image, as long as the
    /// overall size of the memory remains the same. Note the hazards
    /// inherent in editing code that may have been executed: any
    /// stack frames with PC still active in this code must be
    /// suspended (e.g., called into a hostcall that is then invoking
    /// this method, or async-yielded) and any active PC values must
    /// point to valid instructions. Thus this is mostly useful for
    /// patching in-place at particular sites, such as by the use of
    /// Cranelift's `patchable_call` instruction.
    ///
    /// If this fails, then the memory remains executable.
    pub fn unpublish(&mut self) -> Result<()> {
        assert!(self.published);
        self.published = false;

        if self.text().is_empty() {
            return Ok(());
        }

        if self.custom_unpublish()? {
            return Ok(());
        }

        if !self.mmap.supports_virtual_memory() {
            bail!("this target requires virtual memory to be enabled");
        }

        // SAFETY: we are guaranteed by our own safety conditions that
        // we have exclusive access to this code and can change its
        // permissions (removing the execute bit) without causing
        // problems.
        #[cfg(has_virtual_memory)]
        unsafe {
            self.mmap.make_readwrite(0..self.mmap.len())?;
        }

        // Note that we do *not* unregister: we expect unpublish
        // to be used for temporary edits, so we want the
        // registration to "stick" after the initial publish and
        // not toggle in subsequent unpublish/publish cycles.

        Ok(())
    }

    fn custom_unpublish(&mut self) -> Result<bool> {
        if let Some(mem) = self.custom_code_memory.as_ref() {
            let text = self.text();
            mem.unpublish_executable(text.as_ptr(), text.len())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Return a mutable borrow to the code, suitable for editing.
    ///
    /// Must not be published.
    ///
    /// # Panics
    ///
    /// This method panics if the code has been published (and not
    /// subsequently unpublished).
    pub fn text_mut(&mut self) -> &mut [u8] {
        assert!(!self.published);
        // SAFETY: we assert !published, which means we either have
        // not yet applied readonly + execute permissinos, or we have
        // undone that and flipped back to read-write via unpublish.
        unsafe { &mut self.mmap.as_mut_slice()[self.text.clone()] }
    }

    unsafe fn register_unwind_info(&mut self) -> Result<()> {
        if self.unwind.len() == 0 {
            return Ok(());
        }
        #[cfg(has_host_compiler_backend)]
        {
            let text = self.text();
            let unwind_info = &self.mmap[self.unwind.clone()];
            let registration = unsafe {
                crate::runtime::vm::UnwindRegistration::new(
                    text.as_ptr(),
                    unwind_info.as_ptr(),
                    unwind_info.len(),
                )
                .context("failed to create unwind info registration")?
            };
            self.unwind_registration = Some(registration);
            return Ok(());
        }
        #[cfg(not(has_host_compiler_backend))]
        {
            bail!("should not have unwind info for non-native backend")
        }
    }

    #[cfg(feature = "debug-builtins")]
    fn register_debug_image(&mut self) -> Result<()> {
        if !self.has_native_debug_info {
            return Ok(());
        }

        // TODO-DebugInfo: we're copying the whole image here, which is pretty wasteful.
        // Use the existing memory by teaching code here about relocations in DWARF sections
        // and anything else necessary that is done in "create_gdbjit_image" right now.
        let image = self.mmap().to_vec();
        let text: &[u8] = self.text();
        let bytes = crate::native_debug::create_gdbjit_image(image, (text.as_ptr(), text.len()))?;
        let reg = crate::runtime::vm::GdbJitImageRegistration::register(bytes);
        self.debug_registration = Some(reg);
        Ok(())
    }

    /// Looks up the given offset within this module's text section and returns
    /// the trap code associated with that instruction, if there is one.
    pub fn lookup_trap_code(&self, text_offset: usize) -> Option<Trap> {
        lookup_trap_code(self.trap_data(), text_offset)
    }

    /// Get the raw address range of this CodeMemory.
    pub(crate) fn raw_addr_range(&self) -> Range<usize> {
        let start = self.text().as_ptr().addr();
        let end = start + self.text().len();
        start..end
    }

    /// Create a "deep clone": a separate CodeMemory for the same code
    /// that can be patched or mutated independently. Also returns a
    /// "metadata and location" handle that can be registered with the
    /// global module registry and used for trap metadata lookups.
    #[cfg(feature = "debug")]
    pub(crate) fn deep_clone(self: &Arc<Self>, engine: &Engine) -> Result<CodeMemory> {
        let mmap = self.mmap.deep_clone()?;
        Self::new(engine, mmap)
    }
}

/// Returns the range of `inner` within `outer`, such that `outer[range]` is the
/// same as `inner`.
///
/// This method requires that `inner` is a sub-slice of `outer`, and if that
/// isn't true then this method will panic.
fn subslice_range(inner: &[u8], outer: &[u8]) -> Range<usize> {
    if inner.len() == 0 {
        return 0..0;
    }

    assert!(outer.as_ptr() <= inner.as_ptr());
    assert!((&inner[inner.len() - 1] as *const _) <= (&outer[outer.len() - 1] as *const _));

    let start = inner.as_ptr() as usize - outer.as_ptr() as usize;
    start..start + inner.len()
}
