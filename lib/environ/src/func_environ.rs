use cast;
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir;
use cranelift_codegen::ir::condcodes::*;
use cranelift_codegen::ir::immediates::{Imm64, Offset32, Uimm64};
use cranelift_codegen::ir::types::*;
use cranelift_codegen::ir::{
    AbiParam, ArgumentPurpose, ExtFuncData, FuncRef, Function, InstBuilder, Signature,
};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_entity::EntityRef;
use cranelift_wasm::{
    self, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex, GlobalIndex,
    GlobalVariable, MemoryIndex, SignatureIndex, TableIndex, WasmResult,
};
use module::{MemoryPlan, MemoryStyle, Module, TableStyle};
use std::clone::Clone;
use std::vec::Vec;
use vmoffsets::VMOffsets;
use WASM_PAGE_SIZE;

/// Compute an `ir::ExternalName` for a given wasm function index.
pub fn get_func_name(func_index: FuncIndex) -> ir::ExternalName {
    ir::ExternalName::user(0, func_index.as_u32())
}

/// Compute an `ir::ExternalName` for the `memory.grow` libcall for
/// 32-bit locally-defined memories.
pub fn get_memory32_grow_name() -> ir::ExternalName {
    ir::ExternalName::user(1, 0)
}

/// Compute an `ir::ExternalName` for the `memory.grow` libcall for
/// 32-bit imported memories.
pub fn get_imported_memory32_grow_name() -> ir::ExternalName {
    ir::ExternalName::user(1, 1)
}

/// Compute an `ir::ExternalName` for the `memory.size` libcall for
/// 32-bit locally-defined memories.
pub fn get_memory32_size_name() -> ir::ExternalName {
    ir::ExternalName::user(1, 2)
}

/// Compute an `ir::ExternalName` for the `memory.size` libcall for
/// 32-bit imported memories.
pub fn get_imported_memory32_size_name() -> ir::ExternalName {
    ir::ExternalName::user(1, 3)
}

/// The FuncEnvironment implementation for use by the `ModuleEnvironment`.
pub struct FuncEnvironment<'module_environment> {
    /// Target-specified configuration.
    target_config: TargetFrontendConfig,

    /// The module-level environment which this function-level environment belongs to.
    module: &'module_environment Module,

    /// The Cranelift global holding the vmctx address.
    vmctx: Option<ir::GlobalValue>,

    /// The Cranelift global holding the base address of the signature IDs vector.
    /// TODO: Now that the bases are just offsets from vmctx rather than loads, we
    /// can eliminate these base variables.
    signature_ids_base: Option<ir::GlobalValue>,

    /// The Cranelift global holding the base address of the imported functions table.
    imported_functions_base: Option<ir::GlobalValue>,

    /// The Cranelift global holding the base address of the imported tables table.
    imported_tables_base: Option<ir::GlobalValue>,

    /// The Cranelift global holding the base address of the imported memories table.
    imported_memories_base: Option<ir::GlobalValue>,

    /// The Cranelift global holding the base address of the imported globals table.
    imported_globals_base: Option<ir::GlobalValue>,

    /// The Cranelift global holding the base address of the tables vector.
    tables_base: Option<ir::GlobalValue>,

    /// The Cranelift global holding the base address of the memories vector.
    memories_base: Option<ir::GlobalValue>,

    /// The Cranelift global holding the base address of the globals vector.
    globals_base: Option<ir::GlobalValue>,

    /// The external function declaration for implementing wasm's `memory.size`
    /// for locally-defined 32-bit memories.
    memory32_size_extfunc: Option<FuncRef>,

    /// The external function declaration for implementing wasm's `memory.size`
    /// for imported 32-bit memories.
    imported_memory32_size_extfunc: Option<FuncRef>,

    /// The external function declaration for implementing wasm's `memory.grow`
    /// for locally-defined memories.
    memory_grow_extfunc: Option<FuncRef>,

    /// The external function declaration for implementing wasm's `memory.grow`
    /// for imported memories.
    imported_memory_grow_extfunc: Option<FuncRef>,

    /// Offsets to struct fields accessed by JIT code.
    offsets: VMOffsets,
}

