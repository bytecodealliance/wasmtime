//! Binary machine code emission.
//!
//! The `binemit` module contains code for translating Cretonne's intermediate representation into
//! binary machine code.

mod relaxation;
mod memorysink;

pub use self::relaxation::relax_branches;
pub use self::memorysink::{MemoryCodeSink, RelocSink};

use ir::{ExternalName, JumpTable, Function, Inst};
use regalloc::RegDiversions;
use std::fmt;

/// Offset in bytes from the beginning of the function.
///
/// Cretonne can be used as a cross compiler, so we don't want to use a type like `usize` which
/// depends on the *host* platform, not the *target* platform.
pub type CodeOffset = u32;

/// Relocation kinds for every ISA
#[derive(Debug)]
pub enum Reloc {
    /// Intel PC-relative 4-byte
    IntelPCRel4,
    /// Intel absolute 4-byte
    IntelAbs4,
    /// Intel absolute 8-byte
    IntelAbs8,
    /// Arm32 call target
    Arm32Call,
    /// Arm64 call target
    Arm64Call,
    /// RISC-V call target
    RiscvCall,
}

impl fmt::Display for Reloc {
    /// Display trait implementation drops the arch, since its used in contexts where the arch is
    /// already unambigious, e.g. cton syntax with isa specified. In other contexts, use Debug.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Reloc::IntelPCRel4 => write!(f, "{}", "PCRel4"),
            Reloc::IntelAbs4 => write!(f, "{}", "Abs4"),
            Reloc::IntelAbs8 => write!(f, "{}", "Abs8"),
            Reloc::Arm32Call | Reloc::Arm64Call | Reloc::RiscvCall => write!(f, "{}", "Call"),
        }
    }
}

/// Abstract interface for adding bytes to the code segment.
///
/// A `CodeSink` will receive all of the machine code for a function. It also accepts relocations
/// which are locations in the code section that need to be fixed up when linking.
pub trait CodeSink {
    /// Get the current position.
    fn offset(&self) -> CodeOffset;

    /// Add 1 byte to the code section.
    fn put1(&mut self, u8);

    /// Add 2 bytes to the code section.
    fn put2(&mut self, u16);

    /// Add 4 bytes to the code section.
    fn put4(&mut self, u32);

    /// Add 8 bytes to the code section.
    fn put8(&mut self, u64);

    /// Add a relocation referencing an EBB at the current offset.
    fn reloc_ebb(&mut self, Reloc, CodeOffset);

    /// Add a relocation referencing an external symbol at the current offset.
    fn reloc_external(&mut self, Reloc, &ExternalName);

    /// Add a relocation referencing a jump table.
    fn reloc_jt(&mut self, Reloc, JumpTable);
}

/// Report a bad encoding error.
#[inline(never)]
pub fn bad_encoding(func: &Function, inst: Inst) -> ! {
    panic!(
        "Bad encoding {} for {}",
        func.encodings[inst],
        func.dfg.display_inst(inst, None)
    );
}

/// Emit a function to `sink`, given an instruction emitter function.
///
/// This function is called from the `TargetIsa::emit_function()` implementations with the
/// appropriate instruction emitter.
pub fn emit_function<CS, EI>(func: &Function, emit_inst: EI, sink: &mut CS)
where
    CS: CodeSink,
    EI: Fn(&Function, Inst, &mut RegDiversions, &mut CS),
{
    let mut divert = RegDiversions::new();
    for ebb in func.layout.ebbs() {
        divert.clear();
        assert_eq!(func.offsets[ebb], sink.offset());
        for inst in func.layout.ebb_insts(ebb) {
            emit_inst(func, inst, &mut divert, sink);
        }
    }
}
