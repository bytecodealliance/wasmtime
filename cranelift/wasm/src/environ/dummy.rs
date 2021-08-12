//! "Dummy" implementations of `ModuleEnvironment` and `FuncEnvironment` for testing
//! wasm translation. For complete implementations of `ModuleEnvironment` and
//! `FuncEnvironment`, see [wasmtime-environ] in [Wasmtime].
//!
//! [wasmtime-environ]: https://crates.io/crates/wasmtime-environ
//! [Wasmtime]: https://github.com/bytecodealliance/wasmtime

use crate::environ::{
    FuncEnvironment, GlobalVariable, ModuleEnvironment, ReturnMode, TargetEnvironment,
    WasmFuncType, WasmResult,
};
use crate::func_translator::FuncTranslator;
use crate::state::FuncTranslationState;
use crate::translation_utils::{
    DataIndex, DefinedFuncIndex, ElemIndex, FuncIndex, Global, GlobalIndex, Memory, MemoryIndex,
    Table, TableIndex, TypeIndex,
};
use crate::WasmType;
use core::convert::TryFrom;
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::immediates::{Offset32, Uimm64};
use cranelift_codegen::ir::types::*;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_codegen::isa::{CallConv, TargetFrontendConfig};
use cranelift_entity::{EntityRef, PrimaryMap, SecondaryMap};
use cranelift_frontend::FunctionBuilder;
use std::boxed::Box;
use std::string::String;
use std::vec::Vec;
use wasmparser::{FuncValidator, FunctionBody, Operator, ValidatorResources, WasmFeatures};

/// Compute a `ir::ExternalName` for a given wasm function index.
fn get_func_name(func_index: FuncIndex) -> ir::ExternalName {
    ir::ExternalName::user(0, func_index.as_u32())
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
    pub signatures: PrimaryMap<TypeIndex, ir::Signature>,

    /// Module and field names of imported functions as provided by `declare_func_import`.
    pub imported_funcs: Vec<(String, String)>,

    /// Module and field names of imported globals as provided by `declare_global_import`.
    pub imported_globals: Vec<(String, String)>,

    /// Module and field names of imported tables as provided by `declare_table_import`.
    pub imported_tables: Vec<(String, String)>,

    /// Module and field names of imported memories as provided by `declare_memory_import`.
    pub imported_memories: Vec<(String, String)>,

    /// Functions, imported and local.
    pub functions: PrimaryMap<FuncIndex, Exportable<TypeIndex>>,

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
            imported_globals: Vec::new(),
            imported_tables: Vec::new(),
            imported_memories: Vec::new(),
            functions: PrimaryMap::new(),
            function_bodies: PrimaryMap::new(),
            tables: PrimaryMap::new(),
            memories: PrimaryMap::new(),
            globals: PrimaryMap::new(),
            start_func: None,
        }
    }
}

/// State for tracking and checking reachability at each operator. Used for unit testing with the
/// `DummyEnvironment`.
#[derive(Clone)]
pub struct ExpectedReachability {
    /// Before- and after-reachability
    reachability: Vec<(bool, bool)>,
    before_idx: usize,
    after_idx: usize,
}

impl ExpectedReachability {
    fn check_before(&mut self, reachable: bool) {
        assert_eq!(reachable, self.reachability[self.before_idx].0);
        self.before_idx += 1;
    }
    fn check_after(&mut self, reachable: bool) {
        assert_eq!(reachable, self.reachability[self.after_idx].1);
        self.after_idx += 1;
    }
    fn check_end(&self) {
        assert_eq!(self.before_idx, self.reachability.len());
        assert_eq!(self.after_idx, self.reachability.len());
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

    /// Instructs to collect debug data during translation.
    debug_info: bool,

    /// Name of the module from the wasm file.
    pub module_name: Option<String>,

    /// Function names.
    function_names: SecondaryMap<FuncIndex, String>,

    /// Expected reachability data (before/after for each op) to assert. This is used for testing.
    expected_reachability: Option<ExpectedReachability>,
}

impl DummyEnvironment {
    /// Creates a new `DummyEnvironment` instance.
    pub fn new(config: TargetFrontendConfig, return_mode: ReturnMode, debug_info: bool) -> Self {
        Self {
            info: DummyModuleInfo::new(config),
            trans: FuncTranslator::new(),
            func_bytecode_sizes: Vec::new(),
            return_mode,
            debug_info,
            module_name: None,
            function_names: SecondaryMap::new(),
            expected_reachability: None,
        }
    }

