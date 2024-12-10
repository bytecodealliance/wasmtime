//! Memory management for executable code.

use crate::prelude::*;
use crate::runtime::vm::{libcalls, MmapVec, UnwindRegistration};
use crate::Engine;
use alloc::sync::Arc;
use core::ops::Range;
use object::endian::Endianness;
use object::read::{elf::ElfFile64, Object, ObjectSection};
use object::{ObjectSymbol, SectionFlags};
use wasmtime_environ::{lookup_trap_code, obj, Trap};

/// Management of executable memory within a `MmapVec`
///
/// This type consumes ownership of a region of memory and will manage the
/// executable permissions of the contained JIT code as necessary.
pub struct CodeMemory {
    mmap: MmapVec,
    unwind_registration: Option<UnwindRegistration>,
    #[cfg(feature = "debug-builtins")]
    debug_registration: Option<crate::runtime::vm::GdbJitImageRegistration>,
    published: bool,
    enable_branch_protection: bool,
    needs_executable: bool,
    #[cfg(feature = "debug-builtins")]
    has_native_debug_info: bool,
    custom_code_memory: Option<Arc<dyn CustomCodeMemory>>,

    relocations: Vec<(usize, obj::LibCall)>,

    // Ranges within `self.mmap` of where the particular sections lie.
    text: Range<usize>,
    unwind: Range<usize>,
    trap_data: Range<usize>,
    wasm_data: Range<usize>,
    address_map_data: Range<usize>,
    func_name_data: Range<usize>,
    info_data: Range<usize>,
    wasm_dwarf: Range<usize>,
}

