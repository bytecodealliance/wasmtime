/* Wasm architecture can be called such as 'WASI', 'WASM', and 'WASM32'
    so, I decided to call it 'WASM32'.
    Make sure that it doesn't make a refuse.
 */
use crate::dominator_tree::DominatorTree;
use crate::ir::{Function, Type};
use crate::MachInst::*;
use crate::isa::wasm32::settings as wasm21_settings;
use core::fmt;

#[cfg(feature = "unwind")]
use crate::isa::unwind::systemv::RegisterMappingError;
use cranelift_control::ControlPlane;
use alloc::vec::Vec;

pub(crate) mod inst;
mod lower;
mod abi;
mod settings;