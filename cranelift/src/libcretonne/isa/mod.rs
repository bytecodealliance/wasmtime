//! Instruction Set Architectures.
//!
//! The `isa` module provides a `TargetIsa` trait which provides the behavior specialization needed
//! by the ISA-independent code generator.
//!
//! The sub-modules of this module provide definitions for the instruction sets that Cretonne
//! can target. Each sub-module has it's own implementation of `TargetIsa`.

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
