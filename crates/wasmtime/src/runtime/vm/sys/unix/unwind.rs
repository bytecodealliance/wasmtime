//! Module for System V ABI unwind registry.

use crate::prelude::*;
use crate::runtime::vm::SendSyncPtr;
use crate::sync::OnceLock;
use core::ptr::NonNull;

type UnwindFn = unsafe extern "C" fn(usize);

/// Represents a registration of function unwind information for System V ABI.
pub enum UnwindRegistration {
    /// libgcc registers the whole table with one call to `__register_frame`.
    SingleFrame(SendSyncPtr<u8>),
    /// Older libunwind versions register each FDE separately.
    MultipleFrames(TryVec<SendSyncPtr<u8>>),
    /// Newer libunwind versions can register an entire `.eh_frame` section.
    LibunwindSection {
        section: SendSyncPtr<u8>,
        deregister: UnwindFn,
    },
}

// Keep this layout in sync with `struct Libunwind` in helpers.c.
#[repr(C)]
#[derive(Default)]
struct Libunwind {
    add_dynamic_fde: Option<UnwindFn>,
    add_dynamic_eh_frame_section: Option<UnwindFn>,
    remove_dynamic_eh_frame_section: Option<UnwindFn>,
}

cfg_select! {
    // FIXME: at least on the `gcc-arm-linux-gnueabihf` toolchain on Ubuntu
    // these symbols are not provided by default like they are on other targets.
    // I'm not ARM expert so I don't know why. For now though consider this an
    // optional integration feature with the platform and stub out the functions
    // to do nothing which won't break any tests it just means that
    // runtime-generated backtraces won't have the same level of fidelity they
    // do on other targets.
    target_arch = "arm" => {
        unsafe extern "C" fn __register_frame(_: *const u8) {}
        unsafe extern "C" fn __deregister_frame(_: *const u8) {}
        unsafe fn wasmtime_libunwind() -> Libunwind {
            Libunwind::default()
        }
    }
    _ => {
        unsafe extern "C" {
            fn __register_frame(fde: *const u8);
            fn __deregister_frame(fde: *const u8);
            #[wasmtime_versioned_export_macros::versioned_link]
            fn wasmtime_libunwind() -> Libunwind;
        }
    }
}

impl Libunwind {
    fn get() -> &'static Self {
        static LIBUNWIND: OnceLock<Libunwind> = OnceLock::new();
        LIBUNWIND.get_or_init(|| {
            // SAFETY: the C helper returns nullable function pointers with the
            // signatures and layout declared above.
            let api = unsafe { wasmtime_libunwind() };
            #[cfg(target_os = "macos")]
            let api = {
                let mut api = api;
                if let Some((add, remove)) = load_unwind_section_api() {
                    api.add_dynamic_eh_frame_section = Some(add);
                    api.remove_dynamic_eh_frame_section = Some(remove);
                }
                api
            };
            api
        })
    }

    /// There are two primary unwinders on Unix platforms: libunwind and libgcc.
    ///
    /// Unfortunately their interface to `__register_frame` is different. The
    /// libunwind library takes a pointer to an individual FDE while libgcc takes a
    /// null-terminated list of FDEs. This means we need to know what unwinder
    /// is being used at runtime.
    ///
    /// This detection is done currently by looking for a libunwind-specific symbol.
    /// This specific symbol was somewhat recommended by LLVM's
    /// "RTDyldMemoryManager.cpp" file which says:
    ///
    /// > We use the presence of __unw_add_dynamic_fde to detect libunwind.
    ///
    /// I'll note that there's also a different libunwind project at
    /// https://www.nongnu.org/libunwind/ but that doesn't appear to have
    /// `__register_frame` so I don't think that interacts with this.
    fn is_libunwind(&self) -> bool {
        // macOS always uses the libunwind convention.
        cfg!(target_os = "macos") || self.add_dynamic_fde.is_some()
    }
}

