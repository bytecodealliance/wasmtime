//! "Dummy" implementations of `ModuleEnvironment` and `FuncEnvironment` for testing
//! wasm translation. For complete implementations of `ModuleEnvironment` and
//! `FuncEnvironment`, see [wasmtime-environ] in [Wasmtime].
//!
//! [wasmtime-environ]: https://crates.io/crates/wasmtime-environ
//! [Wasmtime]: https://github.com/bytecodealliance/wasmtime

use crate::environ::{FuncEnvironment, GlobalVariable, ModuleEnvironment, TargetEnvironment};
use crate::func_translator::FuncTranslator;
use crate::{
    DataIndex, DefinedFuncIndex, ElemIndex, FuncIndex, Global, GlobalIndex, GlobalInit, Heap,
    HeapData, HeapStyle, Memory, MemoryIndex, Table, TableIndex, TableSize, TypeConvert, TypeIndex,
    WasmFuncType, WasmHeapType, WasmResult,
};
use crate::{TableData, WasmValType};
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_codegen::ir::{types::*, UserFuncName};
use cranelift_codegen::isa::{CallConv, TargetFrontendConfig};
use cranelift_entity::{EntityRef, PrimaryMap, SecondaryMap};
use cranelift_frontend::FunctionBuilder;
use std::boxed::Box;
use std::string::String;
use std::vec::Vec;
use wasmparser::{FuncValidator, FunctionBody, ValidatorResources, WasmFeatures};
use wasmtime_types::wasm_unsupported;

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

/// This `ModuleEnvironment` implementation is a "naïve" one, doing essentially nothing and
/// emitting placeholders when forced to. Don't try to execute code translated for this
/// environment, essentially here for translation debug purposes.
pub struct DummyEnvironment {
    /// Module information.
    pub info: DummyModuleInfo,

    /// Function translation.
    pub trans: FuncTranslator,

    /// Vector of wasm bytecode size for each function.
    pub func_bytecode_sizes: Vec<usize>,

    /// Name of the module from the wasm file.
    pub module_name: Option<String>,

    /// Function names.
    function_names: SecondaryMap<FuncIndex, String>,
}

impl DummyEnvironment {
    /// Creates a new `DummyEnvironment` instance.
    pub fn new(config: TargetFrontendConfig) -> Self {
        Self {
            info: DummyModuleInfo::new(config),
            trans: FuncTranslator::new(),
            func_bytecode_sizes: Vec::new(),
            module_name: None,
            function_names: SecondaryMap::new(),
        }
    }

    /// Return a `DummyFuncEnvironment` for translating functions within this
    /// `DummyEnvironment`.
    pub fn func_env(&self) -> DummyFuncEnvironment {
        DummyFuncEnvironment::new(&self.info)
    }

    /// Get the type for the function at the given index.
    pub fn get_func_type(&self, func_index: FuncIndex) -> TypeIndex {
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
}

/// The `FuncEnvironment` implementation for use by the `DummyEnvironment`.
pub struct DummyFuncEnvironment<'dummy_environment> {
    /// This function environment's module info.
    pub mod_info: &'dummy_environment DummyModuleInfo,

    /// Heaps we have created to implement Wasm linear memories.
    pub heaps: PrimaryMap<Heap, HeapData>,

    /// Cranelift tables we have created to implement Wasm tables.
    tables: SecondaryMap<TableIndex, Option<TableData>>,
}

impl<'dummy_environment> DummyFuncEnvironment<'dummy_environment> {
    /// Construct a new `DummyFuncEnvironment`.
    pub fn new(mod_info: &'dummy_environment DummyModuleInfo) -> Self {
        Self {
            mod_info,
            heaps: Default::default(),
            tables: Default::default(),
        }
    }

    /// Create a signature for `sigidx` amended with a `vmctx` argument after
    /// the standard wasm arguments.
    pub fn vmctx_sig(&self, sigidx: TypeIndex) -> ir::Signature {
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

    fn ensure_table_exists(&mut self, func: &mut ir::Function, index: TableIndex) {
        if self.tables[index].is_some() {
            return;
        }

        // Create a table whose base address is stored at `vmctx+0`.
        let vmctx = func.create_global_value(ir::GlobalValueData::VMContext);
        let base_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: vmctx,
            offset: Offset32::new(0),
            global_type: self.pointer_type(),
            // When tables in wasm become "growable", revisit whether this can be readonly or not.
            flags: ir::MemFlags::trusted().with_readonly(),
        });

