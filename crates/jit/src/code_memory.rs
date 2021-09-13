//! Memory management for executable code.

use crate::unwind::UnwindRegistration;
use crate::MmapVec;
use anyhow::{bail, Context, Result};
use object::read::{File, Object, ObjectSection};
use std::mem::ManuallyDrop;

/// Management of executable memory within a `MmapVec`
///
/// This type consumes ownership of a region of memory and will manage the
/// executable permissions of the contained JIT code as necessary.
pub struct CodeMemory {
    // NB: these are `ManuallyDrop` because `unwind_registration` must be
    // dropped first since it refers to memory owned by `mmap`.
    mmap: ManuallyDrop<MmapVec>,
    unwind_registration: ManuallyDrop<Option<UnwindRegistration>>,
    published: bool,
}

impl Drop for CodeMemory {
    fn drop(&mut self) {
        // Drop `unwind_registration` before `self.mmap`
        unsafe {
            ManuallyDrop::drop(&mut self.unwind_registration);
            ManuallyDrop::drop(&mut self.mmap);
        }
    }
}

fn _assert() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<CodeMemory>();
}

/// Result of publishing a `CodeMemory`, containing references to the parsed
/// internals.
pub struct Publish<'a> {
    /// The parsed ELF image that resides within the original `MmapVec`.
    pub obj: File<'a>,

    /// Reference to the entire `MmapVec` and its contents.
    pub mmap: &'a [u8],

    /// Reference to just the text section of the object file, a subslice of
    /// `mmap`.
    pub text: &'a [u8],
}

impl CodeMemory {
    /// Creates a new `CodeMemory` by taking ownership of the provided
    /// `MmapVec`.
    ///
    /// The returned `CodeMemory` manages the internal `MmapVec` and the
    /// `publish` method is used to actually make the memory executable.
    pub fn new(mmap: MmapVec) -> Self {
        Self {
            mmap: ManuallyDrop::new(mmap),
            unwind_registration: ManuallyDrop::new(None),
            published: false,
        }
    }

    /// Returns a reference to the underlying `MmapVec` this memory owns.
    pub fn mmap(&self) -> &MmapVec {
        &self.mmap
    }

    /// Publishes the internal ELF image to be ready for execution.
    ///
    /// This method can only be called once and will panic if called twice. This
    /// will parse the ELF image from the original `MmapVec` and do everything
    /// necessary to get it ready for execution, including:
    ///
    /// * Change page protections from read/write to read/execute.
    /// * Register unwinding information with the OS
    ///
    /// After this function executes all JIT code should be ready to execute.
    /// The various parsed results of the internals of the `MmapVec` are
    /// returned through the `Publish` structure.
    pub fn publish(&mut self) -> Result<Publish<'_>> {
        assert!(!self.published);
        self.published = true;

        let mut ret = Publish {
            obj: File::parse(&self.mmap[..])
                .with_context(|| "failed to parse internal compilation artifact")?,
            mmap: &self.mmap,
            text: &[],
        };
        let mmap_ptr = self.mmap.as_ptr() as u64;

        // Sanity-check that all sections are aligned correctly.
        for section in ret.obj.sections() {
            let data = match section.data() {
                Ok(data) => data,
                Err(_) => continue,
            };
            if section.align() == 0 || data.len() == 0 {
                continue;
            }
            if (data.as_ptr() as u64 - mmap_ptr) % section.align() != 0 {
                bail!(
                    "section `{}` isn't aligned to {:#x}",
                    section.name().unwrap_or("ERROR"),
                    section.align()
                );
            }
        }

        // Find the `.text` section with executable code in it.
        let text = match ret.obj.section_by_name(".text") {
            Some(section) => section,
            None => return Ok(ret),
        };
        ret.text = match text.data() {
            Ok(data) if !data.is_empty() => data,
            _ => return Ok(ret),
        };

        // The unsafety here comes from a few things:
        //
        // * First in `apply_reloc` we're walking around the `File` that the
        //   `object` crate has to get a mutable view into the text section.
        //   Currently the `object` crate doesn't support easily parsing a file
        //   and updating small bits and pieces of it, so we work around it for
        //   now. ELF's file format should guarantee that `text_mut` doesn't
        //   collide with any memory accessed by `text.relocations()`.
        //
        // * Second we're actually updating some page protections to executable
        //   memory.
        //
        // * Finally we're registering unwinding information which relies on the
        //   correctness of the information in the first place. This applies to
        //   both the actual unwinding tables as well as the validity of the
        //   pointers we pass in itself.
        unsafe {
            let text_mut =
                std::slice::from_raw_parts_mut(ret.text.as_ptr() as *mut u8, ret.text.len());
            let text_offset = ret.text.as_ptr() as usize - ret.mmap.as_ptr() as usize;
            let text_range = text_offset..text_offset + text_mut.len();
            let mut text_section_readwrite = false;
            for (offset, r) in text.relocations() {
                // If the text section was mapped at readonly we need to make it
                // briefly read/write here as we apply relocations.
                if !text_section_readwrite && self.mmap.is_readonly() {
                    self.mmap
                        .make_writable(text_range.clone())
                        .expect("unable to make memory writable");
                    text_section_readwrite = true;
                }
                crate::link::apply_reloc(&ret.obj, text_mut, offset, r);
            }

            // Switch the executable portion from read/write to
            // read/execute, notably not using read/write/execute to prevent
            // modifications.
            self.mmap
                .make_executable(text_range.clone())
                .expect("unable to make memory executable");

            // With all our memory set up use the platform-specific
            // `UnwindRegistration` implementation to inform the general
            // runtime that there's unwinding information available for all
            // our just-published JIT functions.
            *self.unwind_registration = register_unwind_info(&ret.obj, ret.text)?;
        }

        Ok(ret)
    }
}

unsafe fn register_unwind_info(obj: &File, text: &[u8]) -> Result<Option<UnwindRegistration>> {
    let unwind_info = match obj
        .section_by_name(UnwindRegistration::section_name())
        .and_then(|s| s.data().ok())
    {
        Some(info) => info,
        None => return Ok(None),
    };
    if unwind_info.len() == 0 {
        return Ok(None);
    }
    Ok(Some(
        UnwindRegistration::new(
            text.as_ptr() as *mut _,
            unwind_info.as_ptr() as *mut _,
            unwind_info.len(),
        )
        .context("failed to create unwind info registration")?,
    ))
}
