//! Cranelift code generation library.
#![deny(missing_docs)]
#![no_std]
// Various bits and pieces of this crate might only be used for one platform or
// another, but it's not really too useful to learn about that all the time. On
// CI we build at least one version of this crate with `--features all-arch`
// which means we'll always detect truly dead code, otherwise if this is only
// built for one platform we don't have to worry too much about trimming
// everything down.
#![cfg_attr(not(feature = "all-arch"), allow(dead_code))]

#[allow(unused_imports)] // #[macro_use] is required for no_std
#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg(not(feature = "std"))]
use hashbrown::{hash_map, HashMap};
#[cfg(feature = "std")]
use std::collections::{hash_map, HashMap};

pub use crate::context::Context;
pub use crate::value_label::{LabelValueLoc, ValueLabelsRanges, ValueLocRange};
pub use crate::verifier::verify_function;
pub use crate::write::write_function;

pub use cranelift_bforest as bforest;
pub use cranelift_bitset as bitset;
pub use cranelift_control as control;
pub use cranelift_entity as entity;
#[cfg(feature = "unwind")]
pub use gimli;

#[macro_use]
mod machinst;

pub mod binemit;
pub mod cfg_printer;
pub mod cursor;
pub mod data_value;
pub mod dbg;
pub mod dominator_tree;
pub mod flowgraph;
pub mod ir;
pub mod isa;
pub mod loop_analysis;
pub mod print_errors;
pub mod settings;
pub mod timing;
pub mod traversals;
pub mod verifier;
pub mod write;

pub use crate::entity::packed_option;
pub use crate::machinst::buffer::{
    FinalizedMachReloc, FinalizedRelocTarget, MachCallSite, MachSrcLoc, MachTextSectionBuilder,
    MachTrap, OpenPatchRegion, PatchRegion,
};
pub use crate::machinst::{
    CallInfo, CompiledCode, Final, MachBuffer, MachBufferFinalized, MachInst, MachInstEmit,
    MachInstEmitState, MachLabel, RealReg, Reg, RelocDistance, TextSectionBuilder,
    VCodeConstantData, VCodeConstants, Writable,
};

mod alias_analysis;
mod constant_hash;
mod context;
mod ctxhash;
mod egraph;
mod inst_predicates;
mod isle_prelude;
mod iterators;
mod legalizer;
mod nan_canonicalization;
mod opts;
mod ranges;
mod remove_constant_phis;
mod result;
mod scoped_hash_map;
mod unionfind;
mod unreachable_code;
mod value_label;

#[cfg(feature = "souper-harvest")]
mod souper_harvest;

pub use crate::result::{CodegenError, CodegenResult, CompileError};

#[cfg(feature = "incremental-cache")]
pub mod incremental_cache;

/// Even when trace logging is disabled, the trace macro has a significant performance cost so we
/// disable it by default.
#[macro_export]
macro_rules! trace {
    ($($tt:tt)*) => {
        if cfg!(any(feature = "trace-log", debug_assertions)) {
            ::log::trace!($($tt)*);
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/version.rs"));
