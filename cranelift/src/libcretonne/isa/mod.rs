//! Instruction Set Architectures.
//!
//! The `isa` module provides a `TargetIsa` trait which provides the behavior specialization needed
//! by the ISA-independent code generator. The sub-modules of this module provide definitions for
//! the instruction sets that Cretonne can target. Each sub-module has it's own implementation of
//! `TargetIsa`.
//!
//! # Constructing a `TargetIsa` instance
//!
//! The target ISA is built from the following information:
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
//! ```
//! use cretonne::settings::{self, Configurable};
//! use cretonne::isa;
//!
//! let shared_builder = settings::builder();
//! let shared_flags = settings::Flags::new(shared_builder);
//!
//! match isa::lookup("riscv") {
//!     None => {
//!         // The RISC-V target ISA is not available.
//!     }
//!     Some(mut isa_builder) => {
//!         isa_builder.set("supports_m", "on");
//!         let isa = isa_builder.finish(shared_flags);
//!     }
//! }
//! ```
//!
//! The configured target ISA trait object is a `Box<TargetIsa>` which can be used for multiple
//! concurrent function compilations.

pub mod riscv;
mod encoding;

use settings;
use ir::{InstructionData, DataFlowGraph};

/// Look for a supported ISA with the given `name`.
/// Return a builder that can create a corresponding `TargetIsa`.
pub fn lookup(name: &str) -> Option<Builder> {
    match name {
        "riscv" => riscv_builder(),
        _ => None,
    }
}

// Make a builder for RISC-V.
fn riscv_builder() -> Option<Builder> {
    Some(riscv::isa_builder())
}

/// Builder for a `TargetIsa`.
/// Modify the ISA-specific settings before creating the `TargetIsa` trait object with `finish`.
pub struct Builder {
    setup: settings::Builder,
    constructor: fn(settings::Flags, settings::Builder) -> Box<TargetIsa>,
}

impl Builder {
    /// Combine the ISA-specific settings with the provided ISA-independent settings and allocate a
    /// fully configured `TargetIsa` trait object.
    pub fn finish(self, shared_flags: settings::Flags) -> Box<TargetIsa> {
        (self.constructor)(shared_flags, self.setup)
    }
}

impl settings::Configurable for Builder {
    fn set(&mut self, name: &str, value: &str) -> settings::Result<()> {
        self.setup.set(name, value)
    }

    fn set_bool(&mut self, name: &str, value: bool) -> settings::Result<()> {
        self.setup.set_bool(name, value)
    }
}

pub trait TargetIsa {
    /// Encode an instruction after determining it is legal.
    ///
    /// If `inst` can legally be encoded in this ISA, produce the corresponding `Encoding` object.
    /// Otherwise, return `None`.
    ///
    /// This is also the main entry point for determining if an instruction is legal.
    fn encode(&self, dfg: &DataFlowGraph, inst: &InstructionData) -> Option<Encoding>;

    /// Get a static array of names associated with encoding recipes in this ISA. Encoding recipes
    /// are numbered starting from 0, corresponding to indexes into th name array.
    ///
    /// This is just used for printing and parsing encodings in the textual IL format.
    fn recipe_names(&self) -> &'static [&'static str];
}

/// Bits needed to encode an instruction as binary machine code.
///
/// The encoding consists of two parts, both specific to the target ISA: An encoding *recipe*, and
/// encoding *bits*. The recipe determines the native instruction format and the mapping of
/// operands to encoded bits. The encoding bits provide additional information to the recipe,
/// typically parts of the opcode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Encoding {
    recipe: u16,
    bits: u16,
}

impl Encoding {
    /// Create a new `Encoding` containing `(recipe, bits)`.
    pub fn new(recipe: u16, bits: u16) -> Encoding {
        Encoding {
            recipe: recipe,
            bits: bits,
        }
    }

    /// Get the recipe number in this encoding.
    pub fn recipe(self) -> usize {
        self.recipe as usize
    }

    /// Get the recipe-specific encoding bits.
    pub fn bits(self) -> u16 {
        self.bits
    }
}
