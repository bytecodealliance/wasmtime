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
use super::{Addend, CodeInfo, CodeOffset, CodeSink, Reloc};
use crate::binemit::stack_map::StackMap;
use crate::ir::{ConstantOffset, ExternalName, Opcode, SourceLoc, TrapCode};
use core::ptr::write_unaligned;

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
    /// Pointer to start of sink's preallocated memory.
    data: *mut u8,
    /// Offset is isize because its major consumer needs it in that form.
    offset: isize,
    relocs: &'a mut dyn RelocSink,
    traps: &'a mut dyn TrapSink,
    /// Information about the generated code and read-only data.
    pub info: CodeInfo,
}

impl<'a> MemoryCodeSink<'a> {
    /// Create a new memory code sink that writes a function to the memory pointed to by `data`.
    ///
    /// # Safety
    ///
    /// This function is unsafe since `MemoryCodeSink` does not perform bounds checking on the
    /// memory buffer, and it can't guarantee that the `data` pointer is valid.
    pub unsafe fn new(
        data: *mut u8,
        relocs: &'a mut dyn RelocSink,
        traps: &'a mut dyn TrapSink,
    ) -> Self {
        Self {
            data,
            offset: 0,
            info: CodeInfo {
                code_size: 0,
                jumptables_size: 0,
                rodata_size: 0,
                total_size: 0,
            },
            relocs,
            traps,
        }
    }
}

/// A trait for receiving relocations for code that is emitted directly into memory.
pub trait RelocSink {
    /// Add a relocation referencing an external symbol at the current offset.
    fn reloc_external(
        &mut self,
        _: CodeOffset,
        _: SourceLoc,
        _: Reloc,
        _: &ExternalName,
        _: Addend,
    );

    /// Add a relocation referencing a constant.
    fn reloc_constant(&mut self, _: CodeOffset, _: Reloc, _: ConstantOffset);

    /// Track a call site whose return address is the given CodeOffset, for the given opcode. Does
    /// nothing in general, only useful for certain embedders (SpiderMonkey).
    fn add_call_site(&mut self, _: Opcode, _: CodeOffset, _: SourceLoc) {}
}

/// A trait for receiving trap codes and offsets.
///
/// If you don't need information about possible traps, you can use the
/// [`NullTrapSink`](NullTrapSink) implementation.
pub trait TrapSink {
    /// Add trap information for a specific offset.
    fn trap(&mut self, _: CodeOffset, _: SourceLoc, _: TrapCode);
}

impl<'a> MemoryCodeSink<'a> {
    fn write<T>(&mut self, x: T) {
        unsafe {
            #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
            write_unaligned(self.data.offset(self.offset) as *mut T, x);
            self.offset += core::mem::size_of::<T>() as isize;
        }
    }
}

impl<'a> CodeSink for MemoryCodeSink<'a> {
    fn offset(&self) -> CodeOffset {
        self.offset as CodeOffset
    }

    fn put1(&mut self, x: u8) {
        self.write(x);
    }

    fn put2(&mut self, x: u16) {
        self.write(x);
    }

    fn put4(&mut self, x: u32) {
        self.write(x);
    }

    fn put8(&mut self, x: u64) {
        self.write(x);
    }

    fn reloc_external(
        &mut self,
        srcloc: SourceLoc,
        rel: Reloc,
        name: &ExternalName,
        addend: Addend,
    ) {
        let ofs = self.offset();
        self.relocs.reloc_external(ofs, srcloc, rel, name, addend);
    }

    fn reloc_constant(&mut self, rel: Reloc, constant_offset: ConstantOffset) {
        let ofs = self.offset();
        self.relocs.reloc_constant(ofs, rel, constant_offset);
    }

    fn trap(&mut self, code: TrapCode, srcloc: SourceLoc) {
        let ofs = self.offset();
        self.traps.trap(ofs, srcloc, code);
    }

    fn begin_jumptables(&mut self) {
        self.info.code_size = self.offset();
    }

    fn begin_rodata(&mut self) {
        self.info.jumptables_size = self.offset() - self.info.code_size;
    }

    fn end_codegen(&mut self) {
        self.info.rodata_size = self.offset() - (self.info.jumptables_size + self.info.code_size);
        self.info.total_size = self.offset();
    }

    fn add_call_site(&mut self, opcode: Opcode, loc: SourceLoc) {
        debug_assert!(
            opcode.is_call(),
            "adding call site info for a non-call instruction."
        );
        let ret_addr = self.offset();
        self.relocs.add_call_site(opcode, ret_addr, loc);
    }
}

/// A `RelocSink` implementation that does nothing, which is convenient when
/// compiling code that does not relocate anything.
#[derive(Default)]
pub struct NullRelocSink {}

impl RelocSink for NullRelocSink {
    fn reloc_external(
        &mut self,
        _: CodeOffset,
        _: SourceLoc,
        _: Reloc,
        _: &ExternalName,
        _: Addend,
    ) {
    }
    fn reloc_constant(&mut self, _: CodeOffset, _: Reloc, _: ConstantOffset) {}
}

/// A `TrapSink` implementation that does nothing, which is convenient when
/// compiling code that does not rely on trapping semantics.
#[derive(Default)]
pub struct NullTrapSink {}

impl TrapSink for NullTrapSink {
    fn trap(&mut self, _offset: CodeOffset, _srcloc: SourceLoc, _code: TrapCode) {}
}

/// A trait for emitting stack maps.
pub trait StackMapSink {
    /// Output a bitmap of the stack representing the live reference variables at this code offset.
    fn add_stack_map(&mut self, _: CodeOffset, _: StackMap);
}

/// Placeholder StackMapSink that does nothing.
pub struct NullStackMapSink {}

impl StackMapSink for NullStackMapSink {
    fn add_stack_map(&mut self, _: CodeOffset, _: StackMap) {}
}
