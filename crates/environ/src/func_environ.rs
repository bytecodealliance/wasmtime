use crate::module::{MemoryPlan, MemoryStyle, Module, TableStyle};
use crate::vmoffsets::VMOffsets;
use crate::WASM_PAGE_SIZE;
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir;
use cranelift_codegen::ir::condcodes::*;
use cranelift_codegen::ir::immediates::{Offset32, Uimm64};
use cranelift_codegen::ir::types::*;
use cranelift_codegen::ir::{AbiParam, ArgumentPurpose, Function, InstBuilder, Signature};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_entity::EntityRef;
use cranelift_wasm::{
    self, FuncIndex, GlobalIndex, GlobalVariable, MemoryIndex, SignatureIndex, TableIndex,
    TargetEnvironment, WasmError, WasmResult,
};
#[cfg(feature = "lightbeam")]
use cranelift_wasm::{DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex};
use std::convert::TryFrom;

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

/// An index type for builtin functions.
pub struct BuiltinFunctionIndex(u32);

impl BuiltinFunctionIndex {
    /// Returns an index for wasm's `memory.grow` builtin function.
    pub const fn get_memory32_grow_index() -> Self {
        Self(0)
    }
    /// Returns an index for wasm's imported `memory.grow` builtin function.
    pub const fn get_imported_memory32_grow_index() -> Self {
        Self(1)
    }
    /// Returns an index for wasm's `memory.size` builtin function.
    pub const fn get_memory32_size_index() -> Self {
        Self(2)
    }
    /// Returns an index for wasm's imported `memory.size` builtin function.
    pub const fn get_imported_memory32_size_index() -> Self {
        Self(3)
    }
    /// Returns the total number of builtin functions.
    pub const fn builtin_functions_total_number() -> u32 {
        4
    }

    /// Return the index as an u32 number.
    pub const fn index(&self) -> u32 {
        self.0
    }
}

/// The `FuncEnvironment` implementation for use by the `ModuleEnvironment`.
pub struct FuncEnvironment<'module_environment> {
    /// Target-specified configuration.
    target_config: TargetFrontendConfig,

    /// The module-level environment which this function-level environment belongs to.
    module: &'module_environment Module,

    /// The Cranelift global holding the vmctx address.
    vmctx: Option<ir::GlobalValue>,

    /// The external function signature for implementing wasm's `memory.size`
    /// for locally-defined 32-bit memories.
    memory32_size_sig: Option<ir::SigRef>,

    /// The external function signature for implementing wasm's `memory.grow`
    /// for locally-defined memories.
    memory_grow_sig: Option<ir::SigRef>,

    /// Offsets to struct fields accessed by JIT code.
    offsets: VMOffsets,
}

