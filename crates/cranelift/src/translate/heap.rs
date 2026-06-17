//! Heaps to implement WebAssembly linear memories.

use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::{self, InstBuilder, Type};
use cranelift_entity::entity_impl;
use smallvec::SmallVec;
use wasmtime_environ::{IndexType, Memory};

pub use wasmtime_environ::MemoryKind;

/// A single load, relative to some base value.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Load {
    /// The offset added to the base before loading.
    pub offset: u32,
    /// The memory flags for the load.
    pub flags: ir::MemFlagsData,
    /// The type loaded. Only the last load in a chain may be non-pointer-typed.
    pub ty: ir::Type,
}

impl Load {
    /// Emit this load, relative to the given `base`.
    pub fn emit(&self, cursor: &mut FuncCursor<'_>, base: ir::Value) -> ir::Value {
        cursor.ins().load(
            self.ty,
            self.flags,
            base,
            i32::try_from(self.offset).unwrap(),
        )
    }

    /// Emit this load, relative to the given `base`, as an `ir::GlobalValue`.
    ///
    /// Prefer plain `emit`; this is only for cases where a global value is
    /// required (like the stack limit checks that Cranelift emits inline in the
    /// compiled function's prologue).
    pub fn emit_global(&self, func: &mut ir::Function, base: ir::GlobalValue) -> ir::GlobalValue {
        let flags = func.dfg.mem_flags.insert(self.flags).unwrap();
        func.global_values.push(ir::GlobalValueData::Load {
            base,
            offset: i32::try_from(self.offset).unwrap().into(),
            global_type: self.ty,
            flags,
        })
    }
}

/// A chain of loads, rooted at the `vmctx`.
///
/// Used to compute a heap's base address or bound.
#[derive(Clone, PartialEq, Hash)]
pub struct VmctxLoadChain(SmallVec<[Load; 2]>);

impl VmctxLoadChain {
    /// Create a new load sequence.
    ///
    /// The sequence must be non-empty.
    pub fn new(loads: SmallVec<[Load; 2]>) -> Self {
        assert!(!loads.is_empty());
        VmctxLoadChain(loads)
    }

    /// Emit the load chain, starting from `vmctx`, returning the final loaded
    /// value.
    pub fn emit(&self, cursor: &mut FuncCursor<'_>, vmctx: ir::Value) -> ir::Value {
        let mut val = vmctx;
        for load in &self.0 {
            val = load.emit(cursor, val);
        }
        val
    }

    /// Emit the load chain, starting from `vmctx`, as a sequence of CLIF global
    /// values.
    ///
    /// Prefer plain `emit`; this is only for cases where a global value is
    /// required (like the stack limit checks that Cranelift emits inline in the
    /// compiled function's prologue).
    pub fn emit_global(&self, func: &mut ir::Function) -> ir::GlobalValue {
        let mut val = func.global_values.push(ir::GlobalValueData::VMContext);
        for load in &self.0 {
            val = load.emit_global(func, val);
        }
        val
    }
}

/// An opaque reference to a [`HeapData`][crate::HeapData].
///
/// While the order is stable, it is arbitrary.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Heap(u32);
entity_impl!(Heap, "heap");

/// A heap implementing a WebAssembly linear memory.
///
/// Code compiled from WebAssembly runs in a sandbox where it can't access all
/// process memory. Instead, it is given a small set of memory areas to work in,
/// and all accesses are bounds checked. Wasmtime models this through
/// the concept of *heaps*.
///
/// Heap addresses can be smaller than the native pointer size, for example
/// unsigned `i32` offsets on a 64-bit architecture.
///
/// A heap appears as three consecutive ranges of address space:
///
/// 1. The *mapped pages* are the accessible memory range in the heap. A heap
///    may have a minimum guaranteed size which means that some mapped pages are
///    always present.
///
/// 2. The *unmapped pages* is a possibly empty range of address space that may
///    be mapped in the future when the heap is grown. They are addressable but
///    not accessible.
///
/// 3. The *offset-guard pages* is a range of address space that is guaranteed
///    to always cause a trap when accessed. It is used to optimize bounds
///    checking for heap accesses with a shared base pointer. They are
///    addressable but not accessible.
#[derive(Clone, PartialEq, Hash)]
pub struct HeapData {
    /// The address of the start of the heap's storage.
    pub base: VmctxLoadChain,

    /// The dynamic byte length of this heap, if needed.
    pub bound: VmctxLoadChain,

    /// The type of wasm memory that this heap is operating on.
    pub memory: Memory,

    /// Whether this is a linear memory or a GC heap.
    pub kind: MemoryKind,
}

impl HeapData {
    pub fn index_type(&self) -> Type {
        match self.memory.idx_type {
            IndexType::I32 => ir::types::I32,
            IndexType::I64 => ir::types::I64,
        }
    }

    /// Get the [`MemoryTunables`] for this heap based on its [`MemoryKind`].
    pub fn memory_tunables<'a>(
        &self,
        tunables: &'a wasmtime_environ::Tunables,
    ) -> wasmtime_environ::MemoryTunables<'a> {
        wasmtime_environ::MemoryTunables::new(tunables, self.kind)
    }
}
