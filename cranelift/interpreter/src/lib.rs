//! Cranelift IR interpreter.
//!
//! This module is a project for interpreting Cranelift IR.

pub mod environment;
pub mod frame;
pub mod instruction;
pub mod interpreter;
pub mod state;
pub mod step;
pub mod value;
