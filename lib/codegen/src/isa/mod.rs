//! Instruction Set Architectures.
//!
//! The `isa` module provides a `TargetIsa` trait which provides the behavior specialization needed
//! by the ISA-independent code generator. The sub-modules of this module provide definitions for
//! the instruction sets that Cranelift can target. Each sub-module has it's own implementation of
//! `TargetIsa`.
//!
//! # Constructing a `TargetIsa` instance
//!
//! The target ISA is built from the following information:
//!
//! - The name of the target ISA as a string. Cranelift is a cross-compiler, so the ISA to target
//!   can be selected dynamically. Individual ISAs can be left out when Cranelift is compiled, so a
//!   string is used to identify the proper sub-module.
//! - Values for settings that apply to all ISAs. This is represented by a `settings::Flags`
//!   instance.
//! - Values for ISA-specific settings.
//!
//! The `isa::lookup()` function is the main entry point which returns an `isa::Builder`
//! appropriate for the requested ISA:
//!
//! ```
//! # extern crate cranelift_codegen;
//! # #[macro_use] extern crate target_lexicon;
//! # fn main() {
//! use cranelift_codegen::isa;
//! use cranelift_codegen::settings::{self, Configurable};
//! use std::str::FromStr;
//! use target_lexicon::Triple;
//!
//! let shared_builder = settings::builder();
//! let shared_flags = settings::Flags::new(shared_builder);
//!
//! match isa::lookup(triple!("riscv32")) {
//!     Err(_) => {
//!         // The RISC-V target ISA is not available.
//!     }
//!     Ok(mut isa_builder) => {
//!         isa_builder.set("supports_m", "on");
//!         let isa = isa_builder.finish(shared_flags);
//!     }
//! }
//! # }
//! ```
//!
//! The configured target ISA trait object is a `Box<TargetIsa>` which can be used for multiple
//! concurrent function compilations.

pub use isa::constraints::{BranchRange, ConstraintKind, OperandConstraint, RecipeConstraints};
pub use isa::encoding::{EncInfo, Encoding};
pub use isa::registers::{regs_overlap, RegClass, RegClassIndex, RegInfo, RegUnit};
pub use isa::stack::{StackBase, StackBaseMask, StackRef};

use binemit;
use flowgraph;
use ir;
use isa::enc_tables::Encodings;
use regalloc;
use result::CodegenResult;
use settings;
use settings::{CallConv, SetResult};
use std::boxed::Box;
use std::fmt;
use target_lexicon::{Architecture, Triple};
use timing;

#[cfg(build_riscv)]
mod riscv;

#[cfg(build_x86)]
mod x86;

#[cfg(build_arm32)]
mod arm32;

#[cfg(build_arm64)]
mod arm64;

mod constraints;
mod enc_tables;
mod encoding;
pub mod registers;
mod stack;

/// Returns a builder that can create a corresponding `TargetIsa`
/// or `Err(LookupError::Unsupported)` if not enabled.
macro_rules! isa_builder {
    ($module:ident, $name:ident) => {{
        #[cfg($name)]
        fn $name(triple: Triple) -> Result<Builder, LookupError> {
            Ok($module::isa_builder(triple))
        };
        #[cfg(not($name))]
        fn $name(_triple: Triple) -> Result<Builder, LookupError> {
            Err(LookupError::Unsupported)
        }
        $name
    }};
}

/// Look for a supported ISA with the given `name`.
/// Return a builder that can create a corresponding `TargetIsa`.
pub fn lookup(triple: Triple) -> Result<Builder, LookupError> {
    match triple.architecture {
        Architecture::Riscv32 | Architecture::Riscv64 => isa_builder!(riscv, build_riscv)(triple),
        Architecture::I386 | Architecture::I586 | Architecture::I686 | Architecture::X86_64 => {
            isa_builder!(x86, build_x86)(triple)
        }
        Architecture::Thumbv6m
        | Architecture::Thumbv7em
        | Architecture::Thumbv7m
        | Architecture::Arm
        | Architecture::Armv4t
        | Architecture::Armv5te
        | Architecture::Armv7
        | Architecture::Armv7s => isa_builder!(arm32, build_arm32)(triple),
        Architecture::Aarch64 => isa_builder!(arm64, build_arm64)(triple),
        _ => Err(LookupError::Unsupported),
    }
}

/// Describes reason for target lookup failure
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum LookupError {
    /// Support for this target was disabled in the current build.
    SupportDisabled,

    /// Support for this target has not yet been implemented.
    Unsupported,
}

/// Builder for a `TargetIsa`.
/// Modify the ISA-specific settings before creating the `TargetIsa` trait object with `finish`.
pub struct Builder {
    triple: Triple,
    setup: settings::Builder,
    constructor: fn(Triple, settings::Flags, settings::Builder) -> Box<TargetIsa>,
}

impl Builder {
    /// Combine the ISA-specific settings with the provided ISA-independent settings and allocate a
    /// fully configured `TargetIsa` trait object.
    pub fn finish(self, shared_flags: settings::Flags) -> Box<TargetIsa> {
        (self.constructor)(self.triple, shared_flags, self.setup)
    }
}

impl settings::Configurable for Builder {
    fn set(&mut self, name: &str, value: &str) -> SetResult<()> {
        self.setup.set(name, value)
    }

    fn enable(&mut self, name: &str) -> SetResult<()> {
        self.setup.enable(name)
    }
}

