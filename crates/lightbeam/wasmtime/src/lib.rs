//! Support for compiling with Lightbeam.
//!
//! This crates provides an implementation of [`Compiler`] in the form of
//! [`Lightbeam`].

#![allow(dead_code)]

use anyhow::Result;
use cranelift_codegen::binemit;
use cranelift_codegen::ir::{self, ExternalName};
use object::write::Object;
use std::any::Any;
use std::collections::BTreeMap;
use wasmtime_environ::{
    BuiltinFunctionIndex, CompileError, Compiler, FlagValue, FunctionBodyData, FunctionInfo,
    Module, ModuleTranslation, PrimaryMap, TrapInformation, Tunables, TypeTables, VMOffsets,
};
use wasmtime_environ::{
    DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex,
    GlobalIndex, MemoryIndex, TableIndex, Trampoline, TypeIndex, WasmFuncType,
};

/// A compiler that compiles a WebAssembly module with Lightbeam, directly translating the Wasm file.
pub struct Lightbeam;

impl Compiler for Lightbeam {
    fn compile_function(
        &self,
        _translation: &ModuleTranslation,
        _i: DefinedFuncIndex,
        _function_body: FunctionBodyData<'_>,
        _tunables: &Tunables,
        _types: &TypeTables,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        unimplemented!()
        // if tunables.generate_native_debuginfo {
        //     return Err(CompileError::DebugInfoNotSupported);
        // }
        // let func_index = translation.module.func_index(i);

        // let env = FuncEnvironment::new(isa.frontend_config().pointer_bytes(), translation);
        // let mut codegen_session: CodeGenSession<_> = CodeGenSession::new(
        //     translation.function_body_inputs.len() as u32,
        //     &env,
        //     lightbeam::microwasm::I32,
        // );

        // let mut reloc_sink = RelocSink::new(func_index);
        // let mut trap_sink = TrapSink::new();
        // lightbeam::translate_function(
        //     &mut codegen_session,
        //     Sinks {
        //         relocs: &mut reloc_sink,
        //         traps: &mut trap_sink,
        //         offsets: &mut NullOffsetSink,
        //     },
        //     i.as_u32(),
        //     function_body.body,
        // )
        // .map_err(|e| CompileError::Codegen(format!("Failed to translate function: {}", e)))?;

        // let code_section = codegen_session
        //     .into_translated_code_section()
        //     .map_err(|e| CompileError::Codegen(format!("Failed to generate output code: {}", e)))?;

        // Ok(CompiledFunction {
        //     // TODO: try to remove copy here (?)
        //     body: code_section.buffer().to_vec(),
        //     traps: trap_sink.traps,
        //     relocations: reloc_sink.func_relocs,

        //     // not implemented for lightbeam currently
        //     unwind_info: None,
        //     stack_maps: Default::default(),
        //     stack_slots: Default::default(),
        //     value_labels_ranges: Default::default(),
        //     address_map: Default::default(),
        //     jt_offsets: Default::default(),
        // })
    }

    fn emit_obj(
        &self,
        _module: &ModuleTranslation,
        _types: &TypeTables,
        _funcs: PrimaryMap<DefinedFuncIndex, Box<dyn Any + Send>>,
        _emit_dwarf: bool,
        _obj: &mut Object,
    ) -> Result<(PrimaryMap<DefinedFuncIndex, FunctionInfo>, Vec<Trampoline>)> {
        unimplemented!()
    }

    fn emit_trampoline_obj(
        &self,
        _ty: &WasmFuncType,
        _host_fn: usize,
        _obj: &mut Object,
    ) -> Result<(Trampoline, Trampoline)> {
        unimplemented!()
    }

    fn triple(&self) -> &target_lexicon::Triple {
        unimplemented!()
    }

    fn flags(&self) -> BTreeMap<String, FlagValue> {
        unimplemented!()
    }

    fn isa_flags(&self) -> BTreeMap<String, FlagValue> {
        unimplemented!()
    }
}