    /// Return a `DummyFuncEnvironment` for translating functions within this
    /// `DummyEnvironment`.
    pub fn func_env(&self) -> DummyFuncEnvironment {
        DummyFuncEnvironment::new(
            &self.info,
            self.return_mode,
            self.expected_reachability.clone(),
        )
    }

    fn get_func_type(&self, func_index: FuncIndex) -> TypeIndex {
        self.info.functions[func_index].entity
    }

    /// Return the number of imported functions within this `DummyEnvironment`.
    pub fn get_num_func_imports(&self) -> usize {
        self.info.imported_funcs.len()
    }

    /// Return the name of the function, if a name for the function with
    /// the corresponding index exists.
    pub fn get_func_name(&self, func_index: FuncIndex) -> Option<&str> {
        self.function_names.get(func_index).map(String::as_ref)
    }

    /// Test reachability bits before and after every opcode during translation, as provided by the
    /// `FuncTranslationState`. This is generally used only for unit tests. This is applied to
    /// every function in the module (so is likely only useful for test modules with one function).
    pub fn test_expected_reachability(&mut self, reachability: Vec<(bool, bool)>) {
        self.expected_reachability = Some(ExpectedReachability {
            reachability,
            before_idx: 0,
            after_idx: 0,
        });
    }
}

/// The `FuncEnvironment` implementation for use by the `DummyEnvironment`.
pub struct DummyFuncEnvironment<'dummy_environment> {
    pub mod_info: &'dummy_environment DummyModuleInfo,

    return_mode: ReturnMode,

    /// Expected reachability data (before/after for each op) to assert. This is used for testing.
    expected_reachability: Option<ExpectedReachability>,
}

impl<'dummy_environment> DummyFuncEnvironment<'dummy_environment> {
    pub fn new(
        mod_info: &'dummy_environment DummyModuleInfo,
        return_mode: ReturnMode,
        expected_reachability: Option<ExpectedReachability>,
    ) -> Self {
        Self {
            mod_info,
            return_mode,
            expected_reachability,
        }
    }

    // Create a signature for `sigidx` amended with a `vmctx` argument after the standard wasm
    // arguments.
    fn vmctx_sig(&self, sigidx: TypeIndex) -> ir::Signature {
        let mut sig = self.mod_info.signatures[sigidx].clone();
        sig.params.push(ir::AbiParam::special(
            self.pointer_type(),
            ir::ArgumentPurpose::VMContext,
        ));
        sig
    }

    fn reference_type(&self) -> ir::Type {
        match self.pointer_type() {
            ir::types::I32 => ir::types::R32,
            ir::types::I64 => ir::types::R64,
            _ => panic!("unsupported pointer type"),
        }
    }
}

impl<'dummy_environment> TargetEnvironment for DummyFuncEnvironment<'dummy_environment> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.mod_info.config
    }
}

impl<'dummy_environment> FuncEnvironment for DummyFuncEnvironment<'dummy_environment> {
    fn return_mode(&self) -> ReturnMode {
        self.return_mode
    }

    fn make_global(
        &mut self,
        func: &mut ir::Function,
        index: GlobalIndex,
    ) -> WasmResult<GlobalVariable> {
        // Just create a dummy `vmctx` global.
        let offset = i32::try_from((index.index() * 8) + 8).unwrap().into();
        let vmctx = func.create_global_value(ir::GlobalValueData::VMContext {});
        Ok(GlobalVariable::Memory {
            gv: vmctx,
            offset,
            ty: self.mod_info.globals[index].entity.ty,
        })
    }

    fn make_heap(&mut self, func: &mut ir::Function, _index: MemoryIndex) -> WasmResult<ir::Heap> {
        // Create a static heap whose base address is stored at `vmctx+0`.
        let addr = func.create_global_value(ir::GlobalValueData::VMContext);
        let gv = func.create_global_value(ir::GlobalValueData::Load {
            base: addr,
            offset: Offset32::new(0),
            global_type: self.pointer_type(),
            readonly: true,
        });

        Ok(func.create_heap(ir::HeapData {
            base: gv,
            min_size: 0.into(),
            offset_guard_size: 0x8000_0000.into(),
            style: ir::HeapStyle::Static {
                bound: 0x1_0000_0000.into(),
            },
            index_type: I32,
        }))
    }