/// After determining that an instruction doesn't have an encoding, how should we proceed to
/// legalize it?
///
/// The `Encodings` iterator returns a legalization function to call.
pub type Legalize =
    fn(ir::Inst, &mut ir::Function, &mut flowgraph::ControlFlowGraph, &TargetIsa) -> bool;

/// Methods that are specialized to a target ISA. Implies a Display trait that shows the
/// shared flags, as well as any isa-specific flags.
pub trait TargetIsa: fmt::Display {
    /// Get the name of this ISA.
    fn name(&self) -> &'static str;

    /// Get the target triple that was used to make this trait object.
    fn triple(&self) -> &Triple;

    /// Get the ISA-independent flags that were used to make this trait object.
    fn flags(&self) -> &settings::Flags;

    /// Get the pointer type of this ISA.
    fn pointer_type(&self) -> ir::Type {
        ir::Type::int(u16::from(self.pointer_bits())).unwrap()
    }

    /// Get the width of pointers on this ISA, in units of bits.
    fn pointer_bits(&self) -> u8 {
        self.triple().pointer_width().unwrap().bits()
    }

    /// Get the width of pointers on this ISA, in units of bytes.
    fn pointer_bytes(&self) -> u8 {
        self.triple().pointer_width().unwrap().bytes()
    }

    /// Does the CPU implement scalar comparisons using a CPU flags register?
    fn uses_cpu_flags(&self) -> bool {
        false
    }

    /// Does the CPU implement multi-register addressing?
    fn uses_complex_addresses(&self) -> bool {
        false
    }

    /// Get a data structure describing the registers in this ISA.
    fn register_info(&self) -> RegInfo;

    /// Returns an iterartor over legal encodings for the instruction.
    fn legal_encodings<'a>(
        &'a self,
        func: &'a ir::Function,
        inst: &'a ir::InstructionData,
        ctrl_typevar: ir::Type,
    ) -> Encodings<'a>;

    /// Encode an instruction after determining it is legal.
    ///
    /// If `inst` can legally be encoded in this ISA, produce the corresponding `Encoding` object.
    /// Otherwise, return `Legalize` action.
    ///
    /// This is also the main entry point for determining if an instruction is legal.
    fn encode(
        &self,
        func: &ir::Function,
        inst: &ir::InstructionData,
        ctrl_typevar: ir::Type,
    ) -> Result<Encoding, Legalize> {
        let mut iter = self.legal_encodings(func, inst, ctrl_typevar);
        iter.next().ok_or_else(|| iter.legalize())
    }

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
    ///
    /// When this function is called to legalize the signature of the function currently being
    /// compiled, `current` is true. The legalized signature can then also contain special purpose
    /// arguments and return values such as:
    ///
    /// - A `link` argument representing the link registers on RISC architectures that don't push
    ///   the return address on the stack.
    /// - A `link` return value which will receive the value that was passed to the `link`
    ///   argument.
    /// - An `sret` argument can be added if one wasn't present already. This is necessary if the
    ///   signature returns more values than registers are available for returning values.
    /// - An `sret` return value can be added if the ABI requires a function to return its `sret`
    ///   argument in a register.
    ///
    /// Arguments and return values for the caller's frame pointer and other callee-saved registers
    /// should not be added by this function. These arguments are not added until after register
    /// allocation.
    fn legalize_signature(&self, sig: &mut ir::Signature, current: bool);

    /// Get the register class that should be used to represent an ABI argument or return value of
    /// type `ty`. This should be the top-level register class that contains the argument
    /// registers.
    ///
    /// This function can assume that it will only be asked to provide register classes for types
    /// that `legalize_signature()` produces in `ArgumentLoc::Reg` entries.
    fn regclass_for_abi_type(&self, ty: ir::Type) -> RegClass;

    /// Get the set of allocatable registers that can be used when compiling `func`.
    ///
    /// This set excludes reserved registers like the stack pointer and other special-purpose
    /// registers.
    fn allocatable_registers(&self, func: &ir::Function) -> regalloc::RegisterSet;

    /// Compute the stack layout and insert prologue and epilogue code into `func`.
    ///
    /// Return an error if the stack frame is too large.
    fn prologue_epilogue(&self, func: &mut ir::Function) -> CodegenResult<()> {
        let _tt = timing::prologue_epilogue();
        // This default implementation is unlikely to be good enough.
        use ir::stackslot::{StackOffset, StackSize};
        use stack_layout::layout_stack;

        let word_size = StackSize::from(self.pointer_bytes());

        // Account for the SpiderMonkey standard prologue pushes.
        if func.signature.call_conv == CallConv::Baldrdash {
            let bytes = StackSize::from(self.flags().baldrdash_prologue_words()) * word_size;
            let mut ss = ir::StackSlotData::new(ir::StackSlotKind::IncomingArg, bytes);
            ss.offset = Some(-(bytes as StackOffset));
            func.stack_slots.push(ss);
        }

        layout_stack(&mut func.stack_slots, word_size)?;
        Ok(())
    }

    /// Emit binary machine code for a single instruction into the `sink` trait object.
    ///
    /// Note that this will call `put*` methods on the `sink` trait object via its vtable which
    /// is not the fastest way of emitting code.
    fn emit_inst(
        &self,
        func: &ir::Function,
        inst: ir::Inst,
        divert: &mut regalloc::RegDiversions,
        sink: &mut binemit::CodeSink,
    );

    /// Emit a whole function into memory.
    ///
    /// This is more performant than calling `emit_inst` for each instruction.
    fn emit_function_to_memory(&self, func: &ir::Function, sink: &mut binemit::MemoryCodeSink);
}