impl<'module_environment> FuncEnvironment<'module_environment> {
    pub fn new(target_config: TargetFrontendConfig, module: &'module_environment Module) -> Self {
        Self {
            target_config,
            module,
            vmctx: None,
            signature_ids_base: None,
            imported_functions_base: None,
            imported_tables_base: None,
            imported_memories_base: None,
            imported_globals_base: None,
            tables_base: None,
            memories_base: None,
            globals_base: None,
            memory32_size_extfunc: None,
            imported_memory32_size_extfunc: None,
            memory_grow_extfunc: None,
            imported_memory_grow_extfunc: None,
            offsets: VMOffsets::new(target_config.pointer_bytes(), module),
        }
    }

    fn pointer_type(&self) -> ir::Type {
        self.target_config.pointer_type()
    }

    fn vmctx(&mut self, func: &mut Function) -> ir::GlobalValue {
        self.vmctx.unwrap_or_else(|| {
            let vmctx = func.create_global_value(ir::GlobalValueData::VMContext);
            self.vmctx = Some(vmctx);
            vmctx
        })
    }

    fn get_imported_functions_base(&mut self, func: &mut Function) -> ir::GlobalValue {
        self.imported_functions_base.unwrap_or_else(|| {
            let pointer_type = self.pointer_type();
            let vmctx = self.vmctx(func);
            let new_base = func.create_global_value(ir::GlobalValueData::IAddImm {
                base: vmctx,
                offset: Imm64::new(self.offsets.vmctx_imported_functions()),
                global_type: pointer_type,
            });
            self.imported_functions_base = Some(new_base);
            new_base
        })
    }

    fn get_imported_tables_base(&mut self, func: &mut Function) -> ir::GlobalValue {
        self.imported_tables_base.unwrap_or_else(|| {
            let pointer_type = self.pointer_type();
            let vmctx = self.vmctx(func);
            let new_base = func.create_global_value(ir::GlobalValueData::IAddImm {
                base: vmctx,
                offset: Imm64::new(self.offsets.vmctx_imported_tables()),
                global_type: pointer_type,
            });
            self.imported_tables_base = Some(new_base);
            new_base
        })
    }

    fn get_imported_memories_base(&mut self, func: &mut Function) -> ir::GlobalValue {
        self.imported_memories_base.unwrap_or_else(|| {
            let pointer_type = self.pointer_type();
            let vmctx = self.vmctx(func);
            let new_base = func.create_global_value(ir::GlobalValueData::IAddImm {
                base: vmctx,
                offset: Imm64::new(self.offsets.vmctx_imported_memories()),
                global_type: pointer_type,
            });
            self.imported_memories_base = Some(new_base);
            new_base
        })
    }

    fn get_imported_globals_base(&mut self, func: &mut Function) -> ir::GlobalValue {
        self.imported_globals_base.unwrap_or_else(|| {
            let pointer_type = self.pointer_type();
            let vmctx = self.vmctx(func);
            let new_base = func.create_global_value(ir::GlobalValueData::IAddImm {
                base: vmctx,
                offset: Imm64::new(self.offsets.vmctx_imported_globals()),
                global_type: pointer_type,
            });
            self.imported_globals_base = Some(new_base);
            new_base
        })
    }

    fn get_tables_base(&mut self, func: &mut Function) -> ir::GlobalValue {
        self.tables_base.unwrap_or_else(|| {
            let pointer_type = self.pointer_type();
            let vmctx = self.vmctx(func);
            let new_base = func.create_global_value(ir::GlobalValueData::IAddImm {
                base: vmctx,
                offset: Imm64::new(self.offsets.vmctx_tables()),
                global_type: pointer_type,
            });
            self.tables_base = Some(new_base);
            new_base
        })
    }

    fn get_memories_base(&mut self, func: &mut Function) -> ir::GlobalValue {
        self.memories_base.unwrap_or_else(|| {
            let pointer_type = self.pointer_type();
            let vmctx = self.vmctx(func);
            let new_base = func.create_global_value(ir::GlobalValueData::IAddImm {
                base: vmctx,
                offset: Imm64::new(self.offsets.vmctx_memories()),
                global_type: pointer_type,
            });
            self.memories_base = Some(new_base);
            new_base
        })
    }

    fn get_globals_base(&mut self, func: &mut Function) -> ir::GlobalValue {
        self.globals_base.unwrap_or_else(|| {
            let pointer_type = self.pointer_type();
            let vmctx = self.vmctx(func);
            let new_base = func.create_global_value(ir::GlobalValueData::IAddImm {
                base: vmctx,
                offset: Imm64::new(self.offsets.vmctx_globals()),
                global_type: pointer_type,
            });
            self.globals_base = Some(new_base);
            new_base
        })
    }

