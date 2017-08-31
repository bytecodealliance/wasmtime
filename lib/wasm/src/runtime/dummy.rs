use runtime::WasmRuntime;
use translation_utils::{Local, Global, Memory, Table, GlobalIndex, TableIndex, FunctionIndex,
                        MemoryIndex};
use cton_frontend::FunctionBuilder;
use cretonne::ir::{Value, InstBuilder, SigRef};
use cretonne::ir::immediates::{Ieee32, Ieee64};
use cretonne::ir::types::*;

/// This runtime implementation is a "naïve" one, doing essentially nothing and emitting
/// placeholders when forced to. Don't try to execute code translated with this runtime, it is
/// essentially here for translation debug purposes.
pub struct DummyRuntime {
    globals: Vec<Global>,
}

impl DummyRuntime {
    /// Allocates the runtime data structures.
    pub fn new() -> DummyRuntime {
        DummyRuntime { globals: Vec::new() }
    }
}

impl WasmRuntime for DummyRuntime {
    fn translate_get_global(
        &self,
        builder: &mut FunctionBuilder<Local>,
        global_index: GlobalIndex,
    ) -> Value {
        let ref glob = self.globals.get(global_index as usize).unwrap();
        match glob.ty {
            I32 => builder.ins().iconst(glob.ty, -1),
            I64 => builder.ins().iconst(glob.ty, -1),
            F32 => builder.ins().f32const(Ieee32::with_bits(0xbf800000)), // -1.0
            F64 => {
                builder.ins().f64const(
                    Ieee64::with_bits(0xbff0000000000000),
                )
            } // -1.0
            _ => panic!("should not happen"),
        }
    }

    fn translate_set_global(&self, _: &mut FunctionBuilder<Local>, _: GlobalIndex, _: Value) {
        // We do nothing
    }
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
    fn translate_memory_base_address(
        &self,
        builder: &mut FunctionBuilder<Local>,
        _: MemoryIndex,
    ) -> Value {
        builder.ins().iconst(I64, 0)
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
