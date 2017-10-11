use runtime::{FuncEnvironment, GlobalValue, ModuleEnvironment};
use translation_utils::{Global, Memory, Table, GlobalIndex, TableIndex, SignatureIndex,
                        FunctionIndex, MemoryIndex};
use func_translator::FuncTranslator;
use cretonne::ir::{self, InstBuilder};
use cretonne::ir::types::*;
use cretonne::cursor::FuncCursor;
use cretonne::settings;
use wasmparser;
use std::error::Error;

/// Compute a `ir::FunctionName` for a given wasm function index.
fn get_func_name(func_index: FunctionIndex) -> ir::FunctionName {
    ir::FunctionName::new(format!("wasm_0x{:x}", func_index))
}

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

/// The main state belonging to a `DummyEnvironment`. This is split out from
/// `DummyEnvironment` to allow it to be borrowed separately from the
/// `FuncTranslator` field.
pub struct DummyModuleInfo {
    /// Compilation setting flags.
    pub flags: settings::Flags,

    /// Signatures as provided by `declare_signature`.
    pub signatures: Vec<ir::Signature>,

    /// Module and field names of imported functions as provided by `declare_func_import`.
    pub imported_funcs: Vec<(String, String)>,

    /// Functions, imported and local.
    pub functions: Vec<Exportable<SignatureIndex>>,

    /// Function bodies.
    pub function_bodies: Vec<ir::Function>,

    /// Tables as provided by `declare_table`.
    pub tables: Vec<Exportable<Table>>,

    /// Memories as provided by `declare_memory`.
    pub memories: Vec<Exportable<Memory>>,

    /// Globals as provided by `declare_global`.
    pub globals: Vec<Exportable<Global>>,

    /// The start function.
    pub start_func: Option<FunctionIndex>,
}

impl DummyModuleInfo {
    /// Allocates the runtime data structures with the given flags.
    pub fn with_flags(flags: settings::Flags) -> Self {
        Self {
            flags,
            signatures: Vec::new(),
            imported_funcs: Vec::new(),
            functions: Vec::new(),
            function_bodies: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            globals: Vec::new(),
            start_func: None,
        }
    }
}

/// This runtime implementation is a "naïve" one, doing essentially nothing and emitting
/// placeholders when forced to. Don't try to execute code translated with this runtime, it is
/// essentially here for translation debug purposes.
pub struct DummyEnvironment {
    /// Module information.
    pub info: DummyModuleInfo,

    /// Function translation.
    trans: FuncTranslator,
}

impl DummyEnvironment {
    /// Allocates the runtime data structures with default flags.
    pub fn default() -> Self {
        Self::with_flags(settings::Flags::new(&settings::builder()))
    }

    /// Allocates the runtime data structures with the given flags.
    pub fn with_flags(flags: settings::Flags) -> Self {
        Self {
            info: DummyModuleInfo::with_flags(flags),
            trans: FuncTranslator::new(),
        }
    }

    /// Return a `DummyFuncEnvironment` for translating functions within this
    /// `DummyEnvironment`.
    pub fn func_env(&self) -> DummyFuncEnvironment {
        DummyFuncEnvironment::new(&self.info)
    }
}

/// The FuncEnvironment implementation for use by the `DummyEnvironment`.
pub struct DummyFuncEnvironment<'dummy_environment> {
    pub mod_info: &'dummy_environment DummyModuleInfo,
}

impl<'dummy_environment> DummyFuncEnvironment<'dummy_environment> {
    pub fn new(mod_info: &'dummy_environment DummyModuleInfo) -> Self {
        Self { mod_info }
    }
}

impl<'dummy_environment> FuncEnvironment for DummyFuncEnvironment<'dummy_environment> {
    fn flags(&self) -> &settings::Flags {
        &self.mod_info.flags
    }