/// Implementation of a relocation sink that just saves all the information for later
struct RelocSink {
    /// Current function index.
    func_index: FuncIndex,
    // /// Relocations recorded for the function.
    // func_relocs: Vec<Relocation>,
}

impl binemit::RelocSink for RelocSink {
    fn reloc_external(
        &mut self,
        _offset: binemit::CodeOffset,
        _srcloc: ir::SourceLoc,
        _reloc: binemit::Reloc,
        _name: &ExternalName,
        _addend: binemit::Addend,
    ) {
        unimplemented!()
        // let reloc_target = if let ExternalName::User { namespace, index } = *name {
        //     debug_assert_eq!(namespace, 0);
        //     RelocationTarget::UserFunc(FuncIndex::from_u32(index))
        // } else if let ExternalName::LibCall(libcall) = *name {
        //     RelocationTarget::LibCall(libcall)
        // } else {
        //     panic!("unrecognized external name")
        // };
        // self.func_relocs.push(Relocation {
        //     reloc,
        //     reloc_target,
        //     offset,
        //     addend,
        // });
    }

    fn reloc_constant(
        &mut self,
        _code_offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _constant_offset: ir::ConstantOffset,
    ) {
        // Do nothing for now: cranelift emits constant data after the function code and also emits
        // function code with correct relative offsets to the constant data.
    }

    fn reloc_jt(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _jt: ir::JumpTable,
    ) {
        unimplemented!()
        // self.func_relocs.push(Relocation {
        //     reloc,
        //     reloc_target: RelocationTarget::JumpTable(self.func_index, jt),
        //     offset,
        //     addend: 0,
        // });
    }
}

impl RelocSink {
    /// Return a new `RelocSink` instance.
    fn new(func_index: FuncIndex) -> Self {
        Self {
            func_index,
            // func_relocs: Vec::new(),
        }
    }
}

/// Implementation of a trap sink that simply stores all trap info in-memory
#[derive(Default)]
struct TrapSink {
    /// The in-memory vector of trap info
    traps: Vec<TrapInformation>,
}

impl TrapSink {
    /// Create a new `TrapSink`
    fn new() -> Self {
        Self::default()
    }
}

impl binemit::TrapSink for TrapSink {
    fn trap(
        &mut self,
        _code_offset: binemit::CodeOffset,
        _source_loc: ir::SourceLoc,
        _trap_code: ir::TrapCode,
    ) {
        unimplemented!()
        // self.traps.push(TrapInformation {
        //     code_offset,
        //     trap_code,
        // });
    }
}

/// The `FuncEnvironment` implementation for use by the `ModuleEnvironment`.
struct FuncEnvironment<'module_environment> {
    /// The module-level environment which this function-level environment belongs to.
    module: &'module_environment Module,

    /// Offsets to struct fields accessed by JIT code.
    offsets: VMOffsets<u8>,
}

impl<'module_environment> FuncEnvironment<'module_environment> {
    fn new(pointer_bytes: u8, translation: &'module_environment ModuleTranslation<'_>) -> Self {
        Self {
            module: &translation.module,
            offsets: VMOffsets::new(pointer_bytes, &translation.module),
        }
    }
}

// TODO: This is necessary as if Lightbeam used `FuncEnvironment` directly it would cause
//       a circular dependency graph. We should extract common types out into a separate
//       crate that Lightbeam can use but until then we need this trait.
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

    fn global_type(&self, _global_index: u32) -> &Self::GlobalType {
        unimplemented!()
        // &self.module.globals[GlobalIndex::from_u32(global_index)].ty
    }

    fn func_type_index(&self, func_idx: u32) -> u32 {
        self.module.functions[FuncIndex::from_u32(func_idx)].as_u32()
    }

    fn signature(&self, _index: u32) -> &Self::Signature {
        panic!("not implemented")
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

    fn vmctx_builtin_function(&self, func_index: u32) -> u32 {
        self.offsets
            .vmctx_builtin_function(BuiltinFunctionIndex::from_u32(func_index))
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
            .vmctx_vmshared_signature_id(TypeIndex::from_u32(signature_idx))
    }

    // TODO: type of a global
}
