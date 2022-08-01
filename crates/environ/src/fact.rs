//! Wasmtime's Fused Adapter Compiler of Trampolines (FACT)
//!
//! This module contains a compiler which emits trampolines to implement fused
//! adatpers for the component model. A fused adapter is when a core wasm
//! function is lifted from one component instance and then lowered into another
//! component instance. This communication between components is well-defined by
//! the spec and ends up creating what's called a "fused adapter".
//!
//! Adapters are currently implemented with WebAssembly modules. This submodule
//! will generate a core wasm binary which contains the adapters specified
//! during compilation. The actual wasm is then later processed by standard
//! paths in Wasmtime to create native machine code and runtime representations
//! of modules.
//!
//! Note that identification of precisely what goes into an adapter module is
//! not handled in this file, instead that's all done in `translate/adapt.rs`.
//! Otherwise this module is only reponsible for taking a set of adapters and
//! their imports and then generating a core wasm module to implement all of
//! that.

use crate::component::{
    Adapter, AdapterOptions, ComponentTypes, CoreDef, StringEncoding, TypeFuncIndex,
};
use crate::{FuncIndex, GlobalIndex, MemoryIndex};
use std::collections::HashMap;
use std::mem;
use wasm_encoder::*;

mod core_types;
mod signature;
mod trampoline;
mod traps;

/// Representation of an adapter module.
pub struct Module<'a> {
    /// Whether or not debug code is inserted into the adapters themselves.
    debug: bool,
    /// Type information from the creator of this `Module`
    types: &'a ComponentTypes,

    /// Core wasm type section that's incrementally built
    core_types: core_types::CoreTypes,

    /// Core wasm import section which is built as adapters are inserted. Note
    /// that imports here are intern'd to avoid duplicate imports of the same
    /// item.
    core_imports: ImportSection,
    /// Final list of imports that this module ended up using, in the same order
    /// as the imports in the import section.
    imports: Vec<CoreDef>,
    /// Intern'd imports and what index they were assigned.
    imported: HashMap<CoreDef, u32>,

    // Current status of index spaces from the imports generated so far.
    core_funcs: u32,
    core_memories: u32,
    core_globals: u32,

    /// Adapters which will be compiled once they're all registered.
    adapters: Vec<AdapterData>,
}

struct AdapterData {
    /// Export name of this adapter
    name: String,
    /// Options specified during the `canon lift` operation
    lift: Options,
    /// Options specified during the `canon lower` operation
    lower: Options,
    /// The core wasm function that this adapter will be calling (the original
    /// function that was `canon lift`'d)
    callee: FuncIndex,
    /// FIXME(#4185) should be plumbed and handled as part of the new reentrance
    /// rules not yet implemented here.
    called_as_export: bool,
}

struct Options {
    ty: TypeFuncIndex,
    string_encoding: StringEncoding,
    flags: GlobalIndex,
    memory64: bool,
    memory: Option<MemoryIndex>,
    realloc: Option<FuncIndex>,
    post_return: Option<FuncIndex>,
}

enum Context {
    Lift,
    Lower,
}