        let table = &self.mod_info.tables[index].entity;

        let bound = if Some(table.minimum) == table.maximum {
            TableSize::Static {
                bound: table.minimum,
            }
        } else {
            TableSize::Dynamic {
                bound_gv: func.create_global_value(ir::GlobalValueData::Load {
                    base: vmctx,
                    offset: Offset32::new(0),
                    global_type: I32,
                    flags: ir::MemFlags::trusted().with_readonly(),
                }),
            }
        };

        self.tables[index] = Some(TableData {
            base_gv,
            bound,
            element_size: u32::from(self.pointer_bytes()) * 2,
        });
    }
}

impl<'dummy_environment> TypeConvert for DummyFuncEnvironment<'dummy_environment> {
    fn lookup_heap_type(&self, _index: wasmparser::UnpackedIndex) -> WasmHeapType {
        unimplemented!()
    }
}

impl<'dummy_environment> TargetEnvironment for DummyFuncEnvironment<'dummy_environment> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.mod_info.config
    }

    fn heap_access_spectre_mitigation(&self) -> bool {
        false
    }

    fn proof_carrying_code(&self) -> bool {
        false
    }
}

impl<'dummy_environment> FuncEnvironment for DummyFuncEnvironment<'dummy_environment> {
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
            ty: match self.mod_info.globals[index].entity.wasm_ty {
                WasmValType::I32 => ir::types::I32,
                WasmValType::I64 => ir::types::I64,
                WasmValType::F32 => ir::types::F32,
                WasmValType::F64 => ir::types::F64,
                WasmValType::V128 => ir::types::I8X16,
                WasmValType::Ref(_) => ir::types::R64,
            },
        })
    }

    fn heaps(&self) -> &PrimaryMap<Heap, HeapData> {
        &self.heaps
    }

    fn make_heap(&mut self, func: &mut ir::Function, _index: MemoryIndex) -> WasmResult<Heap> {
        // Create a static heap whose base address is stored at `vmctx+0`.
        let addr = func.create_global_value(ir::GlobalValueData::VMContext);
        let gv = func.create_global_value(ir::GlobalValueData::Load {
            base: addr,
            offset: Offset32::new(0),
            global_type: self.pointer_type(),
            flags: ir::MemFlags::trusted().with_readonly(),
        });

        Ok(self.heaps.push(HeapData {
            base: gv,
            min_size: 0,
            max_size: None,
            offset_guard_size: 0x8000_0000,
            style: HeapStyle::Static {
                bound: 0x1_0000_0000,
            },
            index_type: I32,
            memory_type: None,
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
        let name =
            ir::ExternalName::User(func.declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: index.as_u32(),
            }));
        Ok(func.import_function(ir::ExtFuncData {
            name,
            signature,
            colocated: false,
        }))
    }

    fn translate_call_indirect(
        &mut self,
        builder: &mut FunctionBuilder,
        _table_index: TableIndex,
        _sig_index: TypeIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<Option<ir::Inst>> {
        // Pass the current function's vmctx parameter on to the callee.
        let vmctx = builder
            .func
            .special_param(ir::ArgumentPurpose::VMContext)
            .expect("Missing vmctx parameter");

        // The `callee` value is an index into a table of function pointers.
        // Apparently, that table is stored at absolute address 0 in this dummy environment.
        // TODO: Generate bounds checking code.
        let ptr = self.pointer_type();
        let callee_offset = if ptr == I32 {
            builder.ins().imul_imm(callee, 4)
        } else {
            let ext = builder.ins().uextend(I64, callee);
            builder.ins().imul_imm(ext, 4)
        };
        let mflags = ir::MemFlags::trusted();
        let func_ptr = builder.ins().load(ptr, mflags, callee_offset, 0);

        // Build a value list for the indirect call instruction containing the callee, call_args,
        // and the vmctx parameter.
        let mut args = ir::ValueList::default();
        args.push(func_ptr, &mut builder.func.dfg.value_lists);
        args.extend(call_args.iter().cloned(), &mut builder.func.dfg.value_lists);
        args.push(vmctx, &mut builder.func.dfg.value_lists);

        Ok(Some(
            builder
                .ins()
                .CallIndirect(ir::Opcode::CallIndirect, INVALID, sig_ref, args)
                .0,
        ))
    }

    fn translate_return_call_indirect(
        &mut self,
        _builder: &mut FunctionBuilder,
        _table_index: TableIndex,
        _sig_index: TypeIndex,
        _sig_ref: ir::SigRef,
        _callee: ir::Value,
        _call_args: &[ir::Value],
    ) -> WasmResult<()> {
        unimplemented!()
    }

    fn translate_return_call_ref(
        &mut self,
        _builder: &mut FunctionBuilder,
        _sig_ref: ir::SigRef,
        _callee: ir::Value,
        _call_args: &[ir::Value],
    ) -> WasmResult<()> {
        unimplemented!()
    }

    fn translate_call(
        &mut self,
        builder: &mut FunctionBuilder,
        _callee_index: FuncIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        // Pass the current function's vmctx parameter on to the callee.
        let vmctx = builder
            .func
            .special_param(ir::ArgumentPurpose::VMContext)
            .expect("Missing vmctx parameter");

        // Build a value list for the call instruction containing the call_args and the vmctx
        // parameter.
        let mut args = ir::ValueList::default();
        args.extend(call_args.iter().cloned(), &mut builder.func.dfg.value_lists);
        args.push(vmctx, &mut builder.func.dfg.value_lists);

        Ok(builder
            .ins()
            .Call(ir::Opcode::Call, INVALID, callee, args)
            .0)
    }

    fn translate_call_ref(
        &mut self,
        _builder: &mut FunctionBuilder,
        _sig_ref: ir::SigRef,
        _callee: ir::Value,
        _call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        todo!("Implement dummy translate_call_ref")
    }

    fn translate_memory_grow(
        &mut self,
        mut pos: FuncCursor,
        _index: MemoryIndex,
        _heap: Heap,
        _val: ir::Value,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, -1i32 as u32 as i64))
    }

    fn translate_memory_size(
        &mut self,
        mut pos: FuncCursor,
        _index: MemoryIndex,
        _heap: Heap,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, -1i32 as u32 as i64))
    }

    fn translate_memory_copy(
        &mut self,
        _pos: FuncCursor,
        _src_index: MemoryIndex,
        _src_heap: Heap,
        _dst_index: MemoryIndex,
        _dst_heap: Heap,
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
        _heap: Heap,
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
        _heap: Heap,
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
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, -1i32 as u32 as i64))
    }

    fn translate_table_grow(
        &mut self,
        mut pos: FuncCursor,
        _table_index: TableIndex,
        _delta: ir::Value,
        _init_value: ir::Value,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, -1i32 as u32 as i64))
    }

    fn translate_table_get(
        &mut self,
        builder: &mut FunctionBuilder,
        table_index: TableIndex,
        index: ir::Value,
    ) -> WasmResult<ir::Value> {
        let pointer_type = self.pointer_type();
        self.ensure_table_exists(builder.func, table_index);
        let table = self.tables[table_index].as_ref().unwrap();
        let (table_entry_addr, flags) =
            table.prepare_table_addr(builder, index, pointer_type, true);
        let value = builder
            .ins()
            .load(self.reference_type(), flags, table_entry_addr, 0);
        Ok(value)
    }

    fn translate_table_set(
        &mut self,
        builder: &mut FunctionBuilder,
        table_index: TableIndex,
        value: ir::Value,
        index: ir::Value,
    ) -> WasmResult<()> {
        let pointer_type = self.pointer_type();
        self.ensure_table_exists(builder.func, table_index);
        let table = self.tables[table_index].as_ref().unwrap();
        let (table_entry_addr, flags) =
            table.prepare_table_addr(builder, index, pointer_type, true);
        builder.ins().store(flags, value, table_entry_addr, 0);
        Ok(())
    }

    fn translate_table_copy(
        &mut self,
        _pos: FuncCursor,
        _dst_index: TableIndex,
        _src_index: TableIndex,
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
        Ok(pos.ins().iconst(I32, -1i32 as u32 as i64))
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
        _heap: Heap,
        _addr: ir::Value,
        _expected: ir::Value,
        _timeout: ir::Value,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, -1i32 as u32 as i64))
    }

    fn translate_atomic_notify(
        &mut self,
        mut pos: FuncCursor,
        _index: MemoryIndex,
        _heap: Heap,
        _addr: ir::Value,
        _count: ir::Value,
    ) -> WasmResult<ir::Value> {
        Ok(pos.ins().iconst(I32, 0))
    }

    fn translate_ref_i31(&mut self, _pos: FuncCursor, _val: ir::Value) -> WasmResult<ir::Value> {
        Err(wasm_unsupported!("ref.i31"))
    }

    fn translate_i31_get_s(
        &mut self,
        _pos: FuncCursor,
        _i31ref: ir::Value,
    ) -> WasmResult<ir::Value> {
        Err(wasm_unsupported!("i31.get_s"))
    }

    fn translate_i31_get_u(
        &mut self,
        _pos: FuncCursor,
        _i31ref: ir::Value,
    ) -> WasmResult<ir::Value> {
        Err(wasm_unsupported!("i31.get_u"))
    }
}