    fn make_table(&mut self, func: &mut ir::Function, _index: TableIndex) -> WasmResult<ir::Table> {
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

        Ok(func.create_table(ir::TableData {
            base_gv,
            min_size: Uimm64::new(0),
            bound_gv,
            element_size: Uimm64::from(u64::from(self.pointer_bytes()) * 2),
            index_type: I32,
        }))
    }

    fn make_indirect_sig(
        &mut self,
        func: &mut ir::Function,
        index: TypeIndex,
    ) -> WasmResult<ir::SigRef> {
        // A real implementation would probably change the calling convention and add `vmctx` and
        // signature index arguments.
        Ok(func.import_signature(self.vmctx_sig(index)))
    }

    fn make_direct_func(
        &mut self,
        func: &mut ir::Function,
        index: FuncIndex,
    ) -> WasmResult<ir::FuncRef> {
        let sigidx = self.mod_info.functions[index].entity;
        // A real implementation would probably add a `vmctx` argument.
        // And maybe attempt some signature de-duplication.
        let signature = func.import_signature(self.vmctx_sig(sigidx));
        let name = get_func_name(index);
        Ok(func.import_function(ir::ExtFuncData {
            name,
            signature,
            colocated: false,
        }))
    }

    fn before_translate_operator(
        &mut self,
        _op: &Operator,
        _builder: &mut FunctionBuilder,
        state: &FuncTranslationState,
    ) -> WasmResult<()> {
        if let Some(ref mut r) = &mut self.expected_reachability {
            r.check_before(state.reachable());
        }
        Ok(())
    }

    fn after_translate_operator(
        &mut self,
        _op: &Operator,
        _builder: &mut FunctionBuilder,
        state: &FuncTranslationState,
    ) -> WasmResult<()> {
        if let Some(ref mut r) = &mut self.expected_reachability {
            r.check_after(state.reachable());
        }
        Ok(())
    }

    fn after_translate_function(
        &mut self,
        _builder: &mut FunctionBuilder,
        _state: &FuncTranslationState,
    ) -> WasmResult<()> {
        if let Some(ref mut r) = &mut self.expected_reachability {
            r.check_end();
        }
        Ok(())
    }

    fn translate_call_indirect(
        &mut self,
        mut pos: FuncCursor,
        _table_index: TableIndex,
        _table: ir::Table,
        _sig_index: TypeIndex,
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
        let mflags = ir::MemFlags::trusted();
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

    fn translate_memory_copy(
        &mut self,
        _pos: FuncCursor,
        _src_index: MemoryIndex,
        _src_heap: ir::Heap,
        _dst_index: MemoryIndex,
        _dst_heap: ir::Heap,
        _dst: ir::Value,
        _src: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Ok(())
    }

    fn translate_memory_fill(
        &mut self,
        _pos: FuncCursor,
        _index: MemoryIndex,
        _heap: ir::Heap,
        _dst: ir::Value,
        _val: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Ok(())
    }

    fn translate_memory_init(
        &mut self,
        _pos: FuncCursor,
        _index: MemoryIndex,
        _heap: ir::Heap,
        _seg_index: u32,
        _dst: ir::Value,
        _src: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Ok(())
    }

    fn translate_data_drop(&mut self, _pos: FuncCursor, _seg_index: u32) -> WasmResult<()> {
        Ok(())
    }

    fn translate_table_size(
        &mut self,
        mut pos: FuncCursor,
        _index: TableIndex,
        _table: ir::Table,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, -1))
    }

    fn translate_table_grow(
        &mut self,
        mut pos: FuncCursor,
        _table_index: TableIndex,
        _table: ir::Table,
        _delta: ir::Value,
        _init_value: ir::Value,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, -1))
    }

    fn translate_table_get(
        &mut self,
        builder: &mut FunctionBuilder,
        _table_index: TableIndex,
        _table: ir::Table,
        _index: ir::Value,
    ) -> WasmResult<ir::Value> {
        Ok(builder.ins().null(self.reference_type()))
    }

    fn translate_table_set(
        &mut self,
        _builder: &mut FunctionBuilder,
        _table_index: TableIndex,
        _table: ir::Table,
        _value: ir::Value,
        _index: ir::Value,
    ) -> WasmResult<()> {
        Ok(())
    }

    fn translate_table_copy(
        &mut self,
        _pos: FuncCursor,
        _dst_index: TableIndex,
        _dst_table: ir::Table,
        _src_index: TableIndex,
        _src_table: ir::Table,
        _dst: ir::Value,
        _src: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Ok(())
    }

    fn translate_table_fill(
        &mut self,
        _pos: FuncCursor,
        _table_index: TableIndex,
        _dst: ir::Value,
        _val: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Ok(())
    }

    fn translate_table_init(
        &mut self,
        _pos: FuncCursor,
        _seg_index: u32,
        _table_index: TableIndex,
        _table: ir::Table,
        _dst: ir::Value,
        _src: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Ok(())
    }

    fn translate_elem_drop(&mut self, _pos: FuncCursor, _seg_index: u32) -> WasmResult<()> {
        Ok(())
    }

    fn translate_ref_func(
        &mut self,
        mut pos: FuncCursor,
        _func_index: FuncIndex,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().null(self.reference_type()))
    }

    fn translate_custom_global_get(
        &mut self,
        mut pos: FuncCursor,
        _global_index: GlobalIndex,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, -1))
    }

    fn translate_custom_global_set(
        &mut self,
        _pos: FuncCursor,
        _global_index: GlobalIndex,
        _val: ir::Value,
    ) -> WasmResult<()> {
        Ok(())
    }

    fn translate_atomic_wait(
        &mut self,
        mut pos: FuncCursor,
        _index: MemoryIndex,
        _heap: ir::Heap,
        _addr: ir::Value,
        _expected: ir::Value,
        _timeout: ir::Value,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, -1))
    }

    fn translate_atomic_notify(
        &mut self,
        mut pos: FuncCursor,
        _index: MemoryIndex,
        _heap: ir::Heap,
        _addr: ir::Value,
        _count: ir::Value,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, 0))
    }

    fn unsigned_add_overflow_condition(&self) -> ir::condcodes::IntCC {
        unimplemented!()
    }
}

