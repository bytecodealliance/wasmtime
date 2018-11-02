//! "Dummy" implementations of `ModuleEnvironment` and `FuncEnvironment` for testing
//! wasm translation.

use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::immediates::{Imm64, Offset32};
use cranelift_codegen::ir::types::*;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_entity::{EntityRef, PrimaryMap};
use environ::{FuncEnvironment, GlobalVariable, ModuleEnvironment, ReturnMode, WasmResult};
use func_translator::FuncTranslator;
use std::string::String;
use std::vec::Vec;
use translation_utils::{
    DefinedFuncIndex, FuncIndex, Global, GlobalIndex, Memory, MemoryIndex, SignatureIndex, Table,
    TableIndex,
};

/// Compute a `ir::ExternalName` for a given wasm function index.
fn get_func_name(func_index: FuncIndex) -> ir::ExternalName {
    ir::ExternalName::user(0, func_index.index() as u32)
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
    /// Target description relevant to frontends producing Cranelift IR.
    config: TargetFrontendConfig,

    /// Signatures as provided by `declare_signature`.
    pub signatures: PrimaryMap<SignatureIndex, ir::Signature>,

    /// Module and field names of imported functions as provided by `declare_func_import`.
    pub imported_funcs: Vec<(String, String)>,

    /// Functions, imported and local.
    pub functions: PrimaryMap<FuncIndex, Exportable<SignatureIndex>>,

    /// Function bodies.
    pub function_bodies: PrimaryMap<DefinedFuncIndex, ir::Function>,

    /// Tables as provided by `declare_table`.
    pub tables: PrimaryMap<TableIndex, Exportable<Table>>,

    /// Memories as provided by `declare_memory`.
    pub memories: PrimaryMap<MemoryIndex, Exportable<Memory>>,

    /// Globals as provided by `declare_global`.
    pub globals: PrimaryMap<GlobalIndex, Exportable<Global>>,

    /// The start function.
    pub start_func: Option<FuncIndex>,
}

impl DummyModuleInfo {
    /// Creates a new `DummyModuleInfo` instance.
    pub fn new(config: TargetFrontendConfig) -> Self {
        Self {
            config,
            signatures: PrimaryMap::new(),
            imported_funcs: Vec::new(),
            functions: PrimaryMap::new(),
            function_bodies: PrimaryMap::new(),
            tables: PrimaryMap::new(),
            memories: PrimaryMap::new(),
            globals: PrimaryMap::new(),
            start_func: None,
        }
    }
}

/// This `ModuleEnvironment` implementation is a "naïve" one, doing essentially nothing and
/// emitting placeholders when forced to. Don't try to execute code translated for this
/// environment, essentially here for translation debug purposes.
pub struct DummyEnvironment {
    /// Module information.
    pub info: DummyModuleInfo,

    /// Function translation.
    trans: FuncTranslator,

    /// Vector of wasm bytecode size for each function.
    pub func_bytecode_sizes: Vec<usize>,

    /// How to return from functions.
    return_mode: ReturnMode,
}

impl DummyEnvironment {
    /// Creates a new `DummyEnvironment` instance.
    pub fn new(config: TargetFrontendConfig, return_mode: ReturnMode) -> Self {
        Self {
            info: DummyModuleInfo::new(config),
            trans: FuncTranslator::new(),
            func_bytecode_sizes: Vec::new(),
            return_mode,
        }
    }

    /// Return a `DummyFuncEnvironment` for translating functions within this
    /// `DummyEnvironment`.
    pub fn func_env(&self) -> DummyFuncEnvironment {
        DummyFuncEnvironment::new(&self.info, self.return_mode)
    }
}

/// The `FuncEnvironment` implementation for use by the `DummyEnvironment`.
pub struct DummyFuncEnvironment<'dummy_environment> {
    pub mod_info: &'dummy_environment DummyModuleInfo,

    return_mode: ReturnMode,
}

impl<'dummy_environment> DummyFuncEnvironment<'dummy_environment> {
    pub fn new(mod_info: &'dummy_environment DummyModuleInfo, return_mode: ReturnMode) -> Self {
        Self {
            mod_info,
            return_mode,
        }
    }

    // Create a signature for `sigidx` amended with a `vmctx` argument after the standard wasm
    // arguments.
    fn vmctx_sig(&self, sigidx: SignatureIndex) -> ir::Signature {
        let mut sig = self.mod_info.signatures[sigidx].clone();
        sig.params.push(ir::AbiParam::special(
            self.pointer_type(),
            ir::ArgumentPurpose::VMContext,
        ));
        sig
    }
}