impl TypeConvert for DummyEnvironment {
    fn lookup_heap_type(&self, _index: wasmparser::UnpackedIndex) -> WasmHeapType {
        unimplemented!()
    }
}

impl TargetEnvironment for DummyEnvironment {
    fn target_config(&self) -> TargetFrontendConfig {
        self.info.config
    }

    fn heap_access_spectre_mitigation(&self) -> bool {
        false
    }

    fn proof_carrying_code(&self) -> bool {
        false
    }
}

impl<'data> ModuleEnvironment<'data> for DummyEnvironment {
    fn declare_type_func(&mut self, wasm: WasmFuncType) -> WasmResult<()> {
        let mut sig = ir::Signature::new(CallConv::Fast);
        let mut cvt = |ty: &WasmValType| {
            let reference_type = match self.pointer_type() {
                ir::types::I32 => ir::types::R32,
                ir::types::I64 => ir::types::R64,
                _ => panic!("unsupported pointer type"),
            };
            ir::AbiParam::new(match ty {
                WasmValType::I32 => ir::types::I32,
                WasmValType::I64 => ir::types::I64,
                WasmValType::F32 => ir::types::F32,
                WasmValType::F64 => ir::types::F64,
                WasmValType::V128 => ir::types::I8X16,
                WasmValType::Ref(_) => reference_type,
            })
        };
        sig.params.extend(wasm.params().iter().map(&mut cvt));
        sig.returns.extend(wasm.returns().iter().map(&mut cvt));
        self.info.signatures.push(sig);
        Ok(())
    }

