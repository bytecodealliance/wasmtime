//! Binary machine code emission.
//!
//! The `binemit` module contains code for translating Cretonne's intermediate representation into
//! binary machine code.

mod relaxation;

pub use self::relaxation::relax_branches;

use ir::{Ebb, FuncRef, JumpTable, Function, Inst};

/// Offset in bytes from the beginning of the function.
///
/// Cretonne can be used as a cross compiler, so we don't want to use a type like `usize` which
/// depends on the *host* platform, not the *target* platform.
pub type CodeOffset = u32;

/// Relocation kinds depend on the current ISA.
pub struct Reloc(pub u16);

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
    fn reloc_ebb(&mut self, Reloc, Ebb);

    /// Add a relocation referencing an external function at the current offset.
    fn reloc_func(&mut self, Reloc, FuncRef);

    /// Add a relocation referencing a jump table.
    fn reloc_jt(&mut self, Reloc, JumpTable);
}

/// Report a bad encoding error.
#[inline(never)]
pub fn bad_encoding(func: &Function, inst: Inst) -> ! {
    panic!("Bad encoding {} for {}",
           func.encodings[inst],
           func.dfg.display_inst(inst));
}
