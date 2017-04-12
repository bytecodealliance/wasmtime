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
//! let shared_flags = settings::Flags::new(&shared_builder);
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

pub use isa::constraints::{RecipeConstraints, OperandConstraint, ConstraintKind, BranchRange};
pub use isa::encoding::{Encoding, EncInfo};
pub use isa::registers::{RegInfo, RegUnit, RegClass, RegClassIndex};

use binemit::CodeSink;
use settings;
use ir::{Function, Inst, InstructionData, DataFlowGraph, Signature, Type};

pub mod riscv;
pub mod intel;
pub mod arm32;
pub mod arm64;
pub mod registers;
mod encoding;
mod enc_tables;
mod constraints;

/// Look for a supported ISA with the given `name`.
/// Return a builder that can create a corresponding `TargetIsa`.
pub fn lookup(name: &str) -> Option<Builder> {
    match name {
        "riscv" => riscv_builder(),
        "intel" => intel_builder(),
        "arm32" => arm32_builder(),
        "arm64" => arm64_builder(),
        _ => None,
    }
}

// Make a builder for RISC-V.
fn riscv_builder() -> Option<Builder> {
    Some(riscv::isa_builder())
}

fn intel_builder() -> Option<Builder> {
    Some(intel::isa_builder())
}

fn arm32_builder() -> Option<Builder> {
    Some(arm32::isa_builder())
}

fn arm64_builder() -> Option<Builder> {
    Some(arm64::isa_builder())
}

/// Builder for a `TargetIsa`.
/// Modify the ISA-specific settings before creating the `TargetIsa` trait object with `finish`.
pub struct Builder {
    setup: settings::Builder,
    constructor: fn(settings::Flags, &settings::Builder) -> Box<TargetIsa>,
}

impl Builder {
    /// Combine the ISA-specific settings with the provided ISA-independent settings and allocate a
    /// fully configured `TargetIsa` trait object.
    pub fn finish(self, shared_flags: settings::Flags) -> Box<TargetIsa> {
        (self.constructor)(shared_flags, &self.setup)
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

/// After determining that an instruction doesn't have an encoding, how should we proceed to
/// legalize it?
///
/// These actions correspond to the transformation groups defined in `meta/cretonne/legalize.py`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Legalize {
    /// Legalize in terms of narrower types.
    Narrow,

    /// Expanding in terms of other instructions using the same types.
    Expand,
}

/// Methods that are specialized to a target ISA.
pub trait TargetIsa {
    /// Get the name of this ISA.
    fn name(&self) -> &'static str;

    /// Get the ISA-independent flags that were used to make this trait object.
    fn flags(&self) -> &settings::Flags;

    /// Get a data structure describing the registers in this ISA.
    fn register_info(&self) -> RegInfo;

    /// Encode an instruction after determining it is legal.
    ///
    /// If `inst` can legally be encoded in this ISA, produce the corresponding `Encoding` object.
    /// Otherwise, return `None`.
    ///
    /// This is also the main entry point for determining if an instruction is legal.
    fn encode(&self,
              dfg: &DataFlowGraph,
              inst: &InstructionData,
              ctrl_typevar: Type)
              -> Result<Encoding, Legalize>;

    /// Get a data structure describing the instruction encodings in this ISA.
    fn encoding_info(&self) -> EncInfo;

    /// Legalize a function signature.
    ///
    /// This is used to legalize both the signature of the function being compiled and any called
    /// functions. The signature should be modified by adding `ArgumentLoc` annotations to all
    /// arguments and return values.
    ///
    /// Arguments with types that are not supported by the ABI can be expanded into multiple
    /// arguments:
    ///
    /// - Integer types that are too large to fit in a register can be broken into multiple
    ///   arguments of a smaller integer type.
    /// - Floating point types can be bit-cast to an integer type of the same size, and possible
    ///   broken into smaller integer types.
    /// - Vector types can be bit-cast and broken down into smaller vectors or scalars.
    ///
    /// The legalizer will adapt argument and return values as necessary at all ABI boundaries.
    fn legalize_signature(&self, _sig: &mut Signature) {
        unimplemented!()
    }

    /// Emit binary machine code for a single instruction into the `sink` trait object.
    ///
    /// Note that this will call `put*` methods on the trait object via its vtable which is not the
    /// fastest way of emitting code.
    fn emit_inst(&self, func: &Function, inst: Inst, sink: &mut CodeSink);

    /// Get a static array of names associated with relocations in this ISA.
    ///
    /// This array can be indexed by the contents of `binemit::Reloc` objects passed to a
    /// `CodeSink`.
    fn reloc_names(&self) -> &'static [&'static str] {
        unimplemented!()
    }
}