    fn get_signature_ids_base(&mut self, func: &mut Function) -> ir::GlobalValue {
        self.signature_ids_base.unwrap_or_else(|| {
            let pointer_type = self.pointer_type();
            let vmctx = self.vmctx(func);
            let new_base = func.create_global_value(ir::GlobalValueData::IAddImm {
                base: vmctx,
                offset: Imm64::new(self.offsets.vmctx_signature_ids()),
                global_type: pointer_type,
            });
            self.signature_ids_base = Some(new_base);
            new_base
        })
    }

    fn get_memory_grow_sig(&self, func: &mut Function) -> ir::SigRef {
        func.import_signature(Signature {
            params: vec![
                AbiParam::new(I32),
                AbiParam::new(I32),
                AbiParam::special(self.pointer_type(), ArgumentPurpose::VMContext),
            ],
            returns: vec![AbiParam::new(I32)],
            call_conv: self.target_config.default_call_conv,
        })
    }

    /// Return the memory.grow function to call for the given index, along with the
    /// translated index value to pass to it.
    fn get_memory_grow_func(
        &mut self,
        func: &mut Function,
        index: MemoryIndex,
    ) -> (FuncRef, usize) {
        if self.module.is_imported_memory(index) {
            let extfunc = self.imported_memory_grow_extfunc.unwrap_or_else(|| {
                let sig_ref = self.get_memory_grow_sig(func);
                func.import_function(ExtFuncData {
                    name: get_imported_memory32_grow_name(),
                    signature: sig_ref,
                    // We currently allocate all code segments independently, so nothing
                    // is colocated.
                    colocated: false,
                })
            });
            self.imported_memory_grow_extfunc = Some(extfunc);
            (extfunc, index.index())
        } else {
            let extfunc = self.memory_grow_extfunc.unwrap_or_else(|| {
                let sig_ref = self.get_memory_grow_sig(func);
                func.import_function(ExtFuncData {
                    name: get_memory32_grow_name(),
                    signature: sig_ref,
                    // We currently allocate all code segments independently, so nothing
                    // is colocated.
                    colocated: false,
                })
            });
            self.memory_grow_extfunc = Some(extfunc);
            (
                extfunc,
                self.module.defined_memory_index(index).unwrap().index(),
            )
        }
    }

    fn get_memory32_size_sig(&self, func: &mut Function) -> ir::SigRef {
        func.import_signature(Signature {
            params: vec![
                AbiParam::new(I32),
                AbiParam::special(self.pointer_type(), ArgumentPurpose::VMContext),
            ],
            returns: vec![AbiParam::new(I32)],
            call_conv: self.target_config.default_call_conv,
        })
    }

    /// Return the memory.size function to call for the given index, along with the
    /// translated index value to pass to it.
    fn get_memory_size_func(
        &mut self,
        func: &mut Function,
        index: MemoryIndex,
    ) -> (FuncRef, usize) {
        if self.module.is_imported_memory(index) {
            let extfunc = self.imported_memory32_size_extfunc.unwrap_or_else(|| {
                let sig_ref = self.get_memory32_size_sig(func);
                func.import_function(ExtFuncData {
                    name: get_imported_memory32_size_name(),
                    signature: sig_ref,
                    // We currently allocate all code segments independently, so nothing
                    // is colocated.
                    colocated: false,
                })
            });
            self.imported_memory32_size_extfunc = Some(extfunc);
            (extfunc, index.index())
        } else {
            let extfunc = self.memory32_size_extfunc.unwrap_or_else(|| {
                let sig_ref = self.get_memory32_size_sig(func);
                func.import_function(ExtFuncData {
                    name: get_memory32_size_name(),
                    signature: sig_ref,
                    // We currently allocate all code segments independently, so nothing
                    // is colocated.
                    colocated: false,
                })
            });
            self.memory32_size_extfunc = Some(extfunc);
            (
                extfunc,
                self.module.defined_memory_index(index).unwrap().index(),
            )
        }
    }
}

