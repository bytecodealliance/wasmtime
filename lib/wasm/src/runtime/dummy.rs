use runtime::{FuncEnvironment, GlobalValue, WasmRuntime};
use translation_utils::{Local, Global, Memory, Table, GlobalIndex, TableIndex, FunctionIndex,
                        MemoryIndex};
use cton_frontend::FunctionBuilder;
use cretonne::ir::{self, Value, InstBuilder, SigRef};
use cretonne::ir::types::*;

/// This runtime implementation is a "naïve" one, doing essentially nothing and emitting
/// placeholders when forced to. Don't try to execute code translated with this runtime, it is
/// essentially here for translation debug purposes.
pub struct DummyRuntime {
    globals: Vec<Global>,
}

impl DummyRuntime {
    /// Allocates the runtime data structures.
    pub fn new() -> Self {
        Self { globals: Vec::new() }
    }
}

impl FuncEnvironment for DummyRuntime {
    fn native_pointer(&self) -> ir::Type {
        ir::types::I64
    }

    fn make_global(&self, func: &mut ir::Function, index: GlobalIndex) -> GlobalValue {
        // Just create a dummy `vmctx` global.
        let offset = ((index * 8) as i32 + 8).into();
        let gv = func.global_vars.push(ir::GlobalVarData::VmCtx { offset });
        GlobalValue::Memory {
            gv,
            ty: self.globals[index].ty,
        }
    }

    fn make_heap(&self, func: &mut ir::Function, _index: MemoryIndex) -> ir::Heap {
        func.heaps.push(ir::HeapData {
            base: ir::HeapBase::ReservedReg,
            min_size: 0.into(),
            guard_size: 0x8000_0000.into(),
            style: ir::HeapStyle::Static { bound: 0x1_0000_0000.into() },
        })
    }
}

impl WasmRuntime for DummyRuntime {
    fn translate_grow_memory(&mut self, builder: &mut FunctionBuilder<Local>, _: Value) -> Value {
        builder.ins().iconst(I32, -1)
    }
    fn translate_current_memory(&mut self, builder: &mut FunctionBuilder<Local>) -> Value {
        builder.ins().iconst(I32, -1)
    }
    fn translate_call_indirect<'a>(
        &self,
        builder: &'a mut FunctionBuilder<Local>,
        sig_ref: SigRef,
        index_val: Value,
        call_args: &[Value],
    ) -> &'a [Value] {
        let call_inst = builder.ins().call_indirect(sig_ref, index_val, call_args);
        builder.inst_results(call_inst)
    }
    fn declare_global(&mut self, global: Global) {
        self.globals.push(global);
    }
    fn declare_table(&mut self, _: Table) {
        //We do nothing
    }
    fn declare_table_elements(&mut self, _: TableIndex, _: usize, _: &[FunctionIndex]) {
        //We do nothing
    }
    fn declare_memory(&mut self, _: Memory) {
        //We do nothing
    }
    fn declare_data_initialization(
        &mut self,
        _: MemoryIndex,
        _: usize,
        _: &[u8],
    ) -> Result<(), String> {
        // We do nothing
        Ok(())
    }

    fn begin_translation(&mut self) {
        // We do nothing
    }
    fn next_function(&mut self) {
        // We do nothing
    }
}
