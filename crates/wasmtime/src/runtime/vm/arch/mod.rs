//! Architecture-specific support required by Wasmtime.
//!
//! This crate houses any architecture-specific tidbits required when running
//! Wasmtime. Each architecture has its own file in the `arch` folder which is
//! referenced here.
//!
//! All architectures have the same interface when exposed to the rest of the
//! crate.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        mod x86;
        use x86 as imp;
    } else if #[cfg(target_arch = "aarch64")] {
        mod aarch64;
        use aarch64 as imp;
    } else if #[cfg(target_arch = "s390x")] {
        mod s390x;
        use s390x as imp;
    } else if #[cfg(target_arch = "riscv64")] {
        mod riscv64;
        use riscv64 as imp;
    } else {
        mod unsupported;
        use unsupported as imp;
    }
}

// Functions defined in this module but all the implementations delegate to each
// `imp` module. This exists to assert that each module internally provides the
// same set of functionality with the same types for all architectures.

pub fn get_stack_pointer() -> usize {
    imp::get_stack_pointer()
}

pub unsafe fn get_next_older_pc_from_fp(fp: usize) -> usize {
    imp::get_next_older_pc_from_fp(fp)
}

pub const NEXT_OLDER_FP_FROM_FP_OFFSET: usize = imp::NEXT_OLDER_FP_FROM_FP_OFFSET;

pub fn assert_fp_is_aligned(fp: usize) {
    imp::assert_fp_is_aligned(fp)
}
