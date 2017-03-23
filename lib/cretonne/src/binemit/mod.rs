//! Binary machine code emission.
//!
//! The `binemit` module contains code for translating Cretonne's intermediate representation into
//! binary machine code.

use ir::{FuncRef, JumpTable};

/// Relocation kinds depend on the current ISA.
pub struct Reloc(u16);

/// Abstract interface for adding bytes to the code segment.
///
/// A `CodeSink` will receive all of the machine code for a function. It also accepts relocations
/// which are locations in the code section that need to be fixed up when linking.
pub trait CodeSink {
    /// Add 1 byte to the code section.
    fn put1(&mut self, u8);

    /// Add 2 bytes to the code section.
    fn put2(&mut self, u16);

    /// Add 4 bytes to the code section.
    fn put4(&mut self, u32);

    /// Add 8 bytes to the code section.
    fn put8(&mut self, u64);

    /// Add a relocation referencing an external function at the current offset.
    fn reloc_func(&mut self, Reloc, FuncRef);

    /// Add a relocation referencing a jump table.
    fn reloc_jt(&mut self, Reloc, JumpTable);
}
