//! Utility module to create trampolines in/out WebAssembly module.

mod create_handle;
mod func;
mod global;
mod memory;
mod table;

use self::func::create_handle_with_function;
use self::global::create_global;
use self::memory::create_handle_with_memory;
use self::table::create_handle_with_table;
use super::{Callable, FuncType, GlobalType, MemoryType, Store, TableType, Val};
use crate::r#ref::HostRef;
use anyhow::Result;
use std::rc::Rc;

pub use self::global::GlobalState;

pub fn generate_func_export(
    ft: &FuncType,
    func: &Rc<dyn Callable + 'static>,
    store: &HostRef<Store>,
) -> Result<(wasmtime_runtime::InstanceHandle, wasmtime_runtime::Export)> {
    let mut instance = create_handle_with_function(ft, func, store)?;
    let export = instance.lookup("trampoline").expect("trampoline export");
    Ok((instance, export))
}

pub fn generate_global_export(
    gt: &GlobalType,
    val: Val,
) -> Result<(wasmtime_runtime::Export, GlobalState)> {
    create_global(gt, val)
}

pub fn generate_memory_export(
    m: &MemoryType,
) -> Result<(wasmtime_runtime::InstanceHandle, wasmtime_runtime::Export)> {
    let mut instance = create_handle_with_memory(m)?;
    let export = instance.lookup("memory").expect("memory export");
    Ok((instance, export))
}

pub fn generate_table_export(
    t: &TableType,
) -> Result<(wasmtime_runtime::InstanceHandle, wasmtime_runtime::Export)> {
    let mut instance = create_handle_with_table(t)?;
    let export = instance.lookup("table").expect("table export");
    Ok((instance, export))
}

pub(crate) use cranelift_codegen::print_errors::pretty_error;

pub(crate) mod binemit {
    pub(crate) use cranelift_codegen::binemit::{NullStackmapSink, NullTrapSink};

    pub use cranelift_codegen::{binemit, ir};

    /// We don't expect trampoline compilation to produce any relocations, so
    /// this `RelocSink` just asserts that it doesn't recieve any.
    pub(crate) struct TrampolineRelocSink {}

    impl binemit::RelocSink for TrampolineRelocSink {
        fn reloc_ebb(
            &mut self,
            _offset: binemit::CodeOffset,
            _reloc: binemit::Reloc,
            _ebb_offset: binemit::CodeOffset,
        ) {
            panic!("trampoline compilation should not produce ebb relocs");
        }
        fn reloc_external(
            &mut self,
            _offset: binemit::CodeOffset,
            _reloc: binemit::Reloc,
            _name: &ir::ExternalName,
            _addend: binemit::Addend,
        ) {
            panic!("trampoline compilation should not produce external symbol relocs");
        }
        fn reloc_constant(
            &mut self,
            _code_offset: binemit::CodeOffset,
            _reloc: binemit::Reloc,
            _constant_offset: ir::ConstantOffset,
        ) {
            panic!("trampoline compilation should not produce constant relocs");
        }
        fn reloc_jt(
            &mut self,
            _offset: binemit::CodeOffset,
            _reloc: binemit::Reloc,
            _jt: ir::JumpTable,
        ) {
            panic!("trampoline compilation should not produce jump table relocs");
        }
    }
}

pub(crate) mod ir {
    pub(crate) use cranelift_codegen::ir::{
        ExternalName, Function, InstBuilder, MemFlags, StackSlotData, StackSlotKind, TrapCode,
    };
}
pub(crate) use cranelift_codegen::isa::TargetIsa;
pub(crate) use cranelift_codegen::Context;
pub(crate) use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
