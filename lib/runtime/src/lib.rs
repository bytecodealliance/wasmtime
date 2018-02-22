//! Standalone runtime for WebAssembly using Cretonne. Provides functions to translate
//! `get_global`, `set_global`, `current_memory`, `grow_memory`, `call_indirect` that hardcode in
//! the translation the base addresses of regions of memory that will hold the globals, tables and
//! linear memories.

#![deny(missing_docs)]

extern crate cretonne;
extern crate cton_wasm;
extern crate wasmparser;

pub mod module;
pub mod compilation;
pub mod instance;

pub use module::Module;
pub use compilation::Compilation;
pub use instance::Instance;

use cton_wasm::{FunctionIndex, GlobalIndex, TableIndex, MemoryIndex, Global, Table, Memory,
                GlobalValue, SignatureIndex, FuncTranslator};
use cretonne::ir::{InstBuilder, FuncRef, ExtFuncData, ExternalName, Signature, AbiParam, CallConv,
                   ArgumentPurpose, ArgumentLoc, ArgumentExtension, Function};
use cretonne::ir::types::*;
use cretonne::ir::immediates::Offset32;
use cretonne::cursor::FuncCursor;
use cretonne::ir;
use cretonne::isa;
use cretonne::settings;
use cretonne::binemit;
use std::error::Error;

/// Compute a `ir::ExternalName` for a given wasm function index.
pub fn get_func_name(func_index: FunctionIndex) -> cretonne::ir::ExternalName {
    debug_assert!(func_index as u32 as FunctionIndex == func_index);
    ir::ExternalName::user(0, func_index as u32)
}

/// An entity to export.
pub enum Export {
    /// Function export.
    Function(FunctionIndex),
    /// Table export.
    Table(TableIndex),
    /// Memory export.
    Memory(MemoryIndex),
    /// Global export.
    Global(GlobalIndex),
}

/// Implementation of a relocation sink that just saves all the information for later
pub struct RelocSink<'func> {
    func: &'func ir::Function,
    /// Relocations recorded for the function.
    pub func_relocs: Vec<Relocation>,
}

impl<'func> binemit::RelocSink for RelocSink<'func> {
    fn reloc_ebb(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _ebb_offset: binemit::CodeOffset,
    ) {
        // This should use the `offsets` field of `ir::Function`.
        panic!("ebb headers not yet implemented");
    }
    fn reloc_external(
        &mut self,
        offset: binemit::CodeOffset,
        reloc: binemit::Reloc,
        name: &ExternalName,
        addend: binemit::Addend,
    ) {
        // FIXME: Handle grow_memory/current_memory.
        let func_index = if let ExternalName::User { namespace, index } = *name {
            debug_assert!(namespace == 0);
            index
        } else {
            panic!("unrecognized external name")
        } as usize;
        self.func_relocs.push(Relocation {
            reloc,
            func_index,
            offset,
            addend,
        });
    }
    fn reloc_jt(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        jt: ir::JumpTable,
    ) {
        let _jump_table = &self.func.jump_tables[jt];
        panic!("jump tables not yet implemented");
    }
}

impl<'func> RelocSink<'func> {
    fn new(func: &'func Function) -> RelocSink {
        RelocSink {
            func,
            func_relocs: Vec::new(),
        }
    }
}

/// A data initializer for linear memory.
pub struct DataInitializer<'data> {
    /// The index of the memory to initialize.
    pub memory_index: MemoryIndex,
    /// Optionally a globalvar base to initialize at.
    pub base: Option<GlobalIndex>,
    /// A constant offset to initialize at.
    pub offset: usize,
    /// The initialization data.
    pub data: &'data [u8],
}

/// References to the input wasm data buffer to be decoded and processed later.
/// separately from the main module translation.
pub struct LazyContents<'data> {
    /// References to the function bodies.
    pub function_body_inputs: Vec<&'data [u8]>,

    /// References to the data initializers.
    pub data_initializers: Vec<DataInitializer<'data>>,
}

impl<'data> LazyContents<'data> {
    fn new() -> Self {
        Self {
            function_body_inputs: Vec::new(),
            data_initializers: Vec::new(),
        }
    }
}

/// Object containing the standalone runtime information. To be passed after creation as argument
/// to `cton_wasm::translatemodule`.
pub struct ModuleEnvironment<'data, 'module> {
    /// Compilation setting flags.
    pub flags: &'module settings::Flags,

    /// Module information.
    pub module: &'module mut Module,

    /// References to information to be decoded later.
    pub lazy: LazyContents<'data>,
}

