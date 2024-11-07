//! Contains traits that a user of this assembler must implement.

use crate::reg;
use arbitrary::Arbitrary;
use std::{num::NonZeroU8, ops::Index, vec::Vec};

/// Describe how an instruction is emitted into a code buffer.
pub trait CodeSink {
    /// Add 1 byte to the code section.
    fn put1(&mut self, _: u8);

    /// Add 2 bytes to the code section.
    fn put2(&mut self, _: u16);

    /// Add 4 bytes to the code section.
    fn put4(&mut self, _: u32);

    /// Add 8 bytes to the code section.
    fn put8(&mut self, _: u64);

    /// Inform the code buffer of a possible trap at the current location;
    /// required for assembling memory accesses.
    fn add_trap(&mut self, code: TrapCode);

    /// Return the byte offset of the current location in the code buffer;
    /// required for assembling RIP-relative memory accesses.
    fn current_offset(&self) -> u32;

    /// Inform the code buffer of a use of `label` at `offset`; required for
    /// assembling RIP-relative memory accesses.
    fn use_label_at_offset(&mut self, offset: u32, label: Label);

    /// Return the label for a constant `id`; required for assembling
    /// RIP-relative memory accesses of constants.
    fn get_label_for_constant(&mut self, id: Constant) -> Label;
}

/// Provide a convenient implementation for testing.
impl CodeSink for Vec<u8> {
    fn put1(&mut self, v: u8) {
        self.extend_from_slice(&[v]);
    }

    fn put2(&mut self, v: u16) {
        self.extend_from_slice(&v.to_le_bytes());
    }

    fn put4(&mut self, v: u32) {
        self.extend_from_slice(&v.to_le_bytes());
    }

    fn put8(&mut self, v: u64) {
        self.extend_from_slice(&v.to_le_bytes());
    }

    fn add_trap(&mut self, _: TrapCode) {}

    fn current_offset(&self) -> u32 {
        self.len().try_into().unwrap()
    }

    fn use_label_at_offset(&mut self, _: u32, _: Label) {}

    fn get_label_for_constant(&mut self, c: Constant) -> Label {
        Label(c.0)
    }
}

/// Wrap [`CodeSink`]-specific labels.
#[derive(Debug, Clone, Arbitrary)]
pub struct Label(pub u32);

/// Wrap [`CodeSink`]-specific constant keys.
#[derive(Debug, Clone, Arbitrary)]
pub struct Constant(pub u32);

/// Wrap [`CodeSink`]-specific trap codes.
#[derive(Debug, Clone, Copy, Arbitrary)]
pub struct TrapCode(pub NonZeroU8);

/// A table mapping `KnownOffset` identifiers to their `i32` offset values.
///
/// When encoding instructions, Cranelift may not know all of the information
/// needed to construct an immediate. Specifically, addressing modes that
/// require knowing the size of the tail arguments or outgoing arguments (see
/// `SyntheticAmode::finalize`) will not know these sizes until emission.
///
/// This table allows up to do a "late" look up of these values by their
/// `KnownOffset`.
pub trait KnownOffsetTable: Index<KnownOffset, Output = i32> {}
impl KnownOffsetTable for Vec<i32> {}
impl KnownOffsetTable for [i32; 2] {}

/// A `KnownOffset` is a unique identifier for a specific offset known only at
/// emission time.
pub type KnownOffset = usize;

/// A type set fixing the register types used in the assembler.
///
/// This assembler is parameterizable over register types; this allows the
/// assembler users (e.g., Cranelift) to define their own register types
/// independent of this crate.
pub trait Registers {
    /// An x64 general purpose register that may be read.
    type ReadGpr: AsReg + for<'a> Arbitrary<'a>;

    /// An x64 general purpose register that may be read and written.
    type ReadWriteGpr: AsReg + for<'a> Arbitrary<'a>;
}

/// Describe how to interact with an external register type.
pub trait AsReg: Clone + std::fmt::Debug {
    /// Create a register from its hardware encoding.
    ///
    /// This is primarily useful for fuzzing, though it is also useful for
    /// generating fixed registers.
    fn new(enc: u8) -> Self;

    /// Return the register's hardware encoding; e.g., `0` for `%rax`.
    fn enc(&self) -> u8;

    /// Return the register name.
    fn to_string(&self, size: reg::Size) -> &str {
        reg::enc::to_string(self.enc(), size)
    }
}

/// Provide a convenient implementation for testing.
impl AsReg for u8 {
    fn new(enc: u8) -> Self {
        enc
    }
    fn enc(&self) -> u8 {
        *self
    }
}

/// Describe a visitor for the register operands of an instruction.
///
/// Due to how Cranelift's register allocation works, we allow the visitor to
/// modify the register operands in place. This allows Cranelift to convert
/// virtual registers (`[128..N)`) to physical registers (`[0..16)`) without
/// re-allocating the entire instruction object.
pub trait RegisterVisitor<R: Registers> {
    /// Visit a read-only register.
    fn read(&mut self, reg: &mut R::ReadGpr);
    /// Visit a read-write register.
    fn read_write(&mut self, reg: &mut R::ReadWriteGpr);
    /// Visit a read-only fixed register; for safety, this register cannot be
    /// modified in-place.
    fn fixed_read(&mut self, reg: &R::ReadGpr);
    /// Visit a read-write fixed register; for safety, this register cannot be
    /// modified in-place.
    fn fixed_read_write(&mut self, reg: &R::ReadWriteGpr);
}
