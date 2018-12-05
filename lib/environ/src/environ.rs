use cast;
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir;
use cranelift_codegen::ir::condcodes::*;
use cranelift_codegen::ir::immediates::{Imm64, Offset32, Uimm64};
use cranelift_codegen::ir::types::*;
use cranelift_codegen::ir::{
    AbiParam, ArgumentPurpose, ExtFuncData, FuncRef, Function, InstBuilder, Signature,
};
use cranelift_codegen::isa;
use cranelift_entity::EntityRef;
use cranelift_wasm::{
    self, translate_module, FuncIndex, Global, GlobalIndex, GlobalVariable, Memory, MemoryIndex,
    SignatureIndex, Table, TableIndex, WasmResult,
};
use module::{
    DataInitializer, Export, LazyContents, MemoryPlan, MemoryStyle, Module, TableElements,
    TablePlan, TableStyle,
};
use std::clone::Clone;
use std::string::String;
use std::vec::Vec;
use tunables::Tunables;
use vmoffsets::VMOffsets;
use WASM_PAGE_SIZE;

/// Compute a `ir::ExternalName` for a given wasm function index.
pub fn get_func_name(func_index: FuncIndex) -> ir::ExternalName {
    ir::ExternalName::user(0, func_index.as_u32())
}

/// Compute a `ir::ExternalName` for the `memory.grow` libcall.
pub fn get_memory_grow_name() -> ir::ExternalName {
    ir::ExternalName::user(1, 0)
}

/// Compute a `ir::ExternalName` for the `memory.size` libcall.
pub fn get_memory_size_name() -> ir::ExternalName {
    ir::ExternalName::user(1, 1)
}

/// Object containing the standalone environment information. To be passed after creation as
/// argument to `compile_module`.
pub struct ModuleEnvironment<'data, 'module> {
    /// Compilation setting flags.
    isa: &'module isa::TargetIsa,

    /// Module information.
    module: &'module mut Module,

    /// References to information to be decoded later.
    lazy: LazyContents<'data>,

    /// Tunable parameters.
    tunables: Tunables,
}

impl<'data, 'module> ModuleEnvironment<'data, 'module> {
    /// Allocates the enironment data structures with the given isa.
    pub fn new(
        isa: &'module isa::TargetIsa,
        module: &'module mut Module,
        tunables: Tunables,
    ) -> Self {
        Self {
            isa,
            module,
            lazy: LazyContents::new(),
            tunables,
        }
    }

    fn pointer_type(&self) -> ir::Type {
        self.isa.frontend_config().pointer_type()
    }

    /// Translate the given wasm module data using this environment. This consumes the
    /// `ModuleEnvironment` with its mutable reference to the `Module` and produces a
    /// `ModuleTranslation` with an immutable reference to the `Module` (which has
    /// become fully populated).
    pub fn translate(mut self, data: &'data [u8]) -> WasmResult<ModuleTranslation<'data, 'module>> {
        translate_module(data, &mut self)?;

        Ok(ModuleTranslation {
            isa: self.isa,
            module: self.module,
            lazy: self.lazy,
            tunables: self.tunables,
        })
    }
}

/// The FuncEnvironment implementation for use by the `ModuleEnvironment`.
pub struct FuncEnvironment<'module_environment> {
    /// Compilation setting flags.
    isa: &'module_environment isa::TargetIsa,

    /// The module-level environment which this function-level environment belongs to.
    module: &'module_environment Module,

    /// The Cranelift global holding the vmctx address.
    vmctx: Option<ir::GlobalValue>,

    /// The Cranelift global holding the base address of the memories vector.
    memories_base: Option<ir::GlobalValue>,

    /// The Cranelift global holding the base address of the tables vector.
    tables_base: Option<ir::GlobalValue>,

    /// The Cranelift global holding the base address of the globals vector.
    globals_base: Option<ir::GlobalValue>,

    /// The Cranelift global holding the base address of the signature IDs vector.
    signature_ids_base: Option<ir::GlobalValue>,

    /// The external function declaration for implementing wasm's `memory.size`.
    memory_size_extfunc: Option<FuncRef>,

    /// The external function declaration for implementing wasm's `memory.grow`.
    memory_grow_extfunc: Option<FuncRef>,

    /// Offsets to struct fields accessed by JIT code.
    offsets: VMOffsets,
}

