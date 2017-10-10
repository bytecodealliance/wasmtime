use runtime::{FuncEnvironment, GlobalValue, WasmRuntime};
use translation_utils::{Global, Memory, Table, GlobalIndex, TableIndex, SignatureIndex,
                        FunctionIndex, MemoryIndex};
use cretonne::ir::{self, InstBuilder};
use cretonne::ir::types::*;
use cretonne::cursor::FuncCursor;
use cretonne::settings;

/// A collection of names under which a given entity is exported.
pub struct Exportable<T> {
    /// A wasm entity.
    pub entity: T,

    /// Names under which the entity is exported.
    pub export_names: Vec<String>,
}

impl<T> Exportable<T> {
    pub fn new(entity: T) -> Self {
        Self {
            entity,
            export_names: Vec::new(),
        }
    }
}

/// This runtime implementation is a "naïve" one, doing essentially nothing and emitting
/// placeholders when forced to. Don't try to execute code translated with this runtime, it is
/// essentially here for translation debug purposes.
pub struct DummyRuntime {
    /// Compilation setting flags.
    pub flags: settings::Flags,

    /// Signatures as provided by `declare_signature`.
    pub signatures: Vec<ir::Signature>,

    /// Module and field names of imported functions as provided by `declare_func_import`.
    pub imported_funcs: Vec<(String, String)>,

    /// Functions, imported and local.
    pub functions: Vec<Exportable<SignatureIndex>>,

    /// Tables as provided by `declare_table`.
    pub tables: Vec<Exportable<Table>>,

    /// Memories as provided by `declare_memory`.
    pub memories: Vec<Exportable<Memory>>,

    /// Globals as provided by `declare_global`.
    pub globals: Vec<Exportable<Global>>,

    /// The start function.
    pub start_func: Option<FunctionIndex>,
}

impl DummyRuntime {
    /// Allocates the runtime data structures with default flags.
    pub fn default() -> Self {
        Self::with_flags(settings::Flags::new(&settings::builder()))
    }

    /// Allocates the runtime data structures with the given flags.
    pub fn with_flags(flags: settings::Flags) -> Self {
        Self {
            flags,
            signatures: Vec::new(),
            imported_funcs: Vec::new(),
            functions: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            globals: Vec::new(),
            start_func: None,
        }
    }
}

impl FuncEnvironment for DummyRuntime {
    fn flags(&self) -> &settings::Flags {
        &self.flags
    }

    fn make_global(&mut self, func: &mut ir::Function, index: GlobalIndex) -> GlobalValue {
        // Just create a dummy `vmctx` global.
        let offset = ((index * 8) as i32 + 8).into();
        let gv = func.create_global_var(ir::GlobalVarData::VmCtx { offset });
        GlobalValue::Memory {
            gv,
            ty: self.globals[index].entity.ty,
        }
    }

    fn make_heap(&mut self, func: &mut ir::Function, _index: MemoryIndex) -> ir::Heap {
        func.create_heap(ir::HeapData {
            base: ir::HeapBase::ReservedReg,
            min_size: 0.into(),
            guard_size: 0x8000_0000.into(),
            style: ir::HeapStyle::Static { bound: 0x1_0000_0000.into() },
        })
    }

    fn make_indirect_sig(&mut self, func: &mut ir::Function, index: SignatureIndex) -> ir::SigRef {
        // A real implementation would probably change the calling convention and add `vmctx` and
        // signature index arguments.
        func.import_signature(self.signatures[index].clone())
    }

    fn make_direct_func(&mut self, func: &mut ir::Function, index: FunctionIndex) -> ir::FuncRef {
        let sigidx = self.functions[index].entity;
        // A real implementation would probably add a `vmctx` argument.
        // And maybe attempt some signature de-duplication.
        let signature = func.import_signature(self.signatures[sigidx].clone());
        let name = self.get_name(index);
        func.import_function(ir::ExtFuncData { name, signature })
    }

    fn translate_call_indirect(
        &mut self,
        mut pos: FuncCursor,
        _table_index: TableIndex,
        _sig_index: SignatureIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> ir::Inst {
        pos.ins().call_indirect(sig_ref, callee, call_args)
    }

    fn translate_grow_memory(
        &mut self,
        mut pos: FuncCursor,
        _index: MemoryIndex,
        _heap: ir::Heap,
        _val: ir::Value,
    ) -> ir::Value {
        pos.ins().iconst(I32, -1)
    }

    fn translate_current_memory(
        &mut self,
        mut pos: FuncCursor,
        _index: MemoryIndex,
        _heap: ir::Heap,
    ) -> ir::Value {
        pos.ins().iconst(I32, -1)
    }
}

impl WasmRuntime for DummyRuntime {
    fn get_name(&self, func_index: FunctionIndex) -> ir::FunctionName {
        ir::FunctionName::new(format!("wasm_0x{:x}", func_index))
    }

    fn declare_signature(&mut self, sig: &ir::Signature) {
        self.signatures.push(sig.clone());
    }

    fn get_signature(&self, sig_index: SignatureIndex) -> &ir::Signature {
        &self.signatures[sig_index]
    }

    fn declare_func_import(&mut self, sig_index: SignatureIndex, module: &str, field: &str) {
        assert_eq!(
            self.functions.len(),
            self.imported_funcs.len(),
            "Imported functions must be declared first"
        );
        self.functions.push(Exportable::new(sig_index));
        self.imported_funcs.push((
            String::from(module),
            String::from(field),
        ));
    }

    fn get_num_func_imports(&self) -> usize {
        self.imported_funcs.len()
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) {
        self.functions.push(Exportable::new(sig_index));
    }

    fn get_func_type(&self, func_index: FunctionIndex) -> SignatureIndex {
        self.functions[func_index].entity
    }

    fn declare_global(&mut self, global: Global) {
        self.globals.push(Exportable::new(global));
    }

    fn get_global(&self, global_index: GlobalIndex) -> &Global {
        &self.globals[global_index].entity
    }

    fn declare_table(&mut self, table: Table) {
        self.tables.push(Exportable::new(table));
    }
    fn declare_table_elements(
        &mut self,
        _table_index: TableIndex,
        _base: Option<GlobalIndex>,
        _offset: usize,
        _elements: &[FunctionIndex],
    ) {
        // We do nothing
    }
    fn declare_memory(&mut self, memory: Memory) {
        self.memories.push(Exportable::new(memory));
    }
    fn declare_data_initialization(
        &mut self,
        _memory_index: MemoryIndex,
        _base: Option<GlobalIndex>,
        _offset: usize,
        _data: &[u8],
    ) {
        // We do nothing
    }

    fn declare_func_export(&mut self, func_index: FunctionIndex, name: &str) {
        self.functions[func_index].export_names.push(
            String::from(name),
        );
    }

    fn declare_table_export(&mut self, table_index: TableIndex, name: &str) {
        self.tables[table_index].export_names.push(
            String::from(name),
        );
    }

    fn declare_memory_export(&mut self, memory_index: MemoryIndex, name: &str) {
        self.memories[memory_index].export_names.push(
            String::from(name),
        );
    }

    fn declare_global_export(&mut self, global_index: GlobalIndex, name: &str) {
        self.globals[global_index].export_names.push(
            String::from(name),
        );
    }

    fn declare_start_func(&mut self, func_index: FunctionIndex) {
        debug_assert!(self.start_func.is_none());
        self.start_func = Some(func_index);
    }

    fn begin_translation(&mut self) {
        // We do nothing
    }
    fn next_function(&mut self) {
        // We do nothing
    }
}