impl TargetEnvironment for DummyEnvironment {
    fn target_config(&self) -> TargetFrontendConfig {
        self.info.config
    }
}

impl<'data> ModuleEnvironment<'data> for DummyEnvironment {
    fn declare_type_func(&mut self, wasm: WasmFuncType) -> WasmResult<()> {
        let mut sig = ir::Signature::new(CallConv::Fast);
        let mut cvt = |ty: &WasmType| {
            let reference_type = match self.pointer_type() {
                ir::types::I32 => ir::types::R32,
                ir::types::I64 => ir::types::R64,
                _ => panic!("unsupported pointer type"),
            };
            ir::AbiParam::new(match ty {
                WasmType::I32 => ir::types::I32,
                WasmType::I64 => ir::types::I64,
                WasmType::F32 => ir::types::F32,
                WasmType::F64 => ir::types::F64,
                WasmType::V128 => ir::types::I8X16,
                WasmType::FuncRef | WasmType::ExternRef | WasmType::ExnRef => reference_type,
            })
        };
        sig.params.extend(wasm.params.iter().map(&mut cvt));
        sig.returns.extend(wasm.returns.iter().map(&mut cvt));
        self.info.signatures.push(sig);
        Ok(())
    }

    fn declare_func_import(
        &mut self,
        index: TypeIndex,
        module: &'data str,
        field: Option<&'data str>,
    ) -> WasmResult<()> {
        assert_eq!(
            self.info.functions.len(),
            self.info.imported_funcs.len(),
            "Imported functions must be declared first"
        );
        self.info.functions.push(Exportable::new(index));
        self.info
            .imported_funcs
            .push((String::from(module), String::from(field.unwrap())));
        Ok(())
    }

    fn declare_func_type(&mut self, index: TypeIndex) -> WasmResult<()> {
        self.info.functions.push(Exportable::new(index));
        Ok(())
    }

    fn declare_global(&mut self, global: Global) -> WasmResult<()> {
        self.info.globals.push(Exportable::new(global));
        Ok(())
    }

    fn declare_global_import(
        &mut self,
        global: Global,
        module: &'data str,
        field: Option<&'data str>,
    ) -> WasmResult<()> {
        self.info.globals.push(Exportable::new(global));
        self.info
            .imported_globals
            .push((String::from(module), String::from(field.unwrap())));
        Ok(())
    }

    fn declare_table(&mut self, table: Table) -> WasmResult<()> {
        self.info.tables.push(Exportable::new(table));
        Ok(())
    }