impl<'module_environment> FuncEnvironment<'module_environment> {
    pub fn new(
        isa: &'module_environment isa::TargetIsa,
        module: &'module_environment Module,
    ) -> Self {
        Self {
            isa,
            module,
            vmctx: None,
            memories_base: None,
            tables_base: None,
            globals_base: None,
            signature_ids_base: None,
            memory_size_extfunc: None,
            memory_grow_extfunc: None,
            offsets: VMOffsets::new(isa.pointer_bytes()),
        }
    }

    /// Transform the call argument list in preparation for making a call.
    fn get_real_call_args(func: &Function, call_args: &[ir::Value]) -> Vec<ir::Value> {
        let mut real_call_args = Vec::with_capacity(call_args.len() + 1);
        real_call_args.extend_from_slice(call_args);
        real_call_args.push(func.special_param(ArgumentPurpose::VMContext).unwrap());
        real_call_args
    }

    fn vmctx(&mut self, func: &mut Function) -> ir::GlobalValue {
        self.vmctx.unwrap_or_else(|| {
            let vmctx = func.create_global_value(ir::GlobalValueData::VMContext);
            self.vmctx = Some(vmctx);
            vmctx
        })
    }
}

/// This trait is useful for `translate_module` because it tells how to translate
/// enironment-dependent wasm instructions. These functions should not be called by the user.
impl<'data, 'module> cranelift_wasm::ModuleEnvironment<'data>
    for ModuleEnvironment<'data, 'module>
{
    fn target_config(&self) -> isa::TargetFrontendConfig {
        self.isa.frontend_config()
    }

    fn declare_signature(&mut self, sig: &ir::Signature) {
        let mut sig = sig.clone();
        sig.params.push(AbiParam::special(
            self.pointer_type(),
            ArgumentPurpose::VMContext,
        ));
        // TODO: Deduplicate signatures.
        self.module.signatures.push(sig);
    }

    fn get_signature(&self, sig_index: SignatureIndex) -> &ir::Signature {
        &self.module.signatures[sig_index]
    }

    fn declare_func_import(&mut self, sig_index: SignatureIndex, module: &str, field: &str) {
        debug_assert_eq!(
            self.module.functions.len(),
            self.module.imported_funcs.len(),
            "Imported functions must be declared first"
        );
        self.module.functions.push(sig_index);

        self.module
            .imported_funcs
            .push((String::from(module), String::from(field)));
    }

    fn get_num_func_imports(&self) -> usize {
        self.module.imported_funcs.len()
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) {
        self.module.functions.push(sig_index);
    }

    fn get_func_type(&self, func_index: FuncIndex) -> SignatureIndex {
        self.module.functions[func_index]
    }

    fn declare_global_import(&mut self, _global: Global, _module: &str, _field: &str) {
        unimplemented!("imported globals");
    }

    fn declare_global(&mut self, global: Global) {
        self.module.globals.push(global);
    }

    fn get_global(&self, global_index: GlobalIndex) -> &Global {
        &self.module.globals[global_index]
    }

    fn declare_table_import(&mut self, _table: Table, _module: &str, _field: &str) {
        unimplemented!("imported tables");
    }

    fn declare_table(&mut self, table: Table) {
        let plan = TablePlan::for_table(table, &self.tunables);
        self.module.table_plans.push(plan);
    }

    fn declare_table_elements(
        &mut self,
        table_index: TableIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        elements: Vec<FuncIndex>,
    ) {
        debug_assert!(base.is_none(), "global-value offsets not supported yet");
        self.module.table_elements.push(TableElements {
            table_index,
            base,
            offset,
            elements,
        });
    }

    fn declare_memory_import(&mut self, _memory: Memory, _module: &str, _field: &str) {
        unimplemented!("imported memories");
    }

    fn declare_memory(&mut self, memory: Memory) {
        let plan = MemoryPlan::for_memory(memory, &self.tunables);
        self.module.memory_plans.push(plan);
    }

    fn declare_data_initialization(
        &mut self,
        memory_index: MemoryIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        data: &'data [u8],
    ) {
        debug_assert!(base.is_none(), "global-value offsets not supported yet");
        self.lazy.data_initializers.push(DataInitializer {
            memory_index,
            base,
            offset,
            data,
        });
    }

    fn declare_func_export(&mut self, func_index: FuncIndex, name: &str) {
        self.module
            .exports
            .insert(String::from(name), Export::Function(func_index));
    }

    fn declare_table_export(&mut self, table_index: TableIndex, name: &str) {
        self.module
            .exports
            .insert(String::from(name), Export::Table(table_index));
    }

    fn declare_memory_export(&mut self, memory_index: MemoryIndex, name: &str) {
        self.module
            .exports
            .insert(String::from(name), Export::Memory(memory_index));
    }

    fn declare_global_export(&mut self, global_index: GlobalIndex, name: &str) {
        self.module
            .exports
            .insert(String::from(name), Export::Global(global_index));
    }

    fn declare_start_func(&mut self, func_index: FuncIndex) {
        debug_assert!(self.module.start_func.is_none());
        self.module.start_func = Some(func_index);
    }

    fn define_function_body(&mut self, body_bytes: &'data [u8]) -> WasmResult<()> {
        self.lazy.function_body_inputs.push(body_bytes);
        Ok(())
    }
}

