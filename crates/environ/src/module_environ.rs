use crate::func_environ::FuncEnvironment;
use crate::module::{Export, MemoryPlan, Module, TableElements, TablePlan};
use crate::tunables::Tunables;
use cranelift_codegen::ir;
use cranelift_codegen::ir::{AbiParam, ArgumentPurpose};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{
    self, translate_module, DefinedFuncIndex, FuncIndex, Global, GlobalIndex, Memory, MemoryIndex,
    ModuleTranslationState, SignatureIndex, Table, TableIndex, TargetEnvironment, WasmResult,
};
use std::convert::TryFrom;

/// Contains function data: byte code and its offset in the module.
#[derive(Hash)]
pub struct FunctionBodyData<'a> {
    /// Body byte code.
    pub data: &'a [u8],

    /// Body offset in the module file.
    pub module_offset: usize,
}

/// The result of translating via `ModuleEnvironment`. Function bodies are not
/// yet translated, and data initializers have not yet been copied out of the
/// original buffer.
pub struct ModuleTranslation<'data> {
    /// Compilation setting flags.
    pub target_config: TargetFrontendConfig,

    /// Module information.
    pub module: Module,

    /// References to the function bodies.
    pub function_body_inputs: PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,

    /// References to the data initializers.
    pub data_initializers: Vec<DataInitializer<'data>>,

    /// Tunable parameters.
    pub tunables: Tunables,

    /// The decoded Wasm types for the module.
    pub module_translation: Option<ModuleTranslationState>,
}

impl<'data> ModuleTranslation<'data> {
    /// Return a new `FuncEnvironment` for translating a function.
    pub fn func_env(&self) -> FuncEnvironment<'_> {
        FuncEnvironment::new(self.target_config, &self.module)
    }
}

/// Object containing the standalone environment information.
pub struct ModuleEnvironment<'data> {
    /// The result to be filled in.
    result: ModuleTranslation<'data>,
    imports: u32,
}

impl<'data> ModuleEnvironment<'data> {
    /// Allocates the environment data structures.
    pub fn new(target_config: TargetFrontendConfig, tunables: Tunables) -> Self {
        Self {
            result: ModuleTranslation {
                target_config,
                module: Module::new(),
                function_body_inputs: PrimaryMap::new(),
                data_initializers: Vec::new(),
                tunables,
                module_translation: None,
            },
            imports: 0,
        }
    }

    fn pointer_type(&self) -> ir::Type {
        self.result.target_config.pointer_type()
    }

    /// Translate a wasm module using this environment. This consumes the
    /// `ModuleEnvironment` and produces a `ModuleTranslation`.
    pub fn translate(mut self, data: &'data [u8]) -> WasmResult<ModuleTranslation<'data>> {
        assert!(self.result.module_translation.is_none());
        let module_translation = translate_module(data, &mut self)?;
        self.result.module_translation = Some(module_translation);
        Ok(self.result)
    }
}

impl<'data> TargetEnvironment for ModuleEnvironment<'data> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.result.target_config
    }
}