impl<'dummy_environment> FuncEnvironment for DummyFuncEnvironment<'dummy_environment> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.mod_info.config
    }

    fn make_global(&mut self, func: &mut ir::Function, index: GlobalIndex) -> GlobalVariable {
        // Just create a dummy `vmctx` global.
        let offset = ((index.index() * 8) as i64 + 8).into();
        let vmctx = func.create_global_value(ir::GlobalValueData::VMContext {});
        let iadd = func.create_global_value(ir::GlobalValueData::IAddImm {
            base: vmctx,
            offset,
            global_type: self.pointer_type(),
        });
        GlobalVariable::Memory {
            gv: iadd,
            ty: self.mod_info.globals[index].entity.ty,
        }
    }

    fn make_heap(&mut self, func: &mut ir::Function, _index: MemoryIndex) -> ir::Heap {
        // Create a static heap whose base address is stored at `vmctx+0`.
        let addr = func.create_global_value(ir::GlobalValueData::VMContext);
        let gv = func.create_global_value(ir::GlobalValueData::Load {
            base: addr,
            offset: Offset32::new(0),
            global_type: self.pointer_type(),
            readonly: true,
        });

        func.create_heap(ir::HeapData {
            base: gv,
            min_size: 0.into(),
            guard_size: 0x8000_0000.into(),
            style: ir::HeapStyle::Static {
                bound: 0x1_0000_0000.into(),
            },
            index_type: I32,
        })
    }

    fn make_table(&mut self, func: &mut ir::Function, _index: TableIndex) -> ir::Table {
        // Create a table whose base address is stored at `vmctx+0`.
        let vmctx = func.create_global_value(ir::GlobalValueData::VMContext);
        let base_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: vmctx,
            offset: Offset32::new(0),
            global_type: self.pointer_type(),
            readonly: true, // when tables in wasm become "growable", revisit whether this can be readonly or not.
        });
        let bound_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: vmctx,
            offset: Offset32::new(0),
            global_type: I32,
            readonly: true,
        });

        func.create_table(ir::TableData {
            base_gv,
            min_size: Imm64::new(0),
            bound_gv,
            element_size: Imm64::new(i64::from(self.pointer_bytes()) * 2),
            index_type: I32,
        })
    }

    fn make_indirect_sig(&mut self, func: &mut ir::Function, index: SignatureIndex) -> ir::SigRef {
        // A real implementation would probably change the calling convention and add `vmctx` and
        // signature index arguments.
        func.import_signature(self.vmctx_sig(index))
    }

    fn make_direct_func(&mut self, func: &mut ir::Function, index: FuncIndex) -> ir::FuncRef {
        let sigidx = self.mod_info.functions[index].entity;
        // A real implementation would probably add a `vmctx` argument.
        // And maybe attempt some signature de-duplication.
        let signature = func.import_signature(self.vmctx_sig(sigidx));
        let name = get_func_name(index);
        func.import_function(ir::ExtFuncData {
            name,
            signature,
            colocated: false,
        })
    }

    fn translate_call_indirect(
        &mut self,
        mut pos: FuncCursor,
        _table_index: TableIndex,
        _table: ir::Table,
        _sig_index: SignatureIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        // Pass the current function's vmctx parameter on to the callee.
        let vmctx = pos
            .func
            .special_param(ir::ArgumentPurpose::VMContext)
            .expect("Missing vmctx parameter");

        // The `callee` value is an index into a table of function pointers.
        // Apparently, that table is stored at absolute address 0 in this dummy environment.
        // TODO: Generate bounds checking code.
        let ptr = self.pointer_type();
        let callee_offset = if ptr == I32 {
            pos.ins().imul_imm(callee, 4)
        } else {
            let ext = pos.ins().uextend(I64, callee);
            pos.ins().imul_imm(ext, 4)
        };
        let mut mflags = ir::MemFlags::new();
        mflags.set_notrap();
        mflags.set_aligned();
        let func_ptr = pos.ins().load(ptr, mflags, callee_offset, 0);

        // Build a value list for the indirect call instruction containing the callee, call_args,
        // and the vmctx parameter.
        let mut args = ir::ValueList::default();
        args.push(func_ptr, &mut pos.func.dfg.value_lists);
        args.extend(call_args.iter().cloned(), &mut pos.func.dfg.value_lists);
        args.push(vmctx, &mut pos.func.dfg.value_lists);

        Ok(pos
            .ins()
            .CallIndirect(ir::Opcode::CallIndirect, INVALID, sig_ref, args)
            .0)
    }

    fn translate_call(
        &mut self,
        mut pos: FuncCursor,
        _callee_index: FuncIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        // Pass the current function's vmctx parameter on to the callee.
        let vmctx = pos
            .func
            .special_param(ir::ArgumentPurpose::VMContext)
            .expect("Missing vmctx parameter");

        // Build a value list for the call instruction containing the call_args and the vmctx
        // parameter.
        let mut args = ir::ValueList::default();
        args.extend(call_args.iter().cloned(), &mut pos.func.dfg.value_lists);
        args.push(vmctx, &mut pos.func.dfg.value_lists);

        Ok(pos.ins().Call(ir::Opcode::Call, INVALID, callee, args).0)
    }

    fn translate_memory_grow(
        &mut self,
        mut pos: FuncCursor,
        _index: MemoryIndex,
        _heap: ir::Heap,
        _val: ir::Value,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, -1))
    }

    fn translate_memory_size(
        &mut self,
        mut pos: FuncCursor,
        _index: MemoryIndex,
        _heap: ir::Heap,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, -1))
    }

    fn return_mode(&self) -> ReturnMode {
        self.return_mode
    }
}