impl Drop for CodeMemory {
    fn drop(&mut self) {
        // If there is a custom code memory handler, restore the
        // original (non-executable) state of the memory.
        if let Some(mem) = self.custom_code_memory.as_ref() {
            let text = self.text();
            mem.unpublish_executable(text.as_ptr(), text.len())
                .expect("Executable memory unpublish failed");
        }

        // Drop the registrations before `self.mmap` since they (implicitly) refer to it.
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
    fn publish_executable(&self, ptr: *const u8, len: usize) -> anyhow::Result<()>;

    /// Unpublish a region of memory.
    ///
    /// This should perform the opposite effect of `make_executable`,
    /// switching a range of memory back from RX (readable/executable)
    /// to RW (readable/writable). It is guaranteed that no code is
    /// running anymore from this region.
    ///
    /// `ptr` and `ptr.offset(len)` are guaranteed to be aligned as
    /// per `required_alignment()`.
    fn unpublish_executable(&self, ptr: *const u8, len: usize) -> anyhow::Result<()>;
}

impl CodeMemory {
    /// Creates a new `CodeMemory` by taking ownership of the provided
    /// `MmapVec`.
    ///
    /// The returned `CodeMemory` manages the internal `MmapVec` and the
    /// `publish` method is used to actually make the memory executable.
    pub fn new(engine: &Engine, mmap: MmapVec) -> Result<Self> {
        let obj = ElfFile64::<Endianness>::parse(&mmap[..])
            .map_err(obj::ObjectCrateErrorWrapper)
            .with_context(|| "failed to parse internal compilation artifact")?;

        let mut relocations = Vec::new();
        let mut text = 0..0;
        let mut unwind = 0..0;
        let mut enable_branch_protection = None;
        let mut needs_executable = true;
        #[cfg(feature = "debug-builtins")]
        let mut has_native_debug_info = false;
        let mut trap_data = 0..0;
        let mut wasm_data = 0..0;
        let mut address_map_data = 0..0;
        let mut func_name_data = 0..0;
        let mut info_data = 0..0;
        let mut wasm_dwarf = 0..0;
        for section in obj.sections() {
            let data = section.data().map_err(obj::ObjectCrateErrorWrapper)?;
            let name = section.name().map_err(obj::ObjectCrateErrorWrapper)?;
            let range = subslice_range(data, &mmap);

            // Double-check that sections are all aligned properly.
            if section.align() != 0 && data.len() != 0 {
                if (data.as_ptr() as u64 - mmap.as_ptr() as u64) % section.align() != 0 {
                    bail!(
                        "section `{}` isn't aligned to {:#x}",
                        section.name().unwrap_or("ERROR"),
                        section.align()
                    );
                }
            }

            match name {
                obj::ELF_WASM_BTI => match data.len() {
                    1 => enable_branch_protection = Some(data[0] != 0),
                    _ => bail!("invalid `{name}` section"),
                },
                ".text" => {
                    text = range;

                    if let SectionFlags::Elf { sh_flags } = section.flags() {
                        if sh_flags & obj::SH_WASMTIME_NOT_EXECUTED != 0 {
                            needs_executable = false;
                        }
                    }

                    // The text section might have relocations for things like
                    // libcalls which need to be applied, so handle those here.
                    //
                    // Note that only a small subset of possible relocations are
                    // handled. Only those required by the compiler side of
                    // things are processed.
                    for (offset, reloc) in section.relocations() {
                        assert_eq!(reloc.kind(), object::RelocationKind::Absolute);
                        assert_eq!(reloc.encoding(), object::RelocationEncoding::Generic);
                        assert_eq!(usize::from(reloc.size()), core::mem::size_of::<usize>() * 8);
                        assert_eq!(reloc.addend(), 0);
                        let sym = match reloc.target() {
                            object::RelocationTarget::Symbol(id) => id,
                            other => panic!("unknown relocation target {other:?}"),
                        };
                        let sym = obj.symbol_by_index(sym).unwrap().name().unwrap();
                        let libcall = obj::LibCall::from_str(sym)
                            .unwrap_or_else(|| panic!("unknown symbol relocation: {sym}"));

                        let offset = usize::try_from(offset).unwrap();
                        relocations.push((offset, libcall));
                    }
                }
                UnwindRegistration::SECTION_NAME => unwind = range,
                obj::ELF_WASM_DATA => wasm_data = range,
                obj::ELF_WASMTIME_ADDRMAP => address_map_data = range,
                obj::ELF_WASMTIME_TRAPS => trap_data = range,
                obj::ELF_NAME_DATA => func_name_data = range,
                obj::ELF_WASMTIME_INFO => info_data = range,
                obj::ELF_WASMTIME_DWARF => wasm_dwarf = range,
                #[cfg(feature = "debug-builtins")]
                ".debug_info" => has_native_debug_info = true,

                _ => log::debug!("ignoring section {name}"),
            }
        }

        Ok(Self {
            mmap,
            unwind_registration: None,
            #[cfg(feature = "debug-builtins")]
            debug_registration: None,
            published: false,
            enable_branch_protection: enable_branch_protection
                .ok_or_else(|| anyhow!("missing `{}` section", obj::ELF_WASM_BTI))?,
            needs_executable,
            #[cfg(feature = "debug-builtins")]
            has_native_debug_info,
            custom_code_memory: engine.custom_code_memory().cloned(),
            text,
            unwind,
            trap_data,
            address_map_data,
            func_name_data,
            wasm_dwarf,
            info_data,
            wasm_data,
            relocations,
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
    /// This method can only be called once and will panic if called twice. This
    /// will parse the ELF image from the original `MmapVec` and do everything
    /// necessary to get it ready for execution, including:
    ///
    /// * Change page protections from read/write to read/execute.
    /// * Register unwinding information with the OS
    /// * Register this image with the debugger if native DWARF is present
    ///
    /// After this function executes all JIT code should be ready to execute.
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
            // First, if necessary, apply relocations. This can happen for
            // things like libcalls which happen late in the lowering process
            // that don't go through the Wasm-based libcalls layer that's
            // indirected through the `VMContext`. Note that most modules won't
            // have relocations, so this typically doesn't do anything.
            self.apply_relocations()?;

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
            #[cfg(feature = "signals-based-traps")]
            self.mmap.make_readonly(0..self.mmap.len())?;

            // Switch the executable portion from readonly to read/execute.
            if self.needs_executable {
                if !self.custom_publish()? {
                    #[cfg(feature = "signals-based-traps")]
                    {
                        let text = self.text();

                        use wasmtime_jit_icache_coherence as icache_coherence;

                        // Clear the newly allocated code from cache if the processor requires it
                        //
                        // Do this before marking the memory as R+X, technically we should be able to do it after
                        // but there are some CPU's that have had errata about doing this with read only memory.
                        icache_coherence::clear_cache(text.as_ptr().cast(), text.len())
                            .expect("Failed cache clear");

                        self.mmap
                            .make_executable(self.text.clone(), self.enable_branch_protection)
                            .context("unable to make memory executable")?;

                        // Flush any in-flight instructions from the pipeline
                        icache_coherence::pipeline_flush_mt().expect("Failed pipeline flush");
                    }
                    #[cfg(not(feature = "signals-based-traps"))]
                    bail!("this target requires virtual memory to be enabled");
                }
            }

            // With all our memory set up use the platform-specific
            // `UnwindRegistration` implementation to inform the general
            // runtime that there's unwinding information available for all
            // our just-published JIT functions.
            self.register_unwind_info()?;

            #[cfg(feature = "debug-builtins")]
            self.register_debug_image()?;
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

    unsafe fn apply_relocations(&mut self) -> Result<()> {
        if self.relocations.is_empty() {
            return Ok(());
        }

        for (offset, libcall) in self.relocations.iter() {
            let offset = self.text.start + offset;
            let libcall = match libcall {
                obj::LibCall::FloorF32 => libcalls::relocs::floorf32 as usize,
                obj::LibCall::FloorF64 => libcalls::relocs::floorf64 as usize,
                obj::LibCall::NearestF32 => libcalls::relocs::nearestf32 as usize,
                obj::LibCall::NearestF64 => libcalls::relocs::nearestf64 as usize,
                obj::LibCall::CeilF32 => libcalls::relocs::ceilf32 as usize,
                obj::LibCall::CeilF64 => libcalls::relocs::ceilf64 as usize,
                obj::LibCall::TruncF32 => libcalls::relocs::truncf32 as usize,
                obj::LibCall::TruncF64 => libcalls::relocs::truncf64 as usize,
                obj::LibCall::FmaF32 => libcalls::relocs::fmaf32 as usize,
                obj::LibCall::FmaF64 => libcalls::relocs::fmaf64 as usize,
                #[cfg(target_arch = "x86_64")]
                obj::LibCall::X86Pshufb => libcalls::relocs::x86_pshufb as usize,
                #[cfg(not(target_arch = "x86_64"))]
                obj::LibCall::X86Pshufb => unreachable!(),
            };
            self.mmap
                .as_mut_slice()
                .as_mut_ptr()
                .add(offset)
                .cast::<usize>()
                .write_unaligned(libcall);
        }
        Ok(())
    }

    unsafe fn register_unwind_info(&mut self) -> Result<()> {
        if self.unwind.len() == 0 {
            return Ok(());
        }
        let text = self.text();
        let unwind_info = &self.mmap[self.unwind.clone()];
        let registration =
            UnwindRegistration::new(text.as_ptr(), unwind_info.as_ptr(), unwind_info.len())
                .context("failed to create unwind info registration")?;
        self.unwind_registration = Some(registration);
        Ok(())
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
        let bytes = crate::debug::create_gdbjit_image(image, (text.as_ptr(), text.len()))?;
        let reg = crate::runtime::vm::GdbJitImageRegistration::register(bytes);
        self.debug_registration = Some(reg);
        Ok(())
    }

    /// Looks up the given offset within this module's text section and returns
    /// the trap code associated with that instruction, if there is one.
    pub fn lookup_trap_code(&self, text_offset: usize) -> Option<Trap> {
        lookup_trap_code(self.trap_data(), text_offset)
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
