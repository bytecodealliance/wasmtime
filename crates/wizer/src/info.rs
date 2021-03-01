use wasm_encoder::SectionId;

use crate::translate;
use std::collections::BTreeMap;
use std::convert::TryFrom;

/// Info that we keep track of on a per module-within-a-module-linking-bundle
/// basis.
///
/// These are created during during our `parse` pass and then used throughout
/// our later passes.
#[derive(Clone)]
pub(crate) struct ModuleInfo<'a> {
    /// This module's id (i.e. its pre-order traversal index).
    pub id: u32,

    /// The raw sections from the original Wasm input.
    pub raw_sections: Vec<wasm_encoder::RawSection<'a>>,

    /// This vector has `n` entries when the module has `n` import sections. The
    /// `i`th entry is a count of how many instance imports are in the `i`th
    /// import section.
    pub instance_import_counts: Vec<u32>,

    /// Types available in this module.
    ///
    /// We keep track of these for determining how many things we need to
    /// re-export for new instantiations and for inner module's aliases.
    pub types: Vec<wasmparser::TypeDef<'a>>,

    /// Imports made by this module.
    pub imports: Vec<wasmparser::Import<'a>>,

    /// Aliases that this module defines.
    pub aliases: Vec<wasmparser::Alias<'a>>,

    /// How many more not-yet-parsed child modules are expected for this module?
    ///
    /// This is only used during parsing.
    pub child_modules_expected: u32,

    /// Directly nested inner modules of this module.
    ///
    /// These entries are populated as we finish instrumenting the inner
    /// modules.
    pub modules: Vec<ModuleInfo<'a>>,

    /// A map from instance indices to each instance's type for all defined,
    /// imported, and aliased instances.
    pub instances: Vec<wasmparser::InstanceType<'a>>,

    /// A map from indices of defined instantiations (as opposed to imported or
    /// aliased instantiations) to the id of the module that was instantiated
    /// and the import arguments.
    pub instantiations: BTreeMap<u32, (u32, Vec<wasmparser::InstanceArg<'a>>)>,

    /// A map from global indices to each global's type for all defined,
    /// imported, and aliased globals.
    pub globals: Vec<wasmparser::GlobalType>,

    /// The index within the global index space where defined globals (as
    /// opposed to imported or aliased) begin.
    ///
    /// If this is `None`, then there are no locally defined globals.
    pub defined_globals_index: Option<u32>,

    /// This module's exports.
    ///
    /// This is used later on, in the rewrite phase, when we are inserting state
    /// instance imports.
    ///
    /// Note that this does *not* include the `__wizer_thing_N` exports that
    /// this instrumentation pass adds.
    pub exports: Vec<wasmparser::Export<'a>>,

    /// A currently-being-encoded module section.
    ///
    /// As we finish instrumenting child modules, we add them here. If we aren't
    /// currently processing this module's children, then this is `None`.
    pub module_section: Option<wasm_encoder::ModuleSection>,

    /// Maps from function index to the function's type index for all functions
    /// defined, imported, and aliased in this module.
    pub functions: Vec<u32>,

    /// Maps from table index to the table's type for all tables defined,
    /// imported, and aliased in this module.
    pub tables: Vec<wasmparser::TableType>,

    /// Maps from memory index to the memory's type for all memories defined,
    /// imported, and aliased in this module.
    pub memories: Vec<wasmparser::MemoryType>,

    /// The index within the memory index space where defined memories (as
    /// opposed to imported or aliased) begin.
    ///
    /// If this is `None`, then there are no locally defined memories.
    pub defined_memories_index: Option<u32>,
}

impl<'a> ModuleInfo<'a> {
    /// Create the `ModuleInfo` for the root of a module-linking bundle.
    pub fn for_root() -> Self {
        Self {
            id: 0,
            raw_sections: vec![],
            instance_import_counts: vec![],
            modules: vec![],
            types: vec![],
            imports: vec![],
            aliases: vec![],
            globals: vec![],
            defined_globals_index: None,
            instances: vec![],
            instantiations: BTreeMap::new(),
            child_modules_expected: 0,
            module_section: None,
            exports: vec![],
            functions: vec![],
            tables: vec![],
            memories: vec![],
            defined_memories_index: None,
        }
    }

    /// Create a new `ModuleInfo` for an inner module.
    pub fn for_inner(id: u32) -> Self {
        Self {
            id,
            raw_sections: vec![],
            instance_import_counts: vec![],
            modules: vec![],
            types: vec![],
            imports: vec![],
            aliases: vec![],
            globals: vec![],
            defined_globals_index: None,
            instances: vec![],
            instantiations: BTreeMap::new(),
            child_modules_expected: 0,
            module_section: None,
            exports: vec![],
            functions: vec![],
            tables: vec![],
            memories: vec![],
            defined_memories_index: None,
        }
    }

    /// Add a new raw section to this module info during parsing.
    pub fn add_raw_section(
        &mut self,
        id: SectionId,
        range: wasmparser::Range,
        full_wasm: &'a [u8],
    ) {
        self.raw_sections.push(wasm_encoder::RawSection {
            id: id as u8,
            data: &full_wasm[range.start..range.end],
        })
    }

    /// Is this the root of the module linking bundle?
    pub fn is_root(&self) -> bool {
        self.id == 0
    }

