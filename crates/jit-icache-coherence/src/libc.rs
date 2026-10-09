//! Cache maintenance on everything but Windows and Miri. Each function below is
//! provided by the module for the system it runs on.

cfg_select! {
    all(target_arch = "aarch64", any(target_os = "linux", target_os = "android")) => {
        mod membarrier;
        pub(crate) use membarrier::pipeline_flush_mt;
    }
    _ => {
        /// See docs on [crate::pipeline_flush_mt] for a description of what this function is trying to do.
        pub(crate) fn pipeline_flush_mt() -> wasmtime_core::error::Result<()> {
            Ok(())
        }
    }
}

cfg_select! {
    all(target_arch = "aarch64", target_vendor = "apple") => {
        mod aarch64_apple;
        pub(crate) use aarch64_apple::clear_cache;
    }
    target_arch = "aarch64" => {
        mod aarch64;
        pub(crate) use aarch64::clear_cache;
    }
    all(target_arch = "riscv64", target_os = "linux") => {
        mod riscv64_linux;
        pub(crate) use riscv64_linux::clear_cache;
    }
    _ => {
        /// See docs on [crate::clear_cache] for a description of what this function is trying to do.
        pub(crate) fn clear_cache(
            _ptr: *const std::ffi::c_void,
            _len: usize,
        ) -> wasmtime_core::error::Result<()> {
            Ok(())
        }
    }
}