impl<'module_environment> cranelift_wasm::FuncEnvironment for FuncEnvironment<'module_environment> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.target_config
    }

    fn make_table(&mut self, func: &mut ir::Function, index: TableIndex) -> ir::Table {
        let pointer_type = self.pointer_type();

        let (table, def_index) = if let Some(def_index) = self.module.defined_table_index(index) {
            let table = self.get_tables_base(func);
            (table, def_index)
        } else {
            let imported_tables_base = self.get_imported_tables_base(func);
            let from_offset = self.offsets.index_vmtable_import_from(index);
            let table = func.create_global_value(ir::GlobalValueData::Load {
                base: imported_tables_base,
                offset: Offset32::new(from_offset),
                global_type: pointer_type,
                readonly: true,
            });
            (table, DefinedTableIndex::new(0))
        };
        let base_offset = self.offsets.index_vmtable_definition_base(def_index);
        let current_elements_offset = self
            .offsets
            .index_vmtable_definition_current_elements(def_index);

        let base_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: table,
            offset: Offset32::new(base_offset),
            global_type: pointer_type,
            readonly: false,
        });
        let bound_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: table,
            offset: Offset32::new(current_elements_offset),
            global_type: self.offsets.type_of_vmtable_definition_current_elements(),
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

    fn make_heap(&mut self, func: &mut ir::Function, index: MemoryIndex) -> ir::Heap {
        let pointer_type = self.pointer_type();

        let (memory, def_index) = if let Some(def_index) = self.module.defined_memory_index(index) {
            let memory = self.get_memories_base(func);
            (memory, def_index)
        } else {
            let imported_memories_base = self.get_imported_memories_base(func);
            let from_offset = self.offsets.index_vmmemory_import_from(index);
            let memory = func.create_global_value(ir::GlobalValueData::Load {
                base: imported_memories_base,
                offset: Offset32::new(from_offset),
                global_type: pointer_type,
                readonly: true,
            });
            (memory, DefinedMemoryIndex::new(0))
        };
        let base_offset = self.offsets.index_vmmemory_definition_base(def_index);
        let current_length_offset = self
            .offsets
            .index_vmmemory_definition_current_length(def_index);

        // If we have a declared maximum, we can make this a "static" heap, which is
        // allocated up front and never moved.
        let (offset_guard_size, heap_style, readonly_base) = match self.module.memory_plans[index] {
            MemoryPlan {
                memory: _,
                style: MemoryStyle::Dynamic,
                offset_guard_size,
            } => {
                let heap_bound = func.create_global_value(ir::GlobalValueData::Load {
                    base: memory,
                    offset: Offset32::new(current_length_offset),
                    global_type: self.offsets.type_of_vmmemory_definition_current_length(),
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
            base: memory,
            offset: Offset32::new(base_offset),
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

    fn make_global(&mut self, func: &mut ir::Function, index: GlobalIndex) -> GlobalVariable {
        let pointer_type = self.pointer_type();

        let (global, def_index) = if let Some(def_index) = self.module.defined_global_index(index) {
            let global = self.get_globals_base(func);
            (global, def_index)
        } else {
            let imported_globals_base = self.get_imported_globals_base(func);
            let from_offset = self.offsets.index_vmglobal_import_from(index);
            let global = func.create_global_value(ir::GlobalValueData::Load {
                base: imported_globals_base,
                offset: Offset32::new(from_offset),
                global_type: pointer_type,
                readonly: true,
            });
            (global, DefinedGlobalIndex::new(0))
        };
        let offset = self.offsets.index_vmglobal_definition(def_index);

        GlobalVariable::Memory {
            gv: global,
            offset: offset.into(),
            ty: self.module.globals[index].ty,
        }
    }

    fn make_indirect_sig(&mut self, func: &mut ir::Function, index: SignatureIndex) -> ir::SigRef {
        func.import_signature(self.module.signatures[index].clone())
    }

    fn make_direct_func(&mut self, func: &mut ir::Function, index: FuncIndex) -> ir::FuncRef {
        let sigidx = self.module.functions[index];
        let signature = func.import_signature(self.module.signatures[sigidx].clone());
        let name = get_func_name(index);
        func.import_function(ir::ExtFuncData {
            name,
            signature,
            // We currently allocate all code segments independently, so nothing
            // is colocated.
            colocated: false,
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

        // If necessary, check the signature.
        match self.module.table_plans[table_index].style {
            TableStyle::CallerChecksSignature => {
                let sig_id_size = self.offsets.size_of_vmshared_signature_index();
                let sig_id_type = Type::int(u16::from(sig_id_size) * 8).unwrap();
                let signature_ids_base = self.get_signature_ids_base(pos.func);
                let sig_ids = pos.ins().global_value(pointer_type, signature_ids_base);

                // Load the caller ID.
                let mut mem_flags = ir::MemFlags::trusted();
                mem_flags.set_readonly();
                let caller_sig_id = pos.ins().load(
                    sig_id_type,
                    mem_flags,
                    sig_ids,
                    cast::i32(
                        sig_index
                            .as_u32()
                            .checked_mul(u32::from(sig_id_size))
                            .unwrap(),
                    )
                    .unwrap(),
                );

                // Load the callee ID.
                let mem_flags = ir::MemFlags::trusted();
                let callee_sig_id = pos.ins().load(
                    sig_id_type,
                    mem_flags,
                    table_entry_addr,
                    i32::from(self.offsets.vmcaller_checked_anyfunc_type_index()),
                );

                // Check that they match.
                let cmp = pos.ins().icmp(IntCC::Equal, callee_sig_id, caller_sig_id);
                pos.ins().trapz(cmp, ir::TrapCode::BadSignature);
            }
        }

        // Dereference table_entry_addr to get the function address.
        let mem_flags = ir::MemFlags::trusted();
        let func_addr = pos.ins().load(
            pointer_type,
            mem_flags,
            table_entry_addr,
            i32::from(self.offsets.vmcaller_checked_anyfunc_func_ptr()),
        );

        let mut real_call_args = Vec::with_capacity(call_args.len() + 1);
        real_call_args.extend_from_slice(call_args);

        // Append the callee vmctx address.
        let vmctx = pos.ins().load(
            pointer_type,
            mem_flags,
            table_entry_addr,
            i32::from(self.offsets.vmcaller_checked_anyfunc_vmctx()),
        );
        real_call_args.push(vmctx);

        Ok(pos.ins().call_indirect(sig_ref, func_addr, &real_call_args))
    }

    fn translate_call(
        &mut self,
        mut pos: FuncCursor,
        callee_index: FuncIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        let mut real_call_args = Vec::with_capacity(call_args.len() + 1);
        real_call_args.extend_from_slice(call_args);

        // Handle direct calls to locally-defined functions.
        if !self.module.is_imported_function(callee_index) {
            real_call_args.push(pos.func.special_param(ArgumentPurpose::VMContext).unwrap());
            return Ok(pos.ins().call(callee, &real_call_args));
        }

        // Handle direct calls to imported functions. We use an indirect call
        // so that we don't have to patch the code at runtime.
        let pointer_type = self.pointer_type();
        let sig_ref = pos.func.dfg.ext_funcs[callee].signature;
        let imported_functions_base = self.get_imported_functions_base(&mut pos.func);
        let base = pos
            .ins()
            .global_value(pointer_type, imported_functions_base);

        let mem_flags = ir::MemFlags::trusted();

        // Load the callee address.
        let body_offset = self.offsets.index_vmfunction_import_body(callee_index);
        let func_addr = pos.ins().load(pointer_type, mem_flags, base, body_offset);

        // Append the callee vmctx address.
        let vmctx_offset = self.offsets.index_vmfunction_import_vmctx(callee_index);
        let vmctx = pos.ins().load(pointer_type, mem_flags, base, vmctx_offset);
        real_call_args.push(vmctx);

        Ok(pos.ins().call_indirect(sig_ref, func_addr, &real_call_args))
    }

    fn translate_memory_grow(
        &mut self,
        mut pos: FuncCursor,
        index: MemoryIndex,
        _heap: ir::Heap,
        val: ir::Value,
    ) -> WasmResult<ir::Value> {
        let (memory_grow_func, index_arg) = self.get_memory_grow_func(&mut pos.func, index);
        let memory_index = pos.ins().iconst(I32, index_arg as i64);
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
        let (memory_size_func, index_arg) = self.get_memory_size_func(&mut pos.func, index);
        let memory_index = pos.ins().iconst(I32, index_arg as i64);
        let vmctx = pos.func.special_param(ArgumentPurpose::VMContext).unwrap();
        let call_inst = pos.ins().call(memory_size_func, &[memory_index, vmctx]);
        Ok(*pos.func.dfg.inst_results(call_inst).first().unwrap())
    }
}
