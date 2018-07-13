//! Code sink that writes binary machine code into contiguous memory.
//!
//! The `CodeSink` trait is the most general way of extracting binary machine code from Cranelift,
//! and it is implemented by things like the `test binemit` file test driver to generate
//! hexadecimal machine code. The `CodeSink` has some undesirable performance properties because of
//! the dual abstraction: `TargetIsa` is a trait object implemented by each supported ISA, so it
//! can't have any generic functions that could be specialized for each `CodeSink` implementation.
//! This results in many virtual function callbacks (one per `put*` call) when
//! `TargetIsa::emit_inst()` is used.
//!
//! The `MemoryCodeSink` type fixes the performance problem because it is a type known to
//! `TargetIsa` so it can specialize its machine code generation for the type. The trade-off is
//! that a `MemoryCodeSink` will always write binary machine code to raw memory. It forwards any
//! relocations to a `RelocSink` trait object. Relocations are less frequent than the
//! `CodeSink::put*` methods, so the performance impact of the virtual callbacks is less severe.

use super::{Addend, CodeOffset, CodeSink, Reloc};
use ir::{ExternalName, JumpTable, SourceLoc, TrapCode};
use std::ptr::write_unaligned;

/// A `CodeSink` that writes binary machine code directly into memory.
///
/// A `MemoryCodeSink` object should be used when emitting a Cranelift IR function into executable
/// memory. It writes machine code directly to a raw pointer without any bounds checking, so make
/// sure to allocate enough memory for the whole function. The number of bytes required is returned
/// by the `Context::compile()` function.
///
/// Any relocations in the function are forwarded to the `RelocSink` trait object.
///
/// Note that `MemoryCodeSink` writes multi-byte values in the native byte order of the host. This
/// is not the right thing to do for cross compilation.
pub struct MemoryCodeSink<'a> {
    data: *mut u8,
    offset: isize,
    relocs: &'a mut RelocSink,
    traps: &'a mut TrapSink,
}

impl<'a> MemoryCodeSink<'a> {
    /// Create a new memory code sink that writes a function to the memory pointed to by `data`.
    ///
    /// This function is unsafe since `MemoryCodeSink` does not perform bounds checking on the
    /// memory buffer, and it can't guarantee that the `data` pointer is valid.
    pub unsafe fn new<'sink>(
        data: *mut u8,
        relocs: &'sink mut RelocSink,
        traps: &'sink mut TrapSink,
    ) -> MemoryCodeSink<'sink> {
        MemoryCodeSink {
            data,
            offset: 0,
            relocs,
            traps,
        }
    }
}

/// A trait for receiving relocations for code that is emitted directly into memory.
pub trait RelocSink {
    /// Add a relocation referencing an EBB at the current offset.
    fn reloc_ebb(&mut self, CodeOffset, Reloc, CodeOffset);

    /// Add a relocation referencing an external symbol at the current offset.
    fn reloc_external(&mut self, CodeOffset, Reloc, &ExternalName, Addend);

    /// Add a relocation referencing a jump table.
    fn reloc_jt(&mut self, CodeOffset, Reloc, JumpTable);
}

/// A trait for receiving trap codes and offsets.
///
/// If you don't need information about possible traps, you can use the
/// [`NullTrapSink`](binemit/trait.TrapSink.html) implementation.
pub trait TrapSink {
    /// Add trap information for a specific offset.
    fn trap(&mut self, CodeOffset, SourceLoc, TrapCode);
}

impl<'a> CodeSink for MemoryCodeSink<'a> {
    fn offset(&self) -> CodeOffset {
        self.offset as CodeOffset
    }

    fn put1(&mut self, x: u8) {
        unsafe {
            write_unaligned(self.data.offset(self.offset), x);
        }
        self.offset += 1;
    }

    fn put2(&mut self, x: u16) {
        unsafe {
            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
            write_unaligned(self.data.offset(self.offset) as *mut u16, x);
        }
        self.offset += 2;
    }

    fn put4(&mut self, x: u32) {
        unsafe {
            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
            write_unaligned(self.data.offset(self.offset) as *mut u32, x);
        }
        self.offset += 4;
    }

    fn put8(&mut self, x: u64) {
        unsafe {
            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
            write_unaligned(self.data.offset(self.offset) as *mut u64, x);
        }
        self.offset += 8;
    }

    fn reloc_ebb(&mut self, rel: Reloc, ebb_offset: CodeOffset) {
        let ofs = self.offset();
        self.relocs.reloc_ebb(ofs, rel, ebb_offset);
    }

    fn reloc_external(&mut self, rel: Reloc, name: &ExternalName, addend: Addend) {
        let ofs = self.offset();
        self.relocs.reloc_external(ofs, rel, name, addend);
    }

    fn reloc_jt(&mut self, rel: Reloc, jt: JumpTable) {
        let ofs = self.offset();
        self.relocs.reloc_jt(ofs, rel, jt);
    }

    fn trap(&mut self, code: TrapCode, srcloc: SourceLoc) {
        let ofs = self.offset();
        self.traps.trap(ofs, srcloc, code);
    }
}

/// A `TrapSink` implementation that does nothing, which is convenient when
/// compiling code that does not rely on trapping semantics.
pub struct NullTrapSink {}

impl TrapSink for NullTrapSink {
    fn trap(&mut self, _offset: CodeOffset, _srcloc: SourceLoc, _code: TrapCode) {}
}
