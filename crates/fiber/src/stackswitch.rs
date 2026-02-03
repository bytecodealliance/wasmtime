//! ISA-specific stack-switching routines.

// The bodies are defined in inline assembly in the conditionally
// included modules below; their symbols are visible in the binary and
// accessed via the `extern "C"` declarations below that.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "aarch64")] {
        mod aarch64;
        pub(crate) use supported::*;
        pub(crate) use aarch64::*;
    } else if #[cfg(target_arch = "x86_64")] {
        mod x86_64;
        pub(crate) use supported::*;
        pub(crate) use x86_64::*;
    } else if #[cfg(target_arch = "x86")] {
        mod x86;
        pub(crate) use supported::*;
        pub(crate) use x86::*;
    } else if #[cfg(target_arch = "arm")] {
        mod arm;
        pub(crate) use supported::*;
        pub(crate) use arm::*;
    } else if #[cfg(target_arch = "s390x")] {
        mod s390x;
        pub(crate) use supported::*;
        pub(crate) use s390x::*;
    } else if #[cfg(target_arch = "riscv64")]  {
        mod riscv64;
        pub(crate) use supported::*;
        pub(crate) use riscv64::*;
    } else if #[cfg(all(target_arch = "riscv32", not(target_feature = "f"), not(target_feature = "v")))] {
        mod riscv32imac;
        pub(crate) use supported::*;
        pub(crate) use riscv32imac::*;
    } else {
        // No support for this platform. Don't fail compilation though and
        // instead defer the error to happen at runtime when a fiber is created.
        // Should help keep compiles working and narrows the failure to only
        // situations that need fibers on unsupported platforms.
        pub(crate) use unsupported::*;
    }
}

/// A helper module to get reexported above in each case that we actually have
/// stack-switching routines available in inline asm. The fall-through case
/// though reexports the `unsupported` module instead.
#[allow(
    dead_code,
    reason = "expected to have dead code in some configurations"
)]
mod supported {
    pub const SUPPORTED_ARCH: bool = true;
}

/// Helper module reexported in the fallback case above when the current host
/// architecture is not supported for stack switching. The `SUPPORTED_ARCH`
/// boolean here is set to `false` which causes `Fiber::new` to return `false`.
#[allow(
    dead_code,
    reason = "expected to have dead code in some configurations"
)]
mod unsupported {
    pub const SUPPORTED_ARCH: bool = false;

    pub(crate) unsafe fn wasmtime_fiber_init(
        _top_of_stack: *mut u8,
        _entry: extern "C" fn(*mut u8, *mut u8),
        _entry_arg0: *mut u8,
    ) {
        unreachable!();
    }

    pub(crate) unsafe fn wasmtime_fiber_switch(_top_of_stack: *mut u8) {
        unreachable!();
    }
}