impl<'a> Module<'a> {
    /// Creates an empty module.
    pub fn new(types: &'a ComponentTypes, debug: bool) -> Module<'a> {
        Module {
            debug,
            types,
            core_types: Default::default(),
            core_imports: Default::default(),
            imported: Default::default(),
            adapters: Default::default(),
            imports: Default::default(),
            core_funcs: 0,
            core_memories: 0,
            core_globals: 0,
        }
    }

    /// Registers a new adapter within this adapter module.
    ///
    /// The `name` provided is the export name of the adapter from the final
    /// module, and `adapter` contains all metadata necessary for compilation.
    pub fn adapt(&mut self, name: &str, adapter: &Adapter) {
        // Import any items required by the various canonical options
        // (memories, reallocs, etc)
        let mut lift = self.import_options(adapter.lift_ty, &adapter.lift_options);
        let lower = self.import_options(adapter.lower_ty, &adapter.lower_options);

        // Lowering options are not allowed to specify post-return as per the
        // current canonical abi specification.
        assert!(adapter.lower_options.post_return.is_none());

        // Import the core wasm function which was lifted using its appropriate
        // signature since the exported function this adapter generates will
        // call the lifted function.
        let signature = self.signature(&lift, Context::Lift);
        let ty = self
            .core_types
            .function(&signature.params, &signature.results);
        let callee = self.import_func("callee", name, ty, adapter.func.clone());

        // Handle post-return specifically here where we have `core_ty` and the
        // results of `core_ty` are the parameters to the post-return function.
        lift.post_return = adapter.lift_options.post_return.as_ref().map(|func| {
            let ty = self.core_types.function(&signature.results, &[]);
            self.import_func("post_return", name, ty, func.clone())
        });

        self.adapters.push(AdapterData {
            name: name.to_string(),
            lift,
            lower,
            callee,
            // FIXME(#4185) should be plumbed and handled as part of the new
            // reentrance rules not yet implemented here.
            called_as_export: true,
        });
    }

    fn import_options(&mut self, ty: TypeFuncIndex, options: &AdapterOptions) -> Options {
        let AdapterOptions {
            instance,
            string_encoding,
            memory,
            memory64,
            realloc,
            post_return: _, // handled above
        } = options;
        let flags = self.import_global(
            "flags",
            &format!("instance{}", instance.as_u32()),
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
            },
            CoreDef::InstanceFlags(*instance),
        );
        let memory = memory.as_ref().map(|memory| {
            self.import_memory(
                "memory",
                "",
                MemoryType {
                    minimum: 0,
                    maximum: None,
                    shared: false,
                    memory64: *memory64,
                },
                memory.clone().into(),
            )
        });
        let realloc = realloc.as_ref().map(|func| {
            let ptr = if *memory64 {
                ValType::I64
            } else {
                ValType::I32
            };
            let ty = self.core_types.function(&[ptr, ptr, ptr, ptr], &[ptr]);
            self.import_func("realloc", "", ty, func.clone())
        });
        Options {
            ty,
            string_encoding: *string_encoding,
            flags,
            memory64: *memory64,
            memory,
            realloc,
            post_return: None,
        }
    }

    fn import_func(&mut self, module: &str, name: &str, ty: u32, def: CoreDef) -> FuncIndex {
        FuncIndex::from_u32(
            self.import(module, name, EntityType::Function(ty), def, |m| {
                &mut m.core_funcs
            }),
        )
    }

    fn import_global(
        &mut self,
        module: &str,
        name: &str,
        ty: GlobalType,
        def: CoreDef,
    ) -> GlobalIndex {
        GlobalIndex::from_u32(self.import(module, name, EntityType::Global(ty), def, |m| {
            &mut m.core_globals
        }))
    }

    fn import_memory(
        &mut self,
        module: &str,
        name: &str,
        ty: MemoryType,
        def: CoreDef,
    ) -> MemoryIndex {
        MemoryIndex::from_u32(self.import(module, name, EntityType::Memory(ty), def, |m| {
            &mut m.core_memories
        }))
    }

    fn import(
        &mut self,
        module: &str,
        name: &str,
        ty: EntityType,
        def: CoreDef,
        new: impl FnOnce(&mut Self) -> &mut u32,
    ) -> u32 {
        if let Some(prev) = self.imported.get(&def) {
            return *prev;
        }
        let cnt = new(self);
        *cnt += 1;
        let ret = *cnt - 1;
        self.core_imports.import(module, name, ty);
        self.imported.insert(def.clone(), ret);
        self.imports.push(def);
        ret
    }

    /// Encodes this module into a WebAssembly binary.
    pub fn encode(&mut self) -> Vec<u8> {
        let mut funcs = FunctionSection::new();
        let mut code = CodeSection::new();
        let mut exports = ExportSection::new();
        let mut traps = traps::TrapSection::default();

        let mut types = mem::take(&mut self.core_types);
        for adapter in self.adapters.iter() {
            let idx = self.core_funcs + funcs.len();
            exports.export(&adapter.name, ExportKind::Func, idx);

            let signature = self.signature(&adapter.lower, Context::Lower);
            let ty = types.function(&signature.params, &signature.results);
            funcs.function(ty);

            let (function, func_traps) = trampoline::compile(self, &mut types, adapter);
            code.raw(&function);
            traps.append(idx, func_traps);
        }
        self.core_types = types;
        let traps = traps.finish();

        let mut result = wasm_encoder::Module::new();
        result.section(&self.core_types.section);
        result.section(&self.core_imports);
        result.section(&funcs);
        result.section(&exports);
        result.section(&code);
        if self.debug {
            result.section(&CustomSection {
                name: "wasmtime-trampoline-traps",
                data: &traps,
            });
        }
        result.finish()
    }

    /// Returns the imports that were used, in order, to create this adapter
    /// module.
    pub fn imports(&self) -> &[CoreDef] {
        &self.imports
    }
}

impl Options {
    fn ptr(&self) -> ValType {
        if self.memory64 {
            ValType::I64
        } else {
            ValType::I32
        }
    }

    fn ptr_size(&self) -> u8 {
        if self.memory64 {
            8
        } else {
            4
        }
    }
}