/// This trait is useful for `translate_module` because it tells how to translate
/// environment-dependent wasm instructions. These functions should not be called by the user.
impl<'data> cranelift_wasm::ModuleEnvironment<'data> for ModuleEnvironment<'data> {
    fn reserve_signatures(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .signatures
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_signature(&mut self, sig: ir::Signature) -> WasmResult<()> {
        let sig = translate_signature(sig, self.pointer_type());
        // TODO: Deduplicate signatures.
        self.result.module.signatures.push(sig);
        Ok(())
    }

    fn declare_func_import(
        &mut self,
        sig_index: SignatureIndex,
        module: &str,
        field: &str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.functions.len(),
            self.result.module.imported_funcs.len(),
            "Imported functions must be declared first"
        );
        self.result.module.functions.push(sig_index);

        self.result.module.imported_funcs.push((
            String::from(module),
            String::from(field),
            self.imports,
        ));
        self.imports += 1;
        Ok(())
    }

    fn declare_table_import(&mut self, table: Table, module: &str, field: &str) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.table_plans.len(),
            self.result.module.imported_tables.len(),
            "Imported tables must be declared first"
        );
        let plan = TablePlan::for_table(table, &self.result.tunables);
        self.result.module.table_plans.push(plan);

        self.result.module.imported_tables.push((
            String::from(module),
            String::from(field),
            self.imports,
        ));
        self.imports += 1;
        Ok(())
    }

    fn declare_memory_import(
        &mut self,
        memory: Memory,
        module: &str,
        field: &str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.memory_plans.len(),
            self.result.module.imported_memories.len(),
            "Imported memories must be declared first"
        );
        let plan = MemoryPlan::for_memory(memory, &self.result.tunables);
        self.result.module.memory_plans.push(plan);

        self.result.module.imported_memories.push((
            String::from(module),
            String::from(field),
            self.imports,
        ));
        self.imports += 1;
        Ok(())
    }

    fn declare_global_import(
        &mut self,
        global: Global,
        module: &str,
        field: &str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.globals.len(),
            self.result.module.imported_globals.len(),
            "Imported globals must be declared first"
        );
        self.result.module.globals.push(global);

        self.result.module.imported_globals.push((
            String::from(module),
            String::from(field),
            self.imports,
        ));
        self.imports += 1;
        Ok(())
    }

    fn finish_imports(&mut self) -> WasmResult<()> {
        self.result.module.imported_funcs.shrink_to_fit();
        self.result.module.imported_tables.shrink_to_fit();
        self.result.module.imported_memories.shrink_to_fit();
        self.result.module.imported_globals.shrink_to_fit();
        Ok(())
    }

    fn reserve_func_types(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .functions
            .reserve_exact(usize::try_from(num).unwrap());
        self.result
            .function_body_inputs
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) -> WasmResult<()> {
        self.result.module.functions.push(sig_index);
        Ok(())
    }

    fn reserve_tables(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .table_plans
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_table(&mut self, table: Table) -> WasmResult<()> {
        let plan = TablePlan::for_table(table, &self.result.tunables);
        self.result.module.table_plans.push(plan);
        Ok(())
    }

    fn reserve_memories(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .memory_plans
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_memory(&mut self, memory: Memory) -> WasmResult<()> {
        let plan = MemoryPlan::for_memory(memory, &self.result.tunables);
        self.result.module.memory_plans.push(plan);
        Ok(())
    }

    fn reserve_globals(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .globals
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_global(&mut self, global: Global) -> WasmResult<()> {
        self.result.module.globals.push(global);
        Ok(())
    }

    fn reserve_exports(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .exports
            .reserve(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_func_export(&mut self, func_index: FuncIndex, name: &str) -> WasmResult<()> {
        self.result
            .module
            .exports
            .insert(String::from(name), Export::Function(func_index));
        Ok(())
    }

    fn declare_table_export(&mut self, table_index: TableIndex, name: &str) -> WasmResult<()> {
        self.result
            .module
            .exports
            .insert(String::from(name), Export::Table(table_index));
        Ok(())
    }

    fn declare_memory_export(&mut self, memory_index: MemoryIndex, name: &str) -> WasmResult<()> {
        self.result
            .module
            .exports
            .insert(String::from(name), Export::Memory(memory_index));
        Ok(())
    }

    fn declare_global_export(&mut self, global_index: GlobalIndex, name: &str) -> WasmResult<()> {
        self.result
            .module
            .exports
            .insert(String::from(name), Export::Global(global_index));
        Ok(())
    }

    fn declare_start_func(&mut self, func_index: FuncIndex) -> WasmResult<()> {
        debug_assert!(self.result.module.start_func.is_none());
        self.result.module.start_func = Some(func_index);
        Ok(())
    }

    fn reserve_table_elements(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .table_elements
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_table_elements(
        &mut self,
        table_index: TableIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        elements: Box<[FuncIndex]>,
    ) -> WasmResult<()> {
        self.result.module.table_elements.push(TableElements {
            table_index,
            base,
            offset,
            elements,
        });
        Ok(())
    }

    fn define_function_body(
        &mut self,
        _module_translation: &ModuleTranslationState,
        body_bytes: &'data [u8],
        body_offset: usize,
    ) -> WasmResult<()> {
        self.result.function_body_inputs.push(FunctionBodyData {
            data: body_bytes,
            module_offset: body_offset,
        });
        Ok(())
    }

    fn reserve_data_initializers(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .data_initializers
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_data_initialization(
        &mut self,
        memory_index: MemoryIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        data: &'data [u8],
    ) -> WasmResult<()> {
        self.result.data_initializers.push(DataInitializer {
            location: DataInitializerLocation {
                memory_index,
                base,
                offset,
            },
            data,
        });
        Ok(())
    }
}

/// Add environment-specific function parameters.
pub fn translate_signature(mut sig: ir::Signature, pointer_type: ir::Type) -> ir::Signature {
    // Prepend the vmctx argument.
    sig.params.insert(
        0,
        AbiParam::special(pointer_type, ArgumentPurpose::VMContext),
    );
    sig
}

/// A memory index and offset within that memory where a data initialization
/// should is to be performed.
#[derive(Clone)]
pub struct DataInitializerLocation {
    /// The index of the memory to initialize.
    pub memory_index: MemoryIndex,

    /// Optionally a globalvar base to initialize at.
    pub base: Option<GlobalIndex>,

    /// A constant offset to initialize at.
    pub offset: usize,
}

/// A data initializer for linear memory.
pub struct DataInitializer<'data> {
    /// The location where the initialization is to be performed.
    pub location: DataInitializerLocation,

    /// The initialization data.
    pub data: &'data [u8],
}
