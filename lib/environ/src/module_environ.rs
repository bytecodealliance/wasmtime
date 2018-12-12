use cranelift_codegen::ir;
use cranelift_codegen::ir::{AbiParam, ArgumentPurpose};
use cranelift_codegen::isa;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{
    self, translate_module, DefinedFuncIndex, FuncIndex, Global, GlobalIndex, Memory, MemoryIndex,
    SignatureIndex, Table, TableIndex, WasmResult,
};
use func_environ::FuncEnvironment;
use module::{Export, MemoryPlan, Module, TableElements, TablePlan};
use std::clone::Clone;
use std::string::String;
use std::vec::Vec;
use tunables::Tunables;

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

/// This trait is useful for `translate_module` because it tells how to translate
/// enironment-dependent wasm instructions. These functions should not be called by the user.
impl<'data, 'module> cranelift_wasm::ModuleEnvironment<'data>
    for ModuleEnvironment<'data, 'module>
{
    fn target_config(&self) -> isa::TargetFrontendConfig {
        self.isa.frontend_config()
    }

    fn declare_signature(&mut self, sig: &ir::Signature) {
        let sig = translate_signature(sig.clone(), self.pointer_type());
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

    fn declare_global_import(&mut self, global: Global, module: &str, field: &str) {
        debug_assert_eq!(
            self.module.globals.len(),
            self.module.imported_globals.len(),
            "Imported globals must be declared first"
        );
        self.module.globals.push(global);

        self.module
            .imported_globals
            .push((String::from(module), String::from(field)));
    }

    fn declare_global(&mut self, global: Global) {
        self.module.globals.push(global);
    }

    fn get_global(&self, global_index: GlobalIndex) -> &Global {
        &self.module.globals[global_index]
    }

    fn declare_table_import(&mut self, table: Table, module: &str, field: &str) {
        debug_assert_eq!(
            self.module.table_plans.len(),
            self.module.imported_tables.len(),
            "Imported tables must be declared first"
        );
        let plan = TablePlan::for_table(table, &self.tunables);
        self.module.table_plans.push(plan);

        self.module
            .imported_tables
            .push((String::from(module), String::from(field)));
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
        self.module.table_elements.push(TableElements {
            table_index,
            base,
            offset,
            elements,
        });
    }

    fn declare_memory_import(&mut self, memory: Memory, module: &str, field: &str) {
        debug_assert_eq!(
            self.module.memory_plans.len(),
            self.module.imported_memories.len(),
            "Imported memories must be declared first"
        );
        let plan = MemoryPlan::for_memory(memory, &self.tunables);
        self.module.memory_plans.push(plan);

        self.module
            .imported_memories
            .push((String::from(module), String::from(field)));
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

/// Add environment-specific function parameters.
pub fn translate_signature(mut sig: ir::Signature, pointer_type: ir::Type) -> ir::Signature {
    sig.params
        .push(AbiParam::special(pointer_type, ArgumentPurpose::VMContext));
    sig
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

/// References to the input wasm data buffer to be decoded and processed later,
/// separately from the main module translation.
pub struct LazyContents<'data> {
    /// References to the function bodies.
    pub function_body_inputs: PrimaryMap<DefinedFuncIndex, &'data [u8]>,

    /// References to the data initializers.
    pub data_initializers: Vec<DataInitializer<'data>>,
}

impl<'data> LazyContents<'data> {
    pub fn new() -> Self {
        Self {
            function_body_inputs: PrimaryMap::new(),
            data_initializers: Vec::new(),
        }
    }
}