impl<'data, 'module> ModuleEnvironment<'data, 'module> {
    /// Allocates the runtime data structures with the given isa.
    pub fn new(flags: &'module settings::Flags, module: &'module mut Module) -> Self {
        Self {
            flags,
            module,
            lazy: LazyContents::new(),
        }
    }

    fn func_env(&self) -> FuncEnvironment {
        FuncEnvironment::new(&self.flags, &self.module)
    }

    fn native_pointer(&self) -> ir::Type {
        use cton_wasm::FuncEnvironment;
        self.func_env().native_pointer()
    }

    /// Declare that translation of the module is complete. This consumes the
    /// `ModuleEnvironment` with its mutable reference to the `Module` and
    /// produces a `ModuleTranslation` with an immutable reference to the
    /// `Module`.
    pub fn finish_translation(self) -> ModuleTranslation<'data, 'module> {
        ModuleTranslation {
            flags: self.flags,
            module: self.module,
            lazy: self.lazy,
        }
    }
}

/// The FuncEnvironment implementation for use by the `ModuleEnvironment`.
pub struct FuncEnvironment<'module_environment> {
    /// Compilation setting flags.
    settings_flags: &'module_environment settings::Flags,

    /// The module-level environment which this function-level environment belongs to.
    pub module: &'module_environment Module,

    /// The Cretonne global holding the base address of the memories vector.
    pub memories_base: Option<ir::GlobalVar>,

    /// The Cretonne global holding the base address of the globals vector.
    pub globals_base: Option<ir::GlobalVar>,

    /// The external function declaration for implementing wasm's `current_memory`.
    pub current_memory_extfunc: Option<FuncRef>,

    /// The external function declaration for implementing wasm's `grow_memory`.
    pub grow_memory_extfunc: Option<FuncRef>,
}

impl<'module_environment> FuncEnvironment<'module_environment> {
    fn new(
        flags: &'module_environment settings::Flags,
        module: &'module_environment Module,
    ) -> Self {
        Self {
            settings_flags: flags,
            module,
            memories_base: None,
            globals_base: None,
            current_memory_extfunc: None,
            grow_memory_extfunc: None,
        }
    }

    /// Transform the call argument list in preparation for making a call.
    fn get_real_call_args(func: &Function, call_args: &[ir::Value]) -> Vec<ir::Value> {
        let mut real_call_args = Vec::with_capacity(call_args.len() + 1);
        real_call_args.extend_from_slice(call_args);
        real_call_args.push(func.special_param(ArgumentPurpose::VMContext).unwrap());
        real_call_args
    }

    fn ptr_size(&self) -> usize {
        if self.settings_flags.is_64bit() { 8 } else { 4 }
    }
}

impl<'module_environment> cton_wasm::FuncEnvironment for FuncEnvironment<'module_environment> {
    fn flags(&self) -> &settings::Flags {
        &self.settings_flags
    }

    fn make_global(&mut self, func: &mut ir::Function, index: GlobalIndex) -> GlobalValue {
        let ptr_size = self.ptr_size();
        let globals_base = self.globals_base.unwrap_or_else(|| {
            let offset = 0 * ptr_size;
            let offset32 = offset as i32;
            debug_assert_eq!(offset32 as usize, offset);
            let new_base = func.create_global_var(
                ir::GlobalVarData::VmCtx { offset: Offset32::new(offset32) },
            );
            self.globals_base = Some(new_base);
            new_base
        });
        let offset = index as usize * 8;
        let offset32 = offset as i32;
        debug_assert_eq!(offset32 as usize, offset);
        let gv = func.create_global_var(ir::GlobalVarData::Deref {
            base: globals_base,
            offset: Offset32::new(offset32),
        });
        GlobalValue::Memory {
            gv,
            ty: self.module.globals[index].ty,
        }
    }

    fn make_heap(&mut self, func: &mut ir::Function, index: MemoryIndex) -> ir::Heap {
        let ptr_size = self.ptr_size();
        let memories_base = self.memories_base.unwrap_or_else(|| {
            let new_base = func.create_global_var(ir::GlobalVarData::VmCtx {
                offset: Offset32::new(ptr_size as i32),
            });
            self.globals_base = Some(new_base);
            new_base
        });
        let offset = index as usize * ptr_size;
        let offset32 = offset as i32;
        debug_assert_eq!(offset32 as usize, offset);
        let heap_base = func.create_global_var(ir::GlobalVarData::Deref {
            base: memories_base,
            offset: Offset32::new(offset32),
        });
        let h = func.create_heap(ir::HeapData {
            base: ir::HeapBase::GlobalVar(heap_base),
            min_size: 0.into(),
            guard_size: 0x8000_0000.into(),
            style: ir::HeapStyle::Static { bound: 0x1_0000_0000.into() },
        });
        h
    }