impl<'module_environment> FuncEnvironment<'module_environment> {
    pub fn new(target_config: TargetFrontendConfig, module: &'module_environment Module) -> Self {
        Self {
            target_config,
            module,
            vmctx: None,
            memory32_size_sig: None,
            memory_grow_sig: None,
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

    fn get_memory_grow_sig(&mut self, func: &mut Function) -> ir::SigRef {
        let sig = self.memory_grow_sig.unwrap_or_else(|| {
            func.import_signature(Signature {
                params: vec![
                    AbiParam::special(self.pointer_type(), ArgumentPurpose::VMContext),
                    AbiParam::new(I32),
                    AbiParam::new(I32),
                ],
                returns: vec![AbiParam::new(I32)],
                call_conv: self.target_config.default_call_conv,
            })
        });
        self.memory_grow_sig = Some(sig);
        sig
    }

    /// Return the memory.grow function signature to call for the given index, along with the
    /// translated index value to pass to it and its index in `VMBuiltinFunctionsArray`.
    fn get_memory_grow_func(
        &mut self,
        func: &mut Function,
        index: MemoryIndex,
    ) -> (ir::SigRef, usize, BuiltinFunctionIndex) {
        if self.module.is_imported_memory(index) {
            (
                self.get_memory_grow_sig(func),
                index.index(),
                BuiltinFunctionIndex::get_imported_memory32_grow_index(),
            )
        } else {
            (
                self.get_memory_grow_sig(func),
                self.module.defined_memory_index(index).unwrap().index(),
                BuiltinFunctionIndex::get_memory32_grow_index(),
            )
        }
    }

    fn get_memory32_size_sig(&mut self, func: &mut Function) -> ir::SigRef {
        let sig = self.memory32_size_sig.unwrap_or_else(|| {
            func.import_signature(Signature {
                params: vec![
                    AbiParam::special(self.pointer_type(), ArgumentPurpose::VMContext),
                    AbiParam::new(I32),
                ],
                returns: vec![AbiParam::new(I32)],
                call_conv: self.target_config.default_call_conv,
            })
        });
        self.memory32_size_sig = Some(sig);
        sig
    }

    /// Return the memory.size function signature to call for the given index, along with the
    /// translated index value to pass to it and its index in `VMBuiltinFunctionsArray`.
    fn get_memory_size_func(
        &mut self,
        func: &mut Function,
        index: MemoryIndex,
    ) -> (ir::SigRef, usize, BuiltinFunctionIndex) {
        if self.module.is_imported_memory(index) {
            (
                self.get_memory32_size_sig(func),
                index.index(),
                BuiltinFunctionIndex::get_imported_memory32_size_index(),
            )
        } else {
            (
                self.get_memory32_size_sig(func),
                self.module.defined_memory_index(index).unwrap().index(),
                BuiltinFunctionIndex::get_memory32_size_index(),
            )
        }
    }

    /// Translates load of builtin function and returns a pair of values `vmctx`
    /// and address of the loaded function.
    fn translate_load_builtin_function_address(
        &mut self,
        pos: &mut FuncCursor<'_>,
        callee_func_idx: BuiltinFunctionIndex,
    ) -> (ir::Value, ir::Value) {
        // We use an indirect call so that we don't have to patch the code at runtime.
        let pointer_type = self.pointer_type();
        let vmctx = self.vmctx(&mut pos.func);
        let base = pos.ins().global_value(pointer_type, vmctx);

        let mut mem_flags = ir::MemFlags::trusted();
        mem_flags.set_readonly();

        // Load the callee address.
        let body_offset =
            i32::try_from(self.offsets.vmctx_builtin_function(callee_func_idx)).unwrap();
        let func_addr = pos.ins().load(pointer_type, mem_flags, base, body_offset);

        (base, func_addr)
    }
}

#[cfg(feature = "lightbeam")]
impl lightbeam::ModuleContext for FuncEnvironment<'_> {
    type Signature = ir::Signature;
    type GlobalType = ir::Type;

    fn func_index(&self, defined_func_index: u32) -> u32 {
        self.module
            .func_index(DefinedFuncIndex::from_u32(defined_func_index))
            .as_u32()
    }

    fn defined_func_index(&self, func_index: u32) -> Option<u32> {
        self.module
            .defined_func_index(FuncIndex::from_u32(func_index))
            .map(DefinedFuncIndex::as_u32)
    }

    fn defined_global_index(&self, global_index: u32) -> Option<u32> {
        self.module
            .defined_global_index(GlobalIndex::from_u32(global_index))
            .map(DefinedGlobalIndex::as_u32)
    }

    fn global_type(&self, global_index: u32) -> &Self::GlobalType {
        &self.module.globals[GlobalIndex::from_u32(global_index)].ty
    }

    fn func_type_index(&self, func_idx: u32) -> u32 {
        self.module.functions[FuncIndex::from_u32(func_idx)].as_u32()
    }

    fn signature(&self, index: u32) -> &Self::Signature {
        &self.module.signatures[SignatureIndex::from_u32(index)]
    }

    fn defined_table_index(&self, table_index: u32) -> Option<u32> {
        self.module
            .defined_table_index(TableIndex::from_u32(table_index))
            .map(DefinedTableIndex::as_u32)
    }

    fn defined_memory_index(&self, memory_index: u32) -> Option<u32> {
        self.module
            .defined_memory_index(MemoryIndex::from_u32(memory_index))
            .map(DefinedMemoryIndex::as_u32)
    }

    fn vmctx_vmfunction_import_body(&self, func_index: u32) -> u32 {
        self.offsets
            .vmctx_vmfunction_import_body(FuncIndex::from_u32(func_index))
    }
    fn vmctx_vmfunction_import_vmctx(&self, func_index: u32) -> u32 {
        self.offsets
            .vmctx_vmfunction_import_vmctx(FuncIndex::from_u32(func_index))
    }

    fn vmctx_vmglobal_import_from(&self, global_index: u32) -> u32 {
        self.offsets
            .vmctx_vmglobal_import_from(GlobalIndex::from_u32(global_index))
    }
    fn vmctx_vmglobal_definition(&self, defined_global_index: u32) -> u32 {
        self.offsets
            .vmctx_vmglobal_definition(DefinedGlobalIndex::from_u32(defined_global_index))
    }
    fn vmctx_vmmemory_import_from(&self, memory_index: u32) -> u32 {
        self.offsets
            .vmctx_vmmemory_import_from(MemoryIndex::from_u32(memory_index))
    }
    fn vmctx_vmmemory_definition(&self, defined_memory_index: u32) -> u32 {
        self.offsets
            .vmctx_vmmemory_definition(DefinedMemoryIndex::from_u32(defined_memory_index))
    }
    fn vmctx_vmmemory_definition_base(&self, defined_memory_index: u32) -> u32 {
        self.offsets
            .vmctx_vmmemory_definition_base(DefinedMemoryIndex::from_u32(defined_memory_index))
    }
    fn vmctx_vmmemory_definition_current_length(&self, defined_memory_index: u32) -> u32 {
        self.offsets
            .vmctx_vmmemory_definition_current_length(DefinedMemoryIndex::from_u32(
                defined_memory_index,
            ))
    }
    fn vmmemory_definition_base(&self) -> u8 {
        self.offsets.vmmemory_definition_base()
    }
    fn vmmemory_definition_current_length(&self) -> u8 {
        self.offsets.vmmemory_definition_current_length()
    }
    fn vmctx_vmtable_import_from(&self, table_index: u32) -> u32 {
        self.offsets
            .vmctx_vmtable_import_from(TableIndex::from_u32(table_index))
    }
    fn vmctx_vmtable_definition(&self, defined_table_index: u32) -> u32 {
        self.offsets
            .vmctx_vmtable_definition(DefinedTableIndex::from_u32(defined_table_index))
    }
    fn vmctx_vmtable_definition_base(&self, defined_table_index: u32) -> u32 {
        self.offsets
            .vmctx_vmtable_definition_base(DefinedTableIndex::from_u32(defined_table_index))
    }
    fn vmctx_vmtable_definition_current_elements(&self, defined_table_index: u32) -> u32 {
        self.offsets
            .vmctx_vmtable_definition_current_elements(DefinedTableIndex::from_u32(
                defined_table_index,
            ))
    }
    fn vmtable_definition_base(&self) -> u8 {
        self.offsets.vmtable_definition_base()
    }
    fn vmtable_definition_current_elements(&self) -> u8 {
        self.offsets.vmtable_definition_current_elements()
    }
    fn vmcaller_checked_anyfunc_type_index(&self) -> u8 {
        self.offsets.vmcaller_checked_anyfunc_type_index()
    }
    fn vmcaller_checked_anyfunc_func_ptr(&self) -> u8 {
        self.offsets.vmcaller_checked_anyfunc_func_ptr()
    }
    fn vmcaller_checked_anyfunc_vmctx(&self) -> u8 {
        self.offsets.vmcaller_checked_anyfunc_vmctx()
    }
    fn size_of_vmcaller_checked_anyfunc(&self) -> u8 {
        self.offsets.size_of_vmcaller_checked_anyfunc()
    }
    fn vmctx_vmshared_signature_id(&self, signature_idx: u32) -> u32 {
        self.offsets
            .vmctx_vmshared_signature_id(SignatureIndex::from_u32(signature_idx))
    }

    // TODO: type of a global
}

impl<'module_environment> TargetEnvironment for FuncEnvironment<'module_environment> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.target_config
    }
}

