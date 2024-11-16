//! Cranelift IR interpreter.
//!
//! This module is a project for interpreting Cranelift IR.

#![expect(clippy::allow_attributes_without_reason, reason = "crate not migrated")]

pub mod address;
pub mod environment;
pub mod frame;
pub mod instruction;
pub mod interpreter;
pub mod state;
pub mod step;
pub mod value;
