//! Windows-specific hostcalls that implement
//! [WASI](https://github.com/CraneStation/wasmtime-wasi/blob/wasi/docs/WASI-overview.md).
mod fs;
mod fs_helpers;
mod misc;

pub(crate) use self::fs::*;
pub(crate) use self::misc::*;