#[cfg(target_os = "macos")]
fn load_unwind_section_api() -> Option<(UnwindFn, UnwindFn)> {
    // Weak imports tolerate symbols missing at runtime, but the linker still
    // needs their exports in the build SDK. The macOS 11.3 SDK has the older
    // __unw_add_dynamic_fde export but lacks the section APIs, so resolve those
    // at runtime to keep building with older SDKs.
    //
    // SAFETY: dladdr initializes info on success. The returned path is a C
    // string owned by the loaded image containing our linked registration API.
    unsafe {
        let mut info = core::mem::MaybeUninit::<libc::Dl_info>::uninit();
        if libc::dladdr((__register_frame as *const ()).cast(), info.as_mut_ptr()) == 0 {
            return None;
        }
        let info = info.assume_init();
        if info.dli_fname.is_null() {
            return None;
        }

        // Use the same provider as the linked individual-FDE API. RTLD_FIRST
        // prevents borrowing a missing API from a dependency's separate registry.
        let lib = libc::dlopen(
            info.dli_fname,
            libc::RTLD_LAZY | libc::RTLD_LOCAL | libc::RTLD_FIRST,
        );
        if lib.is_null() {
            return None;
        }

        // Search overrides or interposition can change which library is opened.
        if libc::dlsym(lib, c"__register_frame".as_ptr())
            != __register_frame as *const () as *mut libc::c_void
            || libc::dlsym(lib, c"__deregister_frame".as_ptr())
                != __deregister_frame as *const () as *mut libc::c_void
        {
            libc::dlclose(lib);
            return None;
        }
        let add = libc::dlsym(lib, c"__unw_add_dynamic_eh_frame_section".as_ptr());
        let remove = libc::dlsym(lib, c"__unw_remove_dynamic_eh_frame_section".as_ptr());
        if add.is_null() || remove.is_null() {
            libc::dlclose(lib);
            return None;
        }

        // Both symbols have the UnwindFn ABI. Keep the library handle open for
        // the lifetime of the function pointers cached in Libunwind::get.
        Some((
            core::mem::transmute::<*mut libc::c_void, UnwindFn>(add),
            core::mem::transmute::<*mut libc::c_void, UnwindFn>(remove),
        ))
    }
}

impl UnwindRegistration {
    pub const SECTION_NAME: &'static str = ".eh_frame";

    /// Registers precompiled unwinding information with the system.
    ///
    /// The `_base_address` field is ignored here (only used on other
    /// platforms), but the `unwind_info` and `unwind_len` parameters should
    /// describe an in-memory representation of a `.eh_frame` section. This is
    /// typically arranged for by the `wasmtime-obj` crate.
    pub unsafe fn new(
        _base_address: *const u8,
        unwind_info: *const u8,
        unwind_len: usize,
    ) -> Result<UnwindRegistration> {
        #[cfg(has_virtual_memory)]
        debug_assert_eq!(
            unwind_info as usize % crate::runtime::vm::host_page_size(),
            0,
            "The unwind info must always be aligned to a page"
        );

        let api = Libunwind::get();
        let info = SendSyncPtr::new(NonNull::new(unwind_info.cast_mut()).unwrap());
        unsafe {
            if !api.is_libunwind() {
                // libgcc walks the table until an entry of length zero.
                __register_frame(unwind_info);
                return Ok(Self::SingleFrame(info));
            }

            // Require both APIs so removal uses the same registration grouping.
            if let (Some(add), Some(remove)) = (
                api.add_dynamic_eh_frame_section,
                api.remove_dynamic_eh_frame_section,
            ) {
                add(unwind_info as usize);
                return Ok(Self::LibunwindSection {
                    section: info,
                    deregister: remove,
                });
            }

            // libunwind's __register_frame takes a single FDE. Our .eh_frame
            // section ends with a four-byte zero terminator.
            let mut registration = Self::MultipleFrames(TryVec::new());
            let Self::MultipleFrames(frames) = &mut registration else {
                unreachable!();
            };
            let end = unwind_info.add(unwind_len - 4);
            let mut current = unwind_info;
            while current < end {
                let len = current.cast::<u32>().read_unaligned() as usize;
                // Skip the CIE at the beginning of the section.
                if current != unwind_info {
                    // Record the pointer before registering it, so an allocation
                    // failure unregisters all previous entries via Drop.
                    frames.push(SendSyncPtr::new(NonNull::new(current.cast_mut()).unwrap()))?;
                    __register_frame(current);
                }
                current = current.add(len + 4);
            }
            Ok(registration)
        }
    }
}

impl Drop for UnwindRegistration {
    fn drop(&mut self) {
        unsafe {
            match self {
                Self::SingleFrame(frame) => __deregister_frame(frame.as_ptr()),
                Self::MultipleFrames(frames) => {
                    // Preserve reverse registration order when removing FDEs.
                    for frame in frames.iter().rev() {
                        __deregister_frame(frame.as_ptr());
                    }
                }
                Self::LibunwindSection {
                    section,
                    deregister,
                } => {
                    deregister(section.as_ptr() as usize);
                }
            }
        }
    }
}