    fn make_indirect_sig(&mut self, func: &mut ir::Function, index: SignatureIndex) -> ir::SigRef {
        func.import_signature(self.module.signatures[index].clone())
    }

    fn make_direct_func(&mut self, func: &mut ir::Function, index: FunctionIndex) -> ir::FuncRef {
        let sigidx = self.module.functions[index];
        let signature = func.import_signature(self.module.signatures[sigidx].clone());
        let name = get_func_name(index);
        func.import_function(ir::ExtFuncData { name, signature })
    }

    fn translate_call_indirect(
        &mut self,
        mut pos: FuncCursor,
        table_index: TableIndex,
        _sig_index: SignatureIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> ir::Inst {
        // TODO: Cretonne's call_indirect doesn't implement bounds checking
        // or signature checking, so we need to implement it ourselves.
        debug_assert_eq!(table_index, 0, "non-default tables not supported yet");
        let real_call_args = FuncEnvironment::get_real_call_args(pos.func, call_args);
        pos.ins().call_indirect(sig_ref, callee, &real_call_args)
    }

    fn translate_call(
        &mut self,
        mut pos: FuncCursor,
        _callee_index: FunctionIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> ir::Inst {
        let real_call_args = FuncEnvironment::get_real_call_args(pos.func, call_args);
        pos.ins().call(callee, &real_call_args)
    }

    fn translate_grow_memory(
        &mut self,
        mut pos: FuncCursor,
        index: MemoryIndex,
        _heap: ir::Heap,
        val: ir::Value,
    ) -> ir::Value {
        debug_assert_eq!(index, 0, "non-default memories not supported yet");
        let grow_mem_func = self.grow_memory_extfunc.unwrap_or_else(|| {
            let sig_ref = pos.func.import_signature(Signature {
                call_conv: CallConv::Native,
                argument_bytes: None,
                params: vec![AbiParam::new(I32)],
                returns: vec![AbiParam::new(I32)],
            });
            // FIXME: Use a real ExternalName system.
            pos.func.import_function(ExtFuncData {
                name: ExternalName::testcase("grow_memory"),
                signature: sig_ref,
            })
        });
        self.grow_memory_extfunc = Some(grow_mem_func);
        let call_inst = pos.ins().call(grow_mem_func, &[val]);
        *pos.func.dfg.inst_results(call_inst).first().unwrap()
    }

    fn translate_current_memory(
        &mut self,
        mut pos: FuncCursor,
        index: MemoryIndex,
        _heap: ir::Heap,
    ) -> ir::Value {
        debug_assert_eq!(index, 0, "non-default memories not supported yet");
        let cur_mem_func = self.current_memory_extfunc.unwrap_or_else(|| {
            let sig_ref = pos.func.import_signature(Signature {
                call_conv: CallConv::Native,
                argument_bytes: None,
                params: Vec::new(),
                returns: vec![AbiParam::new(I32)],
            });
            // FIXME: Use a real ExternalName system.
            pos.func.import_function(ExtFuncData {
                name: ExternalName::testcase("current_memory"),
                signature: sig_ref,
            })
        });
        self.current_memory_extfunc = Some(cur_mem_func);
        let call_inst = pos.ins().call(cur_mem_func, &[]);
        *pos.func.dfg.inst_results(call_inst).first().unwrap()
    }
}

/// This trait is useful for
/// `cton_wasm::translatemodule` because it
/// tells how to translate runtime-dependent wasm instructions. These functions should not be
/// called by the user.
impl<'data, 'module> cton_wasm::ModuleEnvironment<'data> for ModuleEnvironment<'data, 'module> {
    fn get_func_name(&self, func_index: FunctionIndex) -> cretonne::ir::ExternalName {
        get_func_name(func_index)
    }

    fn declare_signature(&mut self, sig: &ir::Signature) {
        let mut sig = sig.clone();
        sig.params.push(AbiParam {
            value_type: self.native_pointer(),
            purpose: ArgumentPurpose::VMContext,
            extension: ArgumentExtension::None,
            location: ArgumentLoc::Unassigned,
        });
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

        self.module.imported_funcs.push((
            String::from(module),
            String::from(field),
        ));
    }

    fn get_num_func_imports(&self) -> usize {
        self.module.imported_funcs.len()
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) {
        self.module.functions.push(sig_index);
    }

    fn get_func_type(&self, func_index: FunctionIndex) -> SignatureIndex {
        self.module.functions[func_index]
    }

    fn declare_global(&mut self, global: Global) {
        self.module.globals.push(global);
    }

    fn get_global(&self, global_index: GlobalIndex) -> &cton_wasm::Global {
        &self.module.globals[global_index]
    }

    fn declare_table(&mut self, table: Table) {
        self.module.tables.push(table);
    }

    fn declare_table_elements(
        &mut self,
        table_index: TableIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        elements: Vec<FunctionIndex>,
    ) {
        debug_assert!(base.is_none(), "global-value offsets not supported yet");
        self.module.table_elements.push(module::TableElements {
            table_index,
            base,
            offset,
            elements,
        });
    }

    fn declare_memory(&mut self, memory: Memory) {
        self.module.memories.push(memory);
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

    fn declare_func_export(&mut self, func_index: FunctionIndex, name: &str) {
        self.module.exports.insert(
            String::from(name),
            module::Export::Function(func_index),
        );
    }

    fn declare_table_export(&mut self, table_index: TableIndex, name: &str) {
        self.module.exports.insert(
            String::from(name),
            module::Export::Table(table_index),
        );
    }

    fn declare_memory_export(&mut self, memory_index: MemoryIndex, name: &str) {
        self.module.exports.insert(
            String::from(name),
            module::Export::Memory(memory_index),
        );
    }

    fn declare_global_export(&mut self, global_index: GlobalIndex, name: &str) {
        self.module.exports.insert(
            String::from(name),
            module::Export::Global(global_index),
        );
    }

    fn declare_start_func(&mut self, func_index: FunctionIndex) {
        debug_assert!(self.module.start_func.is_none());
        self.module.start_func = Some(func_index);
    }

    fn define_function_body(&mut self, body_bytes: &'data [u8]) -> Result<(), String> {
        self.lazy.function_body_inputs.push(body_bytes);
        Ok(())
    }
}

/// A record of a relocation to perform.
#[derive(Debug)]
pub struct Relocation {
    /// The relocation code.
    pub reloc: binemit::Reloc,
    /// The function index.
    pub func_index: FunctionIndex,
    /// The offset where to apply the relocation.
    pub offset: binemit::CodeOffset,
    /// The addend to add to the relocation value.
    pub addend: binemit::Addend,
}

/// Relocations to apply to function bodies.
pub type Relocations = Vec<Vec<Relocation>>;

/// The result of translating via `ModuleEnvironment`.
pub struct ModuleTranslation<'data, 'module> {
    /// Compilation setting flags.
    pub flags: &'module settings::Flags,

    /// Module information.
    pub module: &'module Module,

    /// Pointers into the raw data buffer.
    pub lazy: LazyContents<'data>,
}

/// Convenience functions for the user to be called after execution for debug purposes.
impl<'data, 'module> ModuleTranslation<'data, 'module> {
    fn func_env(&self) -> FuncEnvironment {
        FuncEnvironment::new(&self.flags, &self.module)
    }

    /// Compile the module, producing a compilation result with associated
    /// relocations.
    pub fn compile(
        &self,
        isa: &isa::TargetIsa,
    ) -> Result<(Compilation<'module>, Relocations), String> {
        let mut functions = Vec::new();
        let mut relocations = Vec::new();
        for (func_index, input) in self.lazy.function_body_inputs.iter().enumerate() {
            let mut context = cretonne::Context::new();
            context.func.name = get_func_name(func_index);
            context.func.signature = self.module.signatures[self.module.functions[func_index]]
                .clone();

            let mut trans = FuncTranslator::new();
            let reader = wasmparser::BinaryReader::new(input);
            trans
                .translate_from_reader(reader, &mut context.func, &mut self.func_env())
                .map_err(|e| String::from(e.description()))?;

            let code_size = context.compile(isa).map_err(
                |e| String::from(e.description()),
            )? as usize;
            let mut code_buf: Vec<u8> = Vec::with_capacity(code_size as usize);
            let mut reloc_sink = RelocSink::new(&context.func);
            code_buf.resize(code_size, 0);
            context.emit_to_memory(code_buf.as_mut_ptr(), &mut reloc_sink, isa);
            functions.push(code_buf);
            relocations.push(reloc_sink.func_relocs);
        }
        Ok((Compilation::new(self.module, functions), relocations))
    }
}
