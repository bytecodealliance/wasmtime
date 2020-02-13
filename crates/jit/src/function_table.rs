//! Runtime function table.
//!
//! This module is primarily used to track JIT functions on Windows for stack walking and unwind.

type FunctionTableReloc = wasmtime_environ::CompiledFunctionUnwindInfoReloc;

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        mod imp_unix;
        use imp_unix as imp;
    } else if #[cfg(all(target_os = "windows", target_arch = "x86_64"))] {
        mod imp_windows;
        use imp_windows as imp;
    } else {
        compile_error! {
            "current platform is not supported"
        }
    }
}

pub(crate) use imp::FunctionTable;
