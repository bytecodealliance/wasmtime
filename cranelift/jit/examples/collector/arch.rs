//! Architecture-specific handling of frame pointers, stack registers, etc.
//!
//! Most of this file has been copied from the [`unwinder`] crate in Wasmtime.

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    /// Stack pointer of the caller, relative to the current frame pointer.
    pub const PARENT_SP_FROM_FP_OFFSET: usize = 16;

    /// Gets the frame pointer which is the parent of the given
    /// frame, pointed to by `fp`.
    #[inline]
    pub(crate) unsafe fn parent_frame_pointer(fp: *const u8) -> *const u8 {
        (unsafe { *(fp as *mut usize) }) as *const u8
    }

    /// Gets the return address of the frame, pointed to by `fp`.
    #[inline]
    pub(crate) unsafe fn return_addr_of_frame(fp: *const u8) -> *const u8 {
        (unsafe { *(fp as *mut usize).offset(1) }) as *const u8
    }
}

#[cfg(target_arch = "aarch64")]
mod aarch64 {
    /// Stack pointer of the caller, relative to the current frame pointer.
    pub const PARENT_SP_FROM_FP_OFFSET: usize = 16;

    /// Gets the frame pointer which is the parent of the given
    /// frame, pointed to by `fp`.
    #[inline]
    pub(crate) unsafe fn parent_frame_pointer(fp: *const u8) -> *const u8 {
        (unsafe { *(fp as *mut usize) }) as *const u8
    }

    /// Gets the return address of the frame, pointed to by `fp`.
    #[inline]
    pub(crate) unsafe fn return_addr_of_frame(fp: *const u8) -> *const u8 {
        (unsafe { *(fp as *mut usize).offset(1) }) as *const u8
    }
}

#[cfg(target_arch = "x86_64")]
pub(crate) use x86_64::*;

#[cfg(target_arch = "aarch64")]
pub(crate) use aarch64::*;
