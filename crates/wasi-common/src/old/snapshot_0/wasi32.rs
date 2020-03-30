//! Types and constants specific to 32-bit wasi. These are similar to the types
//! in the `host` module, but pointers and `usize` values are replaced with
//! `u32`-sized types.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use crate::old::snapshot_0::wasi::*;
use wig::witx_wasi32_types;

pub type uintptr_t = u32;
pub type size_t = u32;

witx_wasi32_types!("phases/old/snapshot_0/witx/wasi_unstable.witx");