    fn declare_func_import(
        &mut self,
        index: TypeIndex,
        module: &'data str,
        field: &'data str,
    ) -> WasmResult<()> {
        assert_eq!(
            self.info.functions.len(),
            self.info.imported_funcs.len(),
            "Imported functions must be declared first"
        );
        self.info.functions.push(Exportable::new(index));
        self.info
            .imported_funcs
            .push((String::from(module), String::from(field)));
        Ok(())
    }

    fn declare_func_type(&mut self, index: TypeIndex) -> WasmResult<()> {
        self.info.functions.push(Exportable::new(index));
        Ok(())
    }

    fn declare_global(&mut self, global: Global, _init: GlobalInit) -> WasmResult<()> {
        self.info.globals.push(Exportable::new(global));
        Ok(())
    }

    fn declare_global_import(
        &mut self,
        global: Global,
        module: &'data str,
        field: &'data str,
    ) -> WasmResult<()> {
        self.info.globals.push(Exportable::new(global));
        self.info
            .imported_globals
            .push((String::from(module), String::from(field)));
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
        field: &'data str,
    ) -> WasmResult<()> {
        self.info.tables.push(Exportable::new(table));
        self.info
            .imported_tables
            .push((String::from(module), String::from(field)));
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
        field: &'data str,
    ) -> WasmResult<()> {
        self.info.memories.push(Exportable::new(memory));
        self.info
            .imported_memories
            .push((String::from(module), String::from(field)));
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
            let mut func_environ = DummyFuncEnvironment::new(&self.info);
            let func_index =
                FuncIndex::new(self.get_num_func_imports() + self.info.function_bodies.len());

            let sig = func_environ.vmctx_sig(self.get_func_type(func_index));
            let mut func =
                ir::Function::with_name_signature(UserFuncName::user(0, func_index.as_u32()), sig);

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
