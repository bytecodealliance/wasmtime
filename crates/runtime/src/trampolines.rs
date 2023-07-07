//! Wasm-to-libcall trampolines.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        #[macro_use]
        mod x86_64;
    } else if #[cfg(target_arch = "aarch64")] {
        #[macro_use]
        mod aarch64;
    } else if #[cfg(target_arch = "s390x")] {
        #[macro_use]
        mod s390x;
    }else if #[cfg(target_arch = "riscv64")] {
        #[macro_use]
        mod riscv64;
    } else {
        compile_error!("unsupported architecture");
    }
}
