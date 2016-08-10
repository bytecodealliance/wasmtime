//! Instruction Set Architectures.
//!
//! The `isa` module provides a `TargetIsa` trait which provides the behavior specialization needed
//! by the ISA-independent code generator. The sub-modules of this module provide definitions for
//! the instruction sets that Cretonne can target. Each sub-module has it's own implementation of
//! `TargetIsa`.
//!
//! # Constructing a `TargetIsa` instance
//!
//! The target ISA is build from the following information:
//!
//! - The name of the target ISA as a string. Cretonne is a cross-compiler, so the ISA to target
//!   can be selected dynamically. Individual ISAs can be left out when Cretonne is compiled, so a
//!   string is used to identify the proper sub-module.
//! - Values for settings that apply to all ISAs. This is represented by a `settings::Flags`
//!   instance.
//! - Values for ISA-specific settings.
//!
//! The `isa::lookup()` function is the main entry point which returns an `isa::Builder`
//! appropriate for the requested ISA:
//!
//! ```ignore
//! let isa_builder = isa::lookup("riscv").unwrap();
//! adjust_isa_settings(&mut isa_builder.settings);
//! let isa = builder.finish(shared_settings());
//! ```
//!

pub mod riscv;

use ir::dfg::DataFlowGraph;
use ir::entities::Inst;

pub trait TargetIsa {
    /// Encode an instruction after determining it is legal.
    ///
    /// If `inst` can legally be encoded in this ISA, produce the corresponding `Encoding` object.
    /// Otherwise, return `None`.
    ///
    /// This is also the main entry point for determining if an instruction is legal.
    fn encode(&self, dfg: &DataFlowGraph, inst: &Inst) -> Option<Encoding>;
}

/// Bits needed to encode an instruction as binary machine code.
///
/// The encoding consists of two parts, both specific to the target ISA: An encoding *recipe*, and
/// encoding *bits*. The recipe determines the native instruction format and the mapping of
/// operands to encoded bits. The encoding bits provide additional information to the recipe,
/// typically parts of the opcode.
pub struct Encoding(u32);

impl Encoding {
    /// Create a new `Encoding` containing `(recipe, bits)`. The `num_bits` parameter is the
    /// ISA-dependent size of `bits`.
    pub fn new(recipe: u32, bits: u32, num_bits: u8) -> Encoding {
        Encoding((recipe << num_bits) | bits)
    }

    /// Split the encoding into two parts: `(recipe, bits)`. Only the target ISA knows how many
    /// bits are in each part.
    pub fn split(&self, num_bits: u8) -> (u32, u32) {
        (self.0 >> num_bits, self.0 & ((1 << num_bits) - 1))
    }
}