impl<'module_environment> cranelift_wasm::FuncEnvironment for FuncEnvironment<'module_environment> {
    fn target_config(&self) -> isa::TargetFrontendConfig {
        self.isa.frontend_config()
    }

    fn make_global(&mut self, func: &mut ir::Function, index: GlobalIndex) -> GlobalVariable {
        let pointer_type = self.pointer_type();

        let vmctx = self.vmctx(func);
        let globals_base = self.globals_base.unwrap_or_else(|| {
            let new_base = func.create_global_value(ir::GlobalValueData::Load {
                base: vmctx,
                offset: Offset32::new(i32::from(self.offsets.vmctx_globals())),
                global_type: pointer_type,
                readonly: true,
            });
            self.globals_base = Some(new_base);
            new_base
        });
        let gv = func.create_global_value(ir::GlobalValueData::IAddImm {
            base: globals_base,
            offset: Imm64::new(i64::from(self.offsets.index_vmglobal(index.as_u32()))),
            global_type: pointer_type,
        });
        GlobalVariable::Memory {
            gv,
            ty: self.module.globals[index].ty,
        }
    }

    fn make_heap(&mut self, func: &mut ir::Function, index: MemoryIndex) -> ir::Heap {
        let pointer_type = self.pointer_type();

        let vmctx = self.vmctx(func);
        let memories_base = self.memories_base.unwrap_or_else(|| {
            let new_base = func.create_global_value(ir::GlobalValueData::Load {
                base: vmctx,
                offset: Offset32::new(i32::from(self.offsets.vmctx_memories())),
                global_type: pointer_type,
                readonly: true,
            });
            self.memories_base = Some(new_base);
            new_base
        });
        // If we have a declared maximum, we can make this a "static" heap, which is
        // allocated up front and never moved.
        let (offset_guard_size, heap_style, readonly_base) = match self.module.memory_plans[index] {
            MemoryPlan {
                memory: _,
                style: MemoryStyle::Dynamic,
                offset_guard_size,
            } => {
                let heap_bound = func.create_global_value(ir::GlobalValueData::Load {
                    base: memories_base,
                    offset: Offset32::new(
                        self.offsets.index_vmmemory_current_length(index.as_u32()),
                    ),
                    global_type: I32,
                    readonly: false,
                });
                (
                    Uimm64::new(offset_guard_size),
                    ir::HeapStyle::Dynamic {
                        bound_gv: heap_bound,
                    },
                    false,
                )
            }
            MemoryPlan {
                memory: _,
                style: MemoryStyle::Static { bound },
                offset_guard_size,
            } => (
                Uimm64::new(offset_guard_size),
                ir::HeapStyle::Static {
                    bound: Uimm64::new(u64::from(bound) * u64::from(WASM_PAGE_SIZE)),
                },
                true,
            ),
        };

        let heap_base = func.create_global_value(ir::GlobalValueData::Load {
            base: memories_base,
            offset: Offset32::new(self.offsets.index_vmmemory_base(index.as_u32())),
            global_type: pointer_type,
            readonly: readonly_base,
        });
        func.create_heap(ir::HeapData {
            base: heap_base,
            min_size: 0.into(),
            offset_guard_size,
            style: heap_style,
            index_type: I32,
        })
    }

