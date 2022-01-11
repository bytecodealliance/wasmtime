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
use super::{Addend, CodeOffset, Reloc};
use crate::binemit::stack_map::StackMap;
use crate::ir::{ExternalName, SourceLoc, TrapCode};

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
}

/// A trait for receiving trap codes and offsets.
///
/// If you don't need information about possible traps, you can use the
/// [`NullTrapSink`](NullTrapSink) implementation.
pub trait TrapSink {
    /// Add trap information for a specific offset.
    fn trap(&mut self, _: CodeOffset, _: SourceLoc, _: TrapCode);
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
