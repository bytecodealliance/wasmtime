//! Unix-specific hostcalls that implement
//! [WASI](https://github.com/CraneStation/wasmtime-wasi/blob/wasi/docs/WASI-overview.md).
mod fs;
mod fs_helpers;
mod misc;

use super::fdentry;
use super::host_impl;

pub(crate) use self::fs::*;
pub(crate) use self::misc::*;
