//! Windows-specific hostcalls that implement
//! [WASI](https://github.com/WebAssembly/WASI).
mod fs;
pub(crate) mod fs_helpers;
mod misc;

pub(crate) use self::fs::*;
pub(crate) use self::misc::*;