impl<'module_environment> cranelift_wasm::FuncEnvironment for FuncEnvironment<'module_environment> {
    fn is_wasm_parameter(&self, signature: &ir::Signature, index: usize) -> bool {
        signature.params[index].purpose == ir::ArgumentPurpose::Normal
    }

    fn make_table(&mut self, func: &mut ir::Function, index: TableIndex) -> WasmResult<ir::Table> {
        let pointer_type = self.pointer_type();

        let (ptr, base_offset, current_elements_offset) = {
            let vmctx = self.vmctx(func);
            if let Some(def_index) = self.module.defined_table_index(index) {
                let base_offset =
                    i32::try_from(self.offsets.vmctx_vmtable_definition_base(def_index)).unwrap();
                let current_elements_offset = i32::try_from(
                    self.offsets
                        .vmctx_vmtable_definition_current_elements(def_index),
                )
                .unwrap();
                (vmctx, base_offset, current_elements_offset)
            } else {
                let from_offset = self.offsets.vmctx_vmtable_import_from(index);
                let table = func.create_global_value(ir::GlobalValueData::Load {
                    base: vmctx,
                    offset: Offset32::new(i32::try_from(from_offset).unwrap()),
                    global_type: pointer_type,
                    readonly: true,
                });
                let base_offset = i32::from(self.offsets.vmtable_definition_base());
                let current_elements_offset =
                    i32::from(self.offsets.vmtable_definition_current_elements());
                (table, base_offset, current_elements_offset)
            }
        };

        let base_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: ptr,
            offset: Offset32::new(base_offset),
            global_type: pointer_type,
            readonly: false,
        });
        let bound_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: ptr,
            offset: Offset32::new(current_elements_offset),
            global_type: self.offsets.type_of_vmtable_definition_current_elements(),
            readonly: false,
        });

        let element_size = match self.module.table_plans[index].style {
            TableStyle::CallerChecksSignature => {
                u64::from(self.offsets.size_of_vmcaller_checked_anyfunc())
            }
        };

        Ok(func.create_table(ir::TableData {
            base_gv,
            min_size: Uimm64::new(0),
            bound_gv,
            element_size: Uimm64::new(element_size),
            index_type: I32,
        }))
    }

    fn make_heap(&mut self, func: &mut ir::Function, index: MemoryIndex) -> WasmResult<ir::Heap> {
        let pointer_type = self.pointer_type();

        let (ptr, base_offset, current_length_offset) = {
            let vmctx = self.vmctx(func);
            if let Some(def_index) = self.module.defined_memory_index(index) {
                let base_offset =
                    i32::try_from(self.offsets.vmctx_vmmemory_definition_base(def_index)).unwrap();
                let current_length_offset = i32::try_from(
                    self.offsets
                        .vmctx_vmmemory_definition_current_length(def_index),
                )
                .unwrap();
                (vmctx, base_offset, current_length_offset)
            } else {
                let from_offset = self.offsets.vmctx_vmmemory_import_from(index);
                let memory = func.create_global_value(ir::GlobalValueData::Load {
                    base: vmctx,
                    offset: Offset32::new(i32::try_from(from_offset).unwrap()),
                    global_type: pointer_type,
                    readonly: true,
                });
                let base_offset = i32::from(self.offsets.vmmemory_definition_base());
                let current_length_offset =
                    i32::from(self.offsets.vmmemory_definition_current_length());
                (memory, base_offset, current_length_offset)
            }
        };

        // If we have a declared maximum, we can make this a "static" heap, which is
        // allocated up front and never moved.
        let (offset_guard_size, heap_style, readonly_base) = match self.module.memory_plans[index] {
            MemoryPlan {
                style: MemoryStyle::Dynamic,
                offset_guard_size,
                memory: _,
            } => {
                let heap_bound = func.create_global_value(ir::GlobalValueData::Load {
                    base: ptr,
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
                style: MemoryStyle::Static { bound },
                offset_guard_size,
                memory: _,
            } => (
                Uimm64::new(offset_guard_size),
                ir::HeapStyle::Static {
                    bound: Uimm64::new(u64::from(bound) * u64::from(WASM_PAGE_SIZE)),
                },
                true,
            ),
        };

        let heap_base = func.create_global_value(ir::GlobalValueData::Load {
            base: ptr,
            offset: Offset32::new(base_offset),
            global_type: pointer_type,
            readonly: readonly_base,
        });
        Ok(func.create_heap(ir::HeapData {
            base: heap_base,
            min_size: 0.into(),
            offset_guard_size,
            style: heap_style,
            index_type: I32,
        }))
    }

    fn make_global(
        &mut self,
        func: &mut ir::Function,
        index: GlobalIndex,
    ) -> WasmResult<GlobalVariable> {
        let pointer_type = self.pointer_type();

        let (ptr, offset) = {
            let vmctx = self.vmctx(func);
            if let Some(def_index) = self.module.defined_global_index(index) {
                let offset =
                    i32::try_from(self.offsets.vmctx_vmglobal_definition(def_index)).unwrap();
                (vmctx, offset)
            } else {
                let from_offset = self.offsets.vmctx_vmglobal_import_from(index);
                let global = func.create_global_value(ir::GlobalValueData::Load {
                    base: vmctx,
                    offset: Offset32::new(i32::try_from(from_offset).unwrap()),
                    global_type: pointer_type,
                    readonly: true,
                });
                (global, 0)
            }
        };

        Ok(GlobalVariable::Memory {
            gv: ptr,
            offset: offset.into(),
            ty: self.module.globals[index].ty,
        })
    }

    fn make_indirect_sig(
        &mut self,
        func: &mut ir::Function,
        index: SignatureIndex,
    ) -> WasmResult<ir::SigRef> {
        Ok(func.import_signature(self.module.signatures[index].clone()))
    }

    fn make_direct_func(
        &mut self,
        func: &mut ir::Function,
        index: FuncIndex,
    ) -> WasmResult<ir::FuncRef> {
        let sigidx = self.module.functions[index];
        let signature = func.import_signature(self.module.signatures[sigidx].clone());
        let name = get_func_name(index);
        Ok(func.import_function(ir::ExtFuncData {
            name,
            signature,
            // We currently allocate all code segments independently, so nothing
            // is colocated.
            colocated: false,
        }))
    }

    fn translate_call_indirect(
        &mut self,
        mut pos: FuncCursor<'_>,
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
        let mem_flags = ir::MemFlags::trusted();
        let func_addr = pos.ins().load(
            pointer_type,
            mem_flags,
            table_entry_addr,
            i32::from(self.offsets.vmcaller_checked_anyfunc_func_ptr()),
        );

        // Check whether `func_addr` is null.
        pos.ins().trapz(func_addr, ir::TrapCode::IndirectCallToNull);

        // If necessary, check the signature.
        match self.module.table_plans[table_index].style {
            TableStyle::CallerChecksSignature => {
                let sig_id_size = self.offsets.size_of_vmshared_signature_index();
                let sig_id_type = Type::int(u16::from(sig_id_size) * 8).unwrap();
                let vmctx = self.vmctx(pos.func);
                let base = pos.ins().global_value(pointer_type, vmctx);
                let offset =
                    i32::try_from(self.offsets.vmctx_vmshared_signature_id(sig_index)).unwrap();

                // Load the caller ID.
                let mut mem_flags = ir::MemFlags::trusted();
                mem_flags.set_readonly();
                let caller_sig_id = pos.ins().load(sig_id_type, mem_flags, base, offset);

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

        let mut real_call_args = Vec::with_capacity(call_args.len() + 1);

        // First append the callee vmctx address.
        let vmctx = pos.ins().load(
            pointer_type,
            mem_flags,
            table_entry_addr,
            i32::from(self.offsets.vmcaller_checked_anyfunc_vmctx()),
        );
        real_call_args.push(vmctx);

        // Then append the regular call arguments.
        real_call_args.extend_from_slice(call_args);

        Ok(pos.ins().call_indirect(sig_ref, func_addr, &real_call_args))
    }

    fn translate_call(
        &mut self,
        mut pos: FuncCursor<'_>,
        callee_index: FuncIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        let mut real_call_args = Vec::with_capacity(call_args.len() + 1);

        // Handle direct calls to locally-defined functions.
        if !self.module.is_imported_function(callee_index) {
            // First append the callee vmctx address.
            real_call_args.push(pos.func.special_param(ArgumentPurpose::VMContext).unwrap());

            // Then append the regular call arguments.
            real_call_args.extend_from_slice(call_args);

            return Ok(pos.ins().call(callee, &real_call_args));
        }

        // Handle direct calls to imported functions. We use an indirect call
        // so that we don't have to patch the code at runtime.
        let pointer_type = self.pointer_type();
        let sig_ref = pos.func.dfg.ext_funcs[callee].signature;
        let vmctx = self.vmctx(&mut pos.func);
        let base = pos.ins().global_value(pointer_type, vmctx);

        let mem_flags = ir::MemFlags::trusted();

        // Load the callee address.
        let body_offset =
            i32::try_from(self.offsets.vmctx_vmfunction_import_body(callee_index)).unwrap();
        let func_addr = pos.ins().load(pointer_type, mem_flags, base, body_offset);

        // First append the callee vmctx address.
        let vmctx_offset =
            i32::try_from(self.offsets.vmctx_vmfunction_import_vmctx(callee_index)).unwrap();
        let vmctx = pos.ins().load(pointer_type, mem_flags, base, vmctx_offset);
        real_call_args.push(vmctx);

        // Then append the regular call arguments.
        real_call_args.extend_from_slice(call_args);

        Ok(pos.ins().call_indirect(sig_ref, func_addr, &real_call_args))
    }

    fn translate_memory_grow(
        &mut self,
        mut pos: FuncCursor<'_>,
        index: MemoryIndex,
        _heap: ir::Heap,
        val: ir::Value,
    ) -> WasmResult<ir::Value> {
        let (func_sig, index_arg, func_idx) = self.get_memory_grow_func(&mut pos.func, index);
        let memory_index = pos.ins().iconst(I32, index_arg as i64);
        let (vmctx, func_addr) = self.translate_load_builtin_function_address(&mut pos, func_idx);
        let call_inst = pos
            .ins()
            .call_indirect(func_sig, func_addr, &[vmctx, val, memory_index]);
        Ok(*pos.func.dfg.inst_results(call_inst).first().unwrap())
    }

    fn translate_memory_size(
        &mut self,
        mut pos: FuncCursor<'_>,
        index: MemoryIndex,
        _heap: ir::Heap,
    ) -> WasmResult<ir::Value> {
        let (func_sig, index_arg, func_idx) = self.get_memory_size_func(&mut pos.func, index);
        let memory_index = pos.ins().iconst(I32, index_arg as i64);
        let (vmctx, func_addr) = self.translate_load_builtin_function_address(&mut pos, func_idx);
        let call_inst = pos
            .ins()
            .call_indirect(func_sig, func_addr, &[vmctx, memory_index]);
        Ok(*pos.func.dfg.inst_results(call_inst).first().unwrap())
    }

    fn translate_memory_copy(
        &mut self,
        _pos: FuncCursor,
        _index: MemoryIndex,
        _heap: ir::Heap,
        _dst: ir::Value,
        _src: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Err(WasmError::Unsupported("bulk memory".to_string()))
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
        Err(WasmError::Unsupported("bulk memory".to_string()))
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
        Err(WasmError::Unsupported("bulk memory".to_string()))
    }

    fn translate_data_drop(&mut self, _pos: FuncCursor, _seg_index: u32) -> WasmResult<()> {
        Err(WasmError::Unsupported("bulk memory".to_string()))
    }

    fn translate_table_size(
        &mut self,
        _pos: FuncCursor,
        _index: TableIndex,
        _table: ir::Table,
    ) -> WasmResult<ir::Value> {
        Err(WasmError::Unsupported("bulk memory".to_string()))
    }

    fn translate_table_copy(
        &mut self,
        _pos: FuncCursor,
        _dst_table_index: TableIndex,
        _dst_table: ir::Table,
        _src_table_index: TableIndex,
        _src_table: ir::Table,
        _dst: ir::Value,
        _src: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Err(WasmError::Unsupported("bulk memory".to_string()))
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
        Err(WasmError::Unsupported("bulk memory".to_string()))
    }

    fn translate_elem_drop(&mut self, _pos: FuncCursor, _seg_index: u32) -> WasmResult<()> {
        Err(WasmError::Unsupported("bulk memory".to_string()))
    }
}
