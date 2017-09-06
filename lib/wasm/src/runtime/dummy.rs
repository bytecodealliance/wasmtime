use runtime::{FuncEnvironment, GlobalValue, WasmRuntime};
use translation_utils::{Local, Global, Memory, Table, GlobalIndex, TableIndex, SignatureIndex,
                        FunctionIndex, MemoryIndex};
use cton_frontend::FunctionBuilder;
use cretonne::ir::{self, Value, InstBuilder};
use cretonne::ir::types::*;
use cretonne::cursor::FuncCursor;

/// This runtime implementation is a "naïve" one, doing essentially nothing and emitting
/// placeholders when forced to. Don't try to execute code translated with this runtime, it is
/// essentially here for translation debug purposes.
pub struct DummyRuntime {
    // Unprocessed signatures exactly as provided by `declare_signature()`.
    signatures: Vec<ir::Signature>,
    globals: Vec<Global>,

    // Types of functions, imported and local.
    func_types: Vec<SignatureIndex>,

    // Names of imported functions.
    imported_funcs: Vec<ir::FunctionName>,
}

impl DummyRuntime {
    /// Allocates the runtime data structures.
    pub fn new() -> Self {
        Self {
            signatures: Vec::new(),
            globals: Vec::new(),
            func_types: Vec::new(),
            imported_funcs: Vec::new(),
        }
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

    fn make_indirect_sig(&self, func: &mut ir::Function, index: SignatureIndex) -> ir::SigRef {
        // A real implementation would probably change the calling convention and add `vmctx` and
        // signature index arguments.
        func.dfg.signatures.push(self.signatures[index].clone())
    }

    fn make_direct_func(&self, func: &mut ir::Function, index: FunctionIndex) -> ir::FuncRef {
        let sigidx = self.func_types[index];
        // A real implementation would probably add a `vmctx` argument.
        // And maybe attempt some signature de-duplication.
        let signature = func.dfg.signatures.push(self.signatures[sigidx].clone());

        let name = match self.imported_funcs.get(index) {
            Some(name) => name.clone(),
            None => ir::FunctionName::new(format!("localfunc{}", index)),
        };

        func.dfg.ext_funcs.push(ir::ExtFuncData { name, signature })
    }

    fn translate_call_indirect(
        &self,
        mut pos: FuncCursor,
        _table_index: TableIndex,
        _sig_index: SignatureIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> ir::Inst {
        pos.ins().call_indirect(sig_ref, callee, call_args)
    }
}

impl WasmRuntime for DummyRuntime {
    fn translate_grow_memory(&mut self, builder: &mut FunctionBuilder<Local>, _: Value) -> Value {
        builder.ins().iconst(I32, -1)
    }
    fn translate_current_memory(&mut self, builder: &mut FunctionBuilder<Local>) -> Value {
        builder.ins().iconst(I32, -1)
    }

    fn declare_signature(&mut self, sig: &ir::Signature) {
        self.signatures.push(sig.clone());
    }

    fn declare_func_import(&mut self, sig_index: SignatureIndex, module: &[u8], field: &[u8]) {
        assert_eq!(
            self.func_types.len(),
            self.imported_funcs.len(),
            "Imported functions must be declared first"
        );
        self.func_types.push(sig_index);

        let mut name = Vec::new();
        name.extend(module.iter().cloned().map(name_fold));
        name.push(b'_');
        name.extend(field.iter().cloned().map(name_fold));
        self.imported_funcs.push(ir::FunctionName::new(name));
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) {
        self.func_types.push(sig_index);
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

// Generate characters suitable for printable `FuncName`s.
fn name_fold(c: u8) -> u8 {
    if (c as char).is_alphanumeric() {
        c
    } else {
        b'_'
    }
}