    fn make_table(&mut self, func: &mut ir::Function, index: TableIndex) -> ir::Table {
        let pointer_type = self.pointer_type();

        let vmctx = self.vmctx(func);
        let tables_base = self.tables_base.unwrap_or_else(|| {
            let new_base = func.create_global_value(ir::GlobalValueData::Load {
                base: vmctx,
                offset: Offset32::new(i32::from(self.offsets.vmctx_tables())),
                global_type: pointer_type,
                readonly: true,
            });
            self.tables_base = Some(new_base);
            new_base
        });
        let base_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: tables_base,
            offset: Offset32::new(self.offsets.index_vmtable_base(index.as_u32())),
            global_type: pointer_type,
            readonly: false,
        });
        let bound_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: tables_base,
            offset: Offset32::new(self.offsets.index_vmtable_current_elements(index.as_u32())),
            global_type: I32,
            readonly: false,
        });

        let element_size = match self.module.table_plans[index].style {
            TableStyle::CallerChecksSignature => {
                u64::from(self.offsets.size_of_vmcaller_checked_anyfunc())
            }
        };

        func.create_table(ir::TableData {
            base_gv,
            min_size: Uimm64::new(0),
            bound_gv,
            element_size: Uimm64::new(element_size),
            index_type: I32,
        })
    }

    fn make_indirect_sig(&mut self, func: &mut ir::Function, index: SignatureIndex) -> ir::SigRef {
        func.import_signature(self.module.signatures[index].clone())
    }

    fn make_direct_func(&mut self, func: &mut ir::Function, index: FuncIndex) -> ir::FuncRef {
        let sigidx = self.module.functions[index];
        let signature = func.import_signature(self.module.signatures[sigidx].clone());
        let name = get_func_name(index);
        // We currently allocate all code segments independently, so nothing
        // is colocated.
        let colocated = false;
        func.import_function(ir::ExtFuncData {
            name,
            signature,
            colocated,
        })
    }

    fn translate_call_indirect(
        &mut self,
        mut pos: FuncCursor,
        table_index: TableIndex,
        table: ir::Table,
        sig_index: SignatureIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        let pointer_type = self.pointer_type();

        let table_entry_addr = pos.ins().table_addr(pointer_type, table, callee, 0);

        // Dereference table_entry_addr to get the function address.
        let mut mem_flags = ir::MemFlags::new();
        mem_flags.set_notrap();
        mem_flags.set_aligned();
        let func_addr = pos.ins().load(
            pointer_type,
            mem_flags,
            table_entry_addr,
            i32::from(self.offsets.vmcaller_checked_anyfunc_func_ptr()),
        );

        // If necessary, check the signature.
        match self.module.table_plans[table_index].style {
            TableStyle::CallerChecksSignature => {
                let sig_id_size = self.offsets.size_of_vmsignature_id();
                let sig_id_type = Type::int(u16::from(sig_id_size) * 8).unwrap();

                let vmctx = self.vmctx(pos.func);
                let signature_ids_base = self.globals_base.unwrap_or_else(|| {
                    let new_base = pos.func.create_global_value(ir::GlobalValueData::Load {
                        base: vmctx,
                        offset: Offset32::new(i32::from(self.offsets.vmctx_signature_ids())),
                        global_type: pointer_type,
                        readonly: true,
                    });
                    self.signature_ids_base = Some(new_base);
                    new_base
                });
                let sig_ids = pos.ins().global_value(pointer_type, signature_ids_base);

                // Load the caller ID.
                // TODO: Factor this out into a MemFlags constructor, as it's used a lot.
                let mut mem_flags = ir::MemFlags::new();
                mem_flags.set_notrap();
                mem_flags.set_aligned();
                let caller_sig_id = pos.ins().load(
                    sig_id_type,
                    mem_flags,
                    sig_ids,
                    cast::i32(
                        sig_index
                            .as_u32()
                            .checked_mul(u32::from(sig_id_size))
                            .unwrap(),
                    ).unwrap(),
                );

                // Load the callee ID.
                let mut mem_flags = ir::MemFlags::new();
                mem_flags.set_notrap();
                mem_flags.set_aligned();
                let callee_sig_id = pos.ins().load(
                    sig_id_type,
                    mem_flags,
                    table_entry_addr,
                    i32::from(self.offsets.vmcaller_checked_anyfunc_type_id()),
                );

                // Check that they match.
                let cmp = pos.ins().icmp(IntCC::Equal, callee_sig_id, caller_sig_id);
                pos.ins().trapz(cmp, ir::TrapCode::BadSignature);
            }
        }

        let real_call_args = FuncEnvironment::get_real_call_args(pos.func, call_args);
        Ok(pos.ins().call_indirect(sig_ref, func_addr, &real_call_args))
    }

    fn translate_call(
        &mut self,
        mut pos: FuncCursor,
        _callee_index: FuncIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        let real_call_args = FuncEnvironment::get_real_call_args(pos.func, call_args);
        Ok(pos.ins().call(callee, &real_call_args))
    }

    fn translate_memory_grow(
        &mut self,
        mut pos: FuncCursor,
        index: MemoryIndex,
        _heap: ir::Heap,
        val: ir::Value,
    ) -> WasmResult<ir::Value> {
        let memory_grow_func = self.memory_grow_extfunc.unwrap_or_else(|| {
            let sig_ref = pos.func.import_signature(Signature {
                call_conv: self.isa.frontend_config().default_call_conv,
                params: vec![
                    AbiParam::new(I32),
                    AbiParam::new(I32),
                    AbiParam::special(self.pointer_type(), ArgumentPurpose::VMContext),
                ],
                returns: vec![AbiParam::new(I32)],
            });
            // We currently allocate all code segments independently, so nothing
            // is colocated.
            let colocated = false;
            pos.func.import_function(ExtFuncData {
                name: get_memory_grow_name(),
                signature: sig_ref,
                colocated,
            })
        });
        self.memory_grow_extfunc = Some(memory_grow_func);
        let memory_index = pos.ins().iconst(I32, index.index() as i64);
        let vmctx = pos.func.special_param(ArgumentPurpose::VMContext).unwrap();
        let call_inst = pos
            .ins()
            .call(memory_grow_func, &[val, memory_index, vmctx]);
        Ok(*pos.func.dfg.inst_results(call_inst).first().unwrap())
    }

    fn translate_memory_size(
        &mut self,
        mut pos: FuncCursor,
        index: MemoryIndex,
        _heap: ir::Heap,
    ) -> WasmResult<ir::Value> {
        let memory_size_func = self.memory_size_extfunc.unwrap_or_else(|| {
            let sig_ref = pos.func.import_signature(Signature {
                call_conv: self.isa.frontend_config().default_call_conv,
                params: vec![
                    AbiParam::new(I32),
                    AbiParam::special(self.pointer_type(), ArgumentPurpose::VMContext),
                ],
                returns: vec![AbiParam::new(I32)],
            });
            // We currently allocate all code segments independently, so nothing
            // is colocated.
            let colocated = false;
            pos.func.import_function(ExtFuncData {
                name: get_memory_size_name(),
                signature: sig_ref,
                colocated,
            })
        });
        self.memory_size_extfunc = Some(memory_size_func);
        let memory_index = pos.ins().iconst(I32, index.index() as i64);
        let vmctx = pos.func.special_param(ArgumentPurpose::VMContext).unwrap();
        let call_inst = pos.ins().call(memory_size_func, &[memory_index, vmctx]);
        Ok(*pos.func.dfg.inst_results(call_inst).first().unwrap())
    }
}

/// The result of translating via `ModuleEnvironment`.
pub struct ModuleTranslation<'data, 'module> {
    /// Compilation setting flags.
    pub isa: &'module isa::TargetIsa,

    /// Module information.
    pub module: &'module Module,

    /// Pointers into the raw data buffer.
    pub lazy: LazyContents<'data>,

    /// Tunable parameters.
    pub tunables: Tunables,
}

impl<'data, 'module> ModuleTranslation<'data, 'module> {
    /// Return a new `FuncEnvironment` for translating a function.
    pub fn func_env(&self) -> FuncEnvironment {
        FuncEnvironment::new(self.isa, &self.module)
    }
}