    fn declare_table_import(
        &mut self,
        table: Table,
        module: &'data str,
        field: Option<&'data str>,
    ) -> WasmResult<()> {
        self.info.tables.push(Exportable::new(table));
        self.info
            .imported_tables
            .push((String::from(module), String::from(field.unwrap())));
        Ok(())
    }

    fn declare_table_elements(
        &mut self,
        _table_index: TableIndex,
        _base: Option<GlobalIndex>,
        _offset: u32,
        _elements: Box<[FuncIndex]>,
    ) -> WasmResult<()> {
        // We do nothing
        Ok(())
    }

    fn declare_passive_element(
        &mut self,
        _elem_index: ElemIndex,
        _segments: Box<[FuncIndex]>,
    ) -> WasmResult<()> {
        Ok(())
    }

    fn declare_passive_data(
        &mut self,
        _elem_index: DataIndex,
        _segments: &'data [u8],
    ) -> WasmResult<()> {
        Ok(())
    }

    fn declare_memory(&mut self, memory: Memory) -> WasmResult<()> {
        self.info.memories.push(Exportable::new(memory));
        Ok(())
    }

    fn declare_memory_import(
        &mut self,
        memory: Memory,
        module: &'data str,
        field: Option<&'data str>,
    ) -> WasmResult<()> {
        self.info.memories.push(Exportable::new(memory));
        self.info
            .imported_memories
            .push((String::from(module), String::from(field.unwrap())));
        Ok(())
    }

    fn declare_data_initialization(
        &mut self,
        _memory_index: MemoryIndex,
        _base: Option<GlobalIndex>,
        _offset: u64,
        _data: &'data [u8],
    ) -> WasmResult<()> {
        // We do nothing
        Ok(())
    }

    fn declare_func_export(&mut self, func_index: FuncIndex, name: &'data str) -> WasmResult<()> {
        self.info.functions[func_index]
            .export_names
            .push(String::from(name));
        Ok(())
    }

    fn declare_table_export(
        &mut self,
        table_index: TableIndex,
        name: &'data str,
    ) -> WasmResult<()> {
        self.info.tables[table_index]
            .export_names
            .push(String::from(name));
        Ok(())
    }

    fn declare_memory_export(
        &mut self,
        memory_index: MemoryIndex,
        name: &'data str,
    ) -> WasmResult<()> {
        self.info.memories[memory_index]
            .export_names
            .push(String::from(name));
        Ok(())
    }

    fn declare_global_export(
        &mut self,
        global_index: GlobalIndex,
        name: &'data str,
    ) -> WasmResult<()> {
        self.info.globals[global_index]
            .export_names
            .push(String::from(name));
        Ok(())
    }

    fn declare_start_func(&mut self, func_index: FuncIndex) -> WasmResult<()> {
        debug_assert!(self.info.start_func.is_none());
        self.info.start_func = Some(func_index);
        Ok(())
    }

    fn define_function_body(
        &mut self,
        mut validator: FuncValidator<ValidatorResources>,
        body: FunctionBody<'data>,
    ) -> WasmResult<()> {
        self.func_bytecode_sizes
            .push(body.get_binary_reader().bytes_remaining());
        let func = {
            let mut func_environ = DummyFuncEnvironment::new(
                &self.info,
                self.return_mode,
                self.expected_reachability.clone(),
            );
            let func_index =
                FuncIndex::new(self.get_num_func_imports() + self.info.function_bodies.len());
            let name = get_func_name(func_index);
            let sig = func_environ.vmctx_sig(self.get_func_type(func_index));
            let mut func = ir::Function::with_name_signature(name, sig);
            if self.debug_info {
                func.collect_debug_info();
            }
            self.trans
                .translate_body(&mut validator, body, &mut func, &mut func_environ)?;
            func
        };
        self.info.function_bodies.push(func);
        Ok(())
    }

    fn declare_module_name(&mut self, name: &'data str) {
        self.module_name = Some(String::from(name));
    }

    fn declare_func_name(&mut self, func_index: FuncIndex, name: &'data str) {
        self.function_names[func_index] = String::from(name);
    }

    fn wasm_features(&self) -> WasmFeatures {
        WasmFeatures {
            multi_value: true,
            simd: true,
            reference_types: true,
            bulk_memory: true,
            ..WasmFeatures::default()
        }
    }
}