impl<'data> ModuleEnvironment<'data> for DummyEnvironment {
    fn target_config(&self) -> &TargetFrontendConfig {
        &self.info.config
    }

    fn get_func_name(&self, func_index: FuncIndex) -> ir::ExternalName {
        get_func_name(func_index)
    }

    fn declare_signature(&mut self, sig: &ir::Signature) {
        self.info.signatures.push(sig.clone());
    }

    fn get_signature(&self, sig_index: SignatureIndex) -> &ir::Signature {
        &self.info.signatures[sig_index]
    }

    fn declare_func_import(
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
        self.info
            .imported_funcs
            .push((String::from(module), String::from(field)));
    }

    fn get_num_func_imports(&self) -> usize {
        self.info.imported_funcs.len()
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) {
        self.info.functions.push(Exportable::new(sig_index));
    }

    fn get_func_type(&self, func_index: FuncIndex) -> SignatureIndex {
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
        _elements: Vec<FuncIndex>,
    ) {
        // We do nothing
    }
    fn declare_memory(&mut self, memory: Memory) {
        self.info.memories.push(Exportable::new(memory));
    }
    fn declare_data_initialization(
        &mut self,
        _memory_index: MemoryIndex,
        _base: Option<GlobalIndex>,
        _offset: usize,
        _data: &'data [u8],
    ) {
        // We do nothing
    }

    fn declare_func_export(&mut self, func_index: FuncIndex, name: &'data str) {
        self.info.functions[func_index]
            .export_names
            .push(String::from(name));
    }

    fn declare_table_export(&mut self, table_index: TableIndex, name: &'data str) {
        self.info.tables[table_index]
            .export_names
            .push(String::from(name));
    }

    fn declare_memory_export(&mut self, memory_index: MemoryIndex, name: &'data str) {
        self.info.memories[memory_index]
            .export_names
            .push(String::from(name));
    }

    fn declare_global_export(&mut self, global_index: GlobalIndex, name: &'data str) {
        self.info.globals[global_index]
            .export_names
            .push(String::from(name));
    }

    fn declare_start_func(&mut self, func_index: FuncIndex) {
        debug_assert!(self.info.start_func.is_none());
        self.info.start_func = Some(func_index);
    }

    fn define_function_body(&mut self, body_bytes: &'data [u8]) -> WasmResult<()> {
        let func = {
            let mut func_environ = DummyFuncEnvironment::new(&self.info, self.return_mode);
            let func_index =
                FuncIndex::new(self.get_num_func_imports() + self.info.function_bodies.len());
            let name = get_func_name(func_index);
            let sig = func_environ.vmctx_sig(self.get_func_type(func_index));
            let mut func = ir::Function::with_name_signature(name, sig);
            self.trans
                .translate(body_bytes, &mut func, &mut func_environ)?;
            func
        };
        self.func_bytecode_sizes.push(body_bytes.len());
        self.info.function_bodies.push(func);
        Ok(())
    }
}
