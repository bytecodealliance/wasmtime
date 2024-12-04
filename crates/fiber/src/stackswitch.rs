//! ISA-specific stack-switching routines.

// The bodies are defined in inline assembly in the conditionally
// included modules below; their symbols are visible in the binary and
// accessed via the `extern "C"` declarations below that.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "aarch64")] {
        mod aarch64;
    } else if #[cfg(target_arch = "x86_64")] {
        mod x86_64;
    } else if #[cfg(target_arch = "x86")] {
        mod x86;
    } else if #[cfg(target_arch = "arm")] {
        mod arm;
    } else if #[cfg(target_arch = "s390x")] {
        // currently `global_asm!` isn't stable on s390x so this is an external
        // assembler file built with the `build.rs`.
    } else if #[cfg(target_arch = "riscv64")]  {
        mod riscv64;
    } else {
        compile_error!("fibers are not supported on this CPU architecture");
    }
}

extern "C" {
    #[wasmtime_versioned_export_macros::versioned_link]
    pub(crate) fn wasmtime_fiber_init(
        top_of_stack: *mut u8,
        entry: extern "C" fn(*mut u8, *mut u8),
        entry_arg0: *mut u8,
    );
    #[wasmtime_versioned_export_macros::versioned_link]
    pub(crate) fn wasmtime_fiber_switch(top_of_stack: *mut u8);
    #[allow(dead_code, reason = "only used on some platforms for inline asm")]
    #[wasmtime_versioned_export_macros::versioned_link]
    pub(crate) fn wasmtime_fiber_start();
}
