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
        mod x86_64;
        pub use x86_64::*;
    } else if #[cfg(target_arch = "aarch64")] {
        mod aarch64;
        pub use aarch64::*;
    } else if #[cfg(target_arch = "s390x")] {
        mod s390x;
        pub use s390x::*;
    } else if #[cfg(target_arch = "riscv64")] {
        mod riscv64;
        pub use riscv64::*;
    } else {
        compile_error!(
            "Wasmtime is being compiled for an architecture \
             that it does not support. If this architecture is \
             one you would like to see supported you may file an \
             issue on Wasmtime's issue tracker: \
             https://github.com/bytecodealliance/wasmtime/issues/new\
        ");
    }
}