    fn make_global(&mut self, func: &mut ir::Function, index: GlobalIndex) -> GlobalValue {
        // Just create a dummy `vmctx` global.
        let offset = ((index * 8) as i32 + 8).into();
        let gv = func.create_global_var(ir::GlobalVarData::VmCtx { offset });
        GlobalValue::Memory {
            gv,
            ty: self.mod_info.globals[index].entity.ty,
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
        func.import_signature(self.mod_info.signatures[index].clone())
    }

    fn make_direct_func(&mut self, func: &mut ir::Function, index: FunctionIndex) -> ir::FuncRef {
        let sigidx = self.mod_info.functions[index].entity;
        // A real implementation would probably add a `vmctx` argument.
        // And maybe attempt some signature de-duplication.
        let signature = func.import_signature(self.mod_info.signatures[sigidx].clone());
        let name = get_func_name(index);
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

impl ModuleEnvironment for DummyEnvironment {
    fn get_func_name(&self, func_index: FunctionIndex) -> ir::FunctionName {
        get_func_name(func_index)
    }

    fn declare_signature(&mut self, sig: &ir::Signature) {
        self.info.signatures.push(sig.clone());
    }

    fn get_signature(&self, sig_index: SignatureIndex) -> &ir::Signature {
        &self.info.signatures[sig_index]
    }

    fn declare_func_import<'data>(
        &mut self,
        sig_index: SignatureIndex,
        module: &'data str,
        field: &'data str,
    ) {
        assert_eq!(
            self.info.functions.len(),
            self.info.imported_funcs.len(),
            "Imported functions must be declared first"
        );
        self.info.functions.push(Exportable::new(sig_index));
        self.info.imported_funcs.push((
            String::from(module),
            String::from(field),
        ));
    }

    fn get_num_func_imports(&self) -> usize {
        self.info.imported_funcs.len()
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) {
        self.info.functions.push(Exportable::new(sig_index));
    }

    fn get_func_type(&self, func_index: FunctionIndex) -> SignatureIndex {
        self.info.functions[func_index].entity
    }

    fn declare_global(&mut self, global: Global) {
        self.info.globals.push(Exportable::new(global));
    }

    fn get_global(&self, global_index: GlobalIndex) -> &Global {
        &self.info.globals[global_index].entity
    }

    fn declare_table(&mut self, table: Table) {
        self.info.tables.push(Exportable::new(table));
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
        self.info.memories.push(Exportable::new(memory));
    }
    fn declare_data_initialization<'data>(
        &mut self,
        _memory_index: MemoryIndex,
        _base: Option<GlobalIndex>,
        _offset: usize,
        _data: &'data [u8],
    ) {
        // We do nothing
    }

    fn declare_func_export<'data>(&mut self, func_index: FunctionIndex, name: &'data str) {
        self.info.functions[func_index].export_names.push(
            String::from(
                name,
            ),
        );
    }

    fn declare_table_export<'data>(&mut self, table_index: TableIndex, name: &'data str) {
        self.info.tables[table_index].export_names.push(
            String::from(name),
        );
    }

    fn declare_memory_export<'data>(&mut self, memory_index: MemoryIndex, name: &'data str) {
        self.info.memories[memory_index].export_names.push(
            String::from(
                name,
            ),
        );
    }

    fn declare_global_export<'data>(&mut self, global_index: GlobalIndex, name: &'data str) {
        self.info.globals[global_index].export_names.push(
            String::from(
                name,
            ),
        );
    }

    fn declare_start_func(&mut self, func_index: FunctionIndex) {
        debug_assert!(self.info.start_func.is_none());
        self.info.start_func = Some(func_index);
    }

    /// Provides the contents of a function body.
    fn define_function_body<'data>(&mut self, body_bytes: &'data [u8]) -> Result<(), String> {
        let function_index = self.get_num_func_imports() + self.info.function_bodies.len();
        let name = get_func_name(function_index);
        let sig = self.get_signature(self.get_func_type(function_index))
            .clone();
        let mut func = ir::Function::with_name_signature(name, sig);
        {
            let mut func_environ = DummyFuncEnvironment::new(&self.info);
            let reader = wasmparser::BinaryReader::new(body_bytes);
            self.trans
                .translate_from_reader(reader, &mut func, &mut func_environ)
                .map_err(|e| String::from(e.description()))?;
        }
        self.info.function_bodies.push(func);
        Ok(())
    }
}