    /// Define an instance type for this module's exports.
    ///
    /// Returns the index of the type and updates the total count of types in
    /// `num_types`.
    pub fn define_instance_type(
        &self,
        num_types: &mut u32,
        types: &mut wasm_encoder::TypeSection,
    ) -> u32 {
        let ty_index = *num_types;
        types.instance(self.exports.iter().map(|e| {
            let name = e.field;
            let index = usize::try_from(e.index).unwrap();
            let item = match e.kind {
                wasmparser::ExternalKind::Function => {
                    let func_ty = self.functions[index];
                    wasm_encoder::EntityType::Function(func_ty)
                }
                wasmparser::ExternalKind::Table => {
                    let ty = self.tables[index];
                    wasm_encoder::EntityType::Table(translate::table_type(ty))
                }
                wasmparser::ExternalKind::Memory => {
                    let ty = self.memories[index];
                    wasm_encoder::EntityType::Memory(translate::memory_type(ty))
                }
                wasmparser::ExternalKind::Global => {
                    let ty = self.globals[index];
                    wasm_encoder::EntityType::Global(translate::global_type(ty))
                }
                wasmparser::ExternalKind::Instance => wasm_encoder::EntityType::Instance(e.index),
                wasmparser::ExternalKind::Module
                | wasmparser::ExternalKind::Type
                | wasmparser::ExternalKind::Event => unreachable!(),
            };
            (name, item)
        }));
        *num_types += 1;
        ty_index
    }

    /// Construct an instance type for instances of this module.
    pub fn instance_type(&self) -> wasmparser::InstanceType<'a> {
        wasmparser::InstanceType {
            exports: self
                .exports
                .iter()
                .map(|e| {
                    let index = usize::try_from(e.index).unwrap();
                    wasmparser::ExportType {
                        name: e.field,
                        ty: match e.kind {
                            wasmparser::ExternalKind::Function => {
                                let func_ty = self.functions[index];
                                wasmparser::ImportSectionEntryType::Function(func_ty)
                            }
                            wasmparser::ExternalKind::Table => {
                                let ty = self.tables[index];
                                wasmparser::ImportSectionEntryType::Table(ty)
                            }
                            wasmparser::ExternalKind::Memory => {
                                let ty = self.memories[index];
                                wasmparser::ImportSectionEntryType::Memory(ty)
                            }
                            wasmparser::ExternalKind::Global => {
                                let ty = self.globals[index];
                                wasmparser::ImportSectionEntryType::Global(ty)
                            }
                            wasmparser::ExternalKind::Instance => {
                                wasmparser::ImportSectionEntryType::Instance(e.index)
                            }
                            wasmparser::ExternalKind::Module
                            | wasmparser::ExternalKind::Type
                            | wasmparser::ExternalKind::Event => unreachable!(),
                        },
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    /// Get an export from the `n`th instance by name.
    pub fn instance_export(
        &self,
        instance: u32,
        name: &str,
    ) -> Option<wasmparser::ImportSectionEntryType> {
        let instance = usize::try_from(instance).unwrap();
        let instance = &self.instances[instance];
        instance
            .exports
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.ty)
    }

    /// Do a pre-order traversal over this module tree.
    pub fn pre_order<'b, F>(&'b self, mut f: F)
    where
        F: FnMut(&'b ModuleInfo<'a>),
    {
        let mut stack = vec![self];
        while let Some(info) = stack.pop() {
            f(info);
            stack.extend(info.modules.iter().rev());
        }
    }

    /// The number of defined memories in this module.
    pub fn defined_memories_len(&self) -> usize {
        self.defined_memories_index.map_or(0, |n| {
            let n = usize::try_from(n).unwrap();
            assert!(self.memories.len() > n);
            self.memories.len() - n
        })
    }

    /// Iterate over the defined memories in this module.
    pub fn defined_memories<'b>(&'b self) -> impl Iterator<Item = wasmparser::MemoryType> + 'b {
        self.memories
            .iter()
            .skip(
                self.defined_memories_index
                    .map_or(self.memories.len(), |i| usize::try_from(i).unwrap()),
            )
            .copied()
    }

    /// The number of defined globals in this module.
    pub fn defined_globals_len(&self) -> usize {
        self.defined_globals_index.map_or(0, |n| {
            let n = usize::try_from(n).unwrap();
            assert!(self.globals.len() > n);
            self.globals.len() - n
        })
    }

    /// Iterate over the defined globals in this module.
    pub fn defined_globals<'b>(&'b self) -> impl Iterator<Item = wasmparser::GlobalType> + 'b {
        self.globals
            .iter()
            .skip(
                self.defined_globals_index
                    .map_or(self.globals.len(), |i| usize::try_from(i).unwrap()),
            )
            .copied()
    }

    /// Iterate over the initial sections in this Wasm module.
    pub fn initial_sections<'b>(
        &'b self,
    ) -> impl Iterator<Item = &'b wasm_encoder::RawSection> + 'b {
        self.raw_sections
            .iter()
            .filter(|s| s.id != SectionId::Custom.into())
            .take_while(|s| match s.id {
                x if x == SectionId::Type.into() => true,
                x if x == SectionId::Import.into() => true,
                x if x == SectionId::Alias.into() => true,
                x if x == SectionId::Module.into() => true,
                x if x == SectionId::Instance.into() => true,
                _ => false,
            })
    }
}
