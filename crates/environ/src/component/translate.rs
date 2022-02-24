use crate::component::*;
use crate::{EntityIndex, ModuleEnvironment, ModuleTranslation, PrimaryMap, Tunables};
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::mem;
use wasmparser::{Chunk, Encoding, Parser, Payload, Validator};

/// Structure used to translate a component and parse it.
pub struct Translator<'a, 'data> {
    result: Translation<'data>,
    validator: &'a mut Validator,
    types: &'a mut ComponentTypesBuilder,
    tunables: &'a Tunables,
    parsers: Vec<Parser>,
    parser: Parser,
}

/// Result of translation of a component to contain all type information and
/// metadata about how to run the component.
#[derive(Default)]
pub struct Translation<'data> {
    /// Final type of the component, intended to be persisted all the way to
    /// runtime.
    pub component: Component,

    /// List of "upvars" or closed over modules that `Component` would refer
    /// to. This contains the core wasm results of translation and the indices
    /// are referred to within types in `Component`.
    pub upvars: PrimaryMap<ModuleUpvarIndex, ModuleTranslation<'data>>,

    // Index spaces which are built-up during translation but do not persist to
    // runtime. These are used to understand the structure of the component and
    // where items come from but at this time these index spaces and their
    // definitions are not required at runtime as they're effectively "erased"
    // at the moment. This will probably change as more of the component model
    // is implemented.
    //
    /// Modules and how they're defined (either closed-over or imported)
    modules: PrimaryMap<ModuleIndex, ModuleDef>,

    /// Instances and how they're defined, either as instantiations of modules
    /// or "synthetically created" as a bag of named items from our other index
    /// spaces.
    instances: PrimaryMap<InstanceIndex, InstanceDef<'data>>,

    /// Both core wasm and component functions, and how they're defined.
    funcs: PrimaryMap<FuncIndex, Func<'data>>,

    /// Core wasm globals, always sourced from a previously module instance.
    globals: PrimaryMap<GlobalIndex, CoreSource<'data>>,

    /// Core wasm memories, always sourced from a previously module instance.
    memories: PrimaryMap<MemoryIndex, CoreSource<'data>>,

    /// Core wasm tables, always sourced from a previously module instance.
    tables: PrimaryMap<TableIndex, CoreSource<'data>>,
}

/// How a module is defined within a component.
#[derive(Debug)]
enum ModuleDef {
    /// This module is defined as an "upvar" or a closed over variable
    /// implicitly available for the component.
    ///
    /// This means that the module was either defined within the component or a
    /// module was aliased into this component which was known defined in the
    /// parent component.
    Upvar(ModuleUpvarIndex),

    /// This module is defined as an import to the current component, so
    /// nothing is known about it except for its type. The `import_index`
    /// provided here indexes into the `Component`'s import list.
    Import {
        type_idx: ModuleTypeIndex,
        import_index: usize,
    },
}

#[derive(Debug)]
enum InstanceDef<'data> {
    Module {
        instance: RuntimeInstanceIndex,
        module: ModuleIndex,
    },
    ModuleSynthetic(HashMap<&'data str, EntityIndex>),
}

/// Source of truth for where a core wasm item comes from.
#[derive(Clone)]
enum Func<'data> {
    Lifted {
        ty: FuncTypeIndex,
        func: CoreSource<'data>,
        options: CanonicalOptions,
    },
    Core(CoreSource<'data>),
}

/// Source of truth for where a core wasm item comes from.
#[derive(Clone)]
enum CoreSource<'data> {
    /// This item comes from an indexed entity within an instance.
    ///
    /// This is only available when the instance is statically known to be
    /// defined within the original component itself so we know the exact
    /// index.
    Index(RuntimeInstanceIndex, EntityIndex),

    /// This item comes from an named entity within an instance.
    ///
    /// This must be used for instances of imported modules because we
    /// otherwise don't know the internal structure of the module and which
    /// index is being exported.
    Export(RuntimeInstanceIndex, &'data str),
}

enum Action {
    KeepGoing,
    Skip(usize),
    Done,
}

impl<'a, 'data> Translator<'a, 'data> {
    /// Creates a new translation state ready to translate a component.
    pub fn new(
        tunables: &'a Tunables,
        validator: &'a mut Validator,
        types: &'a mut ComponentTypesBuilder,
    ) -> Self {
        Self {
            result: Translation::default(),
            tunables,
            validator,
            types,
            parser: Parser::new(0),
            parsers: Vec::new(),
        }
    }

    /// Translates the binary `component`.
    ///
    /// This is the workhorse of compilation which will parse all of
    /// `component` and create type information for Wasmtime and such. The
    /// `component` does not have to be valid and it will be validated during
    /// compilation.
    pub fn translate(mut self, component: &'data [u8]) -> Result<Translation<'data>> {
        let mut remaining = component;
        loop {
            let payload = match self.parser.parse(remaining, true)? {
                Chunk::Parsed { payload, consumed } => {
                    remaining = &remaining[consumed..];
                    payload
                }
                Chunk::NeedMoreData(_) => unreachable!(),
            };

            match self.translate_payload(payload, component)? {
                Action::KeepGoing => {}
                Action::Skip(n) => remaining = &remaining[n..],
                Action::Done => break,
            }
        }
        Ok(self.result)
    }

    fn translate_payload(
        &mut self,
        payload: Payload<'data>,
        component: &'data [u8],
    ) -> Result<Action> {
        match payload {
            Payload::Version {
                num,
                encoding,
                range,
            } => {
                self.validator.version(num, encoding, &range)?;

                match encoding {
                    Encoding::Component => {}
                    Encoding::Module => {
                        bail!("attempted to parse a wasm module with a component parser");
                    }
                }

                // Push a new scope for component types so outer aliases know
                // that the 0th level is this new component.
                self.types.push_component_types_scope();
            }

            Payload::End(offset) => {
                self.validator.end(offset)?;

                // When leaving a module be sure to pop the types scope to
                // ensure that when we go back to the previous module outer
                // type alias indices work correctly again.
                self.types.pop_component_types_scope();

                match self.parsers.pop() {
                    Some(p) => self.parser = p,
                    None => return Ok(Action::Done),
                }
            }

            // When we see a type section the types are validated and then
            // translated into Wasmtime's representation. Each active type
            // definition is recorded in the `ComponentTypesBuilder` tables, or
            // this component's active scope.
            //
            // Note that the push/pop of the component types scope happens above
            // in `Version` and `End` since multiple type sections can appear
            // within a component.
            Payload::ComponentTypeSection(s) => {
                self.validator.component_type_section(&s)?;
                for ty in s {
                    let ty = self.types.component_type_def(&ty?)?;
                    self.types.push_component_typedef(ty);
                }
            }

            Payload::ComponentImportSection(s) => {
                self.validator.component_import_section(&s)?;
                for import in s {
                    let import = import?;
                    let ty = TypeIndex::from_u32(import.ty);
                    let ty = self.types.component_outer_type(0, ty);
                    let (import_index, prev) = self
                        .result
                        .component
                        .imports
                        .insert_full(import.name.to_string(), ty);
                    assert!(prev.is_none());
                    match ty {
                        TypeDef::Module(type_idx) => {
                            self.result.modules.push(ModuleDef::Import {
                                type_idx,
                                import_index,
                            });
                        }
                        TypeDef::Component(_) => {
                            unimplemented!("component imports");
                        }
                        TypeDef::ComponentInstance(_) => {
                            unimplemented!("component instance imports");
                        }
                        TypeDef::Func(_) => {
                            unimplemented!("function imports");
                        }
                        TypeDef::Interface(_) => {
                            unimplemented!("interface type imports");
                        }
                    }
                }
            }

            Payload::ComponentFunctionSection(s) => {
                self.validator.component_function_section(&s)?;
                for func in s {
                    let func = match func? {
                        wasmparser::ComponentFunction::Lift {
                            type_index,
                            func_index,
                            options,
                        } => {
                            let ty = TypeIndex::from_u32(type_index);
                            let func = FuncIndex::from_u32(func_index);
                            self.lift_function(ty, func, &options)
                        }
                        wasmparser::ComponentFunction::Lower {
                            func_index,
                            options,
                        } => {
                            drop((func_index, options));
                            unimplemented!("lowered functions");
                        }
                    };
                    self.result.funcs.push(func);
                }
            }

            // Core wasm modules are translated inline directly here with the
            // `ModuleEnvironment` from core wasm compilation. This will return
            // to the caller the size of the module so it knows how many bytes
            // of the input are skipped.
            Payload::ModuleSection { parser, range } => {
                self.validator.module_section(&range)?;
                let translation = ModuleEnvironment::new(
                    self.tunables,
                    self.validator,
                    self.types.module_types_builder(),
                )
                .translate(parser, &component[range.start..range.end])?;
                let upvar_idx = self.result.upvars.push(translation);
                self.result.modules.push(ModuleDef::Upvar(upvar_idx));
                return Ok(Action::Skip(range.end - range.start));
            }

            Payload::ComponentSection { parser, range } => {
                self.validator.component_section(&range)?;
                let old_parser = mem::replace(&mut self.parser, parser);
                self.parsers.push(old_parser);
                unimplemented!("component section");
            }

            Payload::InstanceSection(s) => {
                self.validator.instance_section(&s)?;
                for instance in s {
                    let instance = match instance? {
                        wasmparser::Instance::Module { index, args } => {
                            self.module_instance(ModuleIndex::from_u32(index), &args)
                        }
                        wasmparser::Instance::ModuleFromExports(exports) => {
                            self.module_instance_from_exports(&exports)
                        }
                        wasmparser::Instance::Component { index, args } => {
                            drop((index, args));
                            unimplemented!("instantiating a component");
                        }
                        wasmparser::Instance::ComponentFromExports(exports) => {
                            drop(exports);
                            unimplemented!("instantiating a component");
                        }
                    };
                    self.result.instances.push(instance);
                }
            }

            Payload::ComponentExportSection(s) => {
                self.validator.component_export_section(&s)?;
                for export in s {
                    self.export(&export?);
                }
            }

            Payload::ComponentStartSection(s) => {
                self.validator.component_start_section(&s)?;
                unimplemented!("component start section");
            }

            Payload::AliasSection(s) => {
                self.validator.alias_section(&s)?;
                for alias in s {
                    self.alias(&alias?);
                }
            }

            // All custom sections are ignored by Wasmtime at this time.
            //
            // FIXME(WebAssembly/component-model#14): probably want to specify
            // and parse a `name` section here.
            Payload::CustomSection { .. } => {}

            // Anything else is either not reachable since we never enable the
            // feature in Wasmtime or we do enable it and it's a bug we don't
            // implement it, so let validation take care of most errors here and
            // if it gets past validation provide a helpful error message to
            // debug.
            other => {
                self.validator.payload(&other)?;
                panic!("unimplemented section {other:?}");
            }
        }

        Ok(Action::KeepGoing)
    }

    fn module_instance(
        &mut self,
        module: ModuleIndex,
        args: &[wasmparser::ModuleArg<'data>],
    ) -> InstanceDef<'data> {
        // Map the flat list of `args` to instead a name-to-instance index.
        let mut instance_by_name = HashMap::new();
        for arg in args {
            match arg.kind {
                wasmparser::ModuleArgKind::Instance(idx) => {
                    instance_by_name.insert(arg.name, InstanceIndex::from_u32(idx));
                }
            }
        }

        let instantiation = match self.result.modules[module] {
            // For modules which we are statically aware of we can look at the
            // exact order of imports required and build up a list of arguemnts
            // in that order. This will be fast at runtime because all we have
            // to do is build up the import lists for instantiation, no name
            // lookups necessary.
            ModuleDef::Upvar(upvar_idx) => {
                let trans = &self.result.upvars[upvar_idx];
                Instantiation::ModuleUpvar {
                    module: upvar_idx,
                    args: self.module_instance_args(
                        &instance_by_name,
                        trans.module.imports().map(|(m, n, _)| (m, n)),
                    ),
                }
            }

            // For imported modules the list of arguments is built to match the
            // order of the imports listed. Note that this will need to be
            // reshuffled at runtime since the actual module being instantiated
            // may originally have required imports in a different order.
            ModuleDef::Import {
                type_idx,
                import_index,
            } => {
                let ty = &self.types[type_idx];
                Instantiation::ModuleImport {
                    import_index,
                    args: self.module_instance_args(
                        &instance_by_name,
                        ty.imports.keys().map(|(a, b)| (a.as_str(), b.as_str())),
                    ),
                }
            }
        };
        let instance = self.result.component.instances.push(instantiation);
        InstanceDef::Module { instance, module }
    }

    /// Translates the named arguments required by a core wasm module specified
    /// by `iter` into a list of where the arguments come from at runtime.
    ///
    /// The `instance_by_name` map is used go go from the module name of an
    /// import to the instance that's satisfying the import. The `name` field
    /// of the import is then looked up within the instance's own exports.
    fn module_instance_args<'b>(
        &self,
        instance_by_name: &HashMap<&'data str, InstanceIndex>,
        iter: impl Iterator<Item = (&'b str, &'b str)>,
    ) -> Box<[CoreExport<EntityIndex>]> {
        iter.map(|(module, name)| {
            self.lookup_core_source(instance_by_name[module], name)
                .to_core_export(|i| i)
        })
        .collect()
    }

    /// Looks up the `CoreSource` corresponding to the export `name` of the
    /// `module` specified.
    ///
    /// This classifies the export of the module as either one which we
    /// statically know by index within the module itself (because we know the
    /// module), or one that must be referred to by name.
    fn lookup_core_source(&self, instance: InstanceIndex, name: &'data str) -> CoreSource<'data> {
        match &self.result.instances[instance] {
            // The `instance` points to an instantiated module...
            InstanceDef::Module { module, instance } => match self.result.modules[*module] {
                // ... and the module instantiated is one that we statically
                // know the structure of. This means that `name` points to an
                // exact index of an item within the module which we lookup here
                // and record.
                ModuleDef::Upvar(upvar_idx) => {
                    let trans = &self.result.upvars[upvar_idx];
                    CoreSource::Index(*instance, trans.module.exports[name])
                }

                // ... and the module instantiated is imported so we don't
                // statically know its structure. This means taht the export
                // must be identified by name.
                ModuleDef::Import { .. } => CoreSource::Export(*instance, name),
            },

            // The `instance `points to a "synthetic" instance created in the
            // component as a collection of named items from other instances.
            // This means that we're simply copying over the original source of
            // the item in the first place.
            InstanceDef::ModuleSynthetic(defs) => match defs[&name] {
                EntityIndex::Function(f) => match &self.result.funcs[f] {
                    Func::Core(c) => c.clone(),
                    // should not be possible to hit with a valid component
                    Func::Lifted { .. } => unreachable!(),
                },
                EntityIndex::Global(g) => self.result.globals[g].clone(),
                EntityIndex::Table(t) => self.result.tables[t].clone(),
                EntityIndex::Memory(m) => self.result.memories[m].clone(),
            },
        }
    }

    /// Creates a synthetic module from the list of items currently in the
    /// module and their given names.
    fn module_instance_from_exports(
        &mut self,
        exports: &[wasmparser::Export<'data>],
    ) -> InstanceDef<'data> {
        let mut map = HashMap::with_capacity(exports.len());
        for export in exports {
            let idx = match export.kind {
                wasmparser::ExternalKind::Func => {
                    let index = FuncIndex::from_u32(export.index);
                    EntityIndex::Function(index)
                }
                wasmparser::ExternalKind::Table => {
                    let index = TableIndex::from_u32(export.index);
                    EntityIndex::Table(index)
                }
                wasmparser::ExternalKind::Memory => {
                    let index = MemoryIndex::from_u32(export.index);
                    EntityIndex::Memory(index)
                }
                wasmparser::ExternalKind::Global => {
                    let index = GlobalIndex::from_u32(export.index);
                    EntityIndex::Global(index)
                }

                // doesn't get past validation
                wasmparser::ExternalKind::Tag => unimplemented!(),
            };
            map.insert(export.name, idx);
        }
        InstanceDef::ModuleSynthetic(map)
    }

    fn export(&mut self, export: &wasmparser::ComponentExport<'data>) {
        let name = export.name;
        let export = match export.kind {
            wasmparser::ComponentExportKind::Module(i) => {
                let idx = ModuleIndex::from_u32(i);
                drop(idx);
                unimplemented!("unimplemented module export");
            }
            wasmparser::ComponentExportKind::Component(i) => {
                let idx = ComponentIndex::from_u32(i);
                drop(idx);
                unimplemented!("unimplemented component export");
            }
            wasmparser::ComponentExportKind::Instance(i) => {
                let idx = InstanceIndex::from_u32(i);
                drop(idx);
                unimplemented!("unimplemented instance export");
            }
            wasmparser::ComponentExportKind::Function(i) => {
                let idx = FuncIndex::from_u32(i);
                match &self.result.funcs[idx] {
                    Func::Lifted { ty, func, options } => Export::LiftedFunction(LiftedFunction {
                        ty: *ty,
                        func: func.to_core_export(|i| match i {
                            EntityIndex::Function(i) => i,
                            _ => unreachable!(),
                        }),
                        options: options.clone(),
                    }),
                    // should not be possible to hit with a valid module.
                    Func::Core(_) => unreachable!(),
                }
            }
            wasmparser::ComponentExportKind::Value(_) => {
                unimplemented!("unimplemented value export");
            }
            wasmparser::ComponentExportKind::Type(i) => {
                let idx = TypeIndex::from_u32(i);
                drop(idx);
                unimplemented!("unimplemented value export");
            }
        };
        self.result
            .component
            .exports
            .insert(name.to_string(), export);
    }

    fn alias(&mut self, alias: &wasmparser::Alias<'data>) {
        match alias {
            wasmparser::Alias::InstanceExport {
                kind,
                instance,
                name,
            } => {
                let instance = InstanceIndex::from_u32(*instance);
                match &self.result.instances[instance] {
                    InstanceDef::Module { .. } | InstanceDef::ModuleSynthetic(_) => {
                        self.alias_module_instance_export(*kind, instance, name);
                    }
                }
            }
            wasmparser::Alias::OuterModule { .. } => {
                unimplemented!("alias outer module");
            }
            wasmparser::Alias::OuterComponent { .. } => {
                unimplemented!("alias outer component");
            }

            // When aliasing a type the `ComponentTypesBuilder` is used to
            // resolve the outer `count` plus the index, and then once it's
            // resolved we push the type information into our local index
            // space.
            //
            // Note that this is just copying indices around as all type
            // information is basically a pointer back into the `TypesBuilder`
            // structure (and the eventual `TypeTables` that it produces).
            wasmparser::Alias::OuterType { count, index } => {
                let index = TypeIndex::from_u32(*index);
                let ty = self.types.component_outer_type(*count, index);
                self.types.push_component_typedef(ty);
            }
        }
    }

    /// Inserts an item in to the relevant namespace aliasing the `name`'d
    /// export of the `instance` provided.
    fn alias_module_instance_export(
        &mut self,
        kind: wasmparser::AliasKind,
        instance: InstanceIndex,
        name: &'data str,
    ) {
        let src = self.lookup_core_source(instance, name);
        match kind {
            wasmparser::AliasKind::Func => {
                self.result.funcs.push(Func::Core(src));
            }
            wasmparser::AliasKind::Global => {
                self.result.globals.push(src);
            }
            wasmparser::AliasKind::Memory => {
                self.result.memories.push(src);
            }
            wasmparser::AliasKind::Table => {
                self.result.tables.push(src);
            }
            other => {
                panic!("unknown/unimplemented alias kind {other:?}");
            }
        }
    }

    fn lift_function(
        &self,
        ty: TypeIndex,
        func: FuncIndex,
        options: &[wasmparser::CanonicalOption],
    ) -> Func<'data> {
        let ty = match self.types.component_outer_type(0, ty) {
            TypeDef::Func(ty) => ty,
            // should not be possible after validation
            _ => unreachable!(),
        };
        let func = match &self.result.funcs[func] {
            Func::Core(core) => core.clone(),
            // should not be possible after validation
            Func::Lifted { .. } => unreachable!(),
        };
        let options = self.canonical_options(options);
        Func::Lifted { ty, func, options }
    }

    fn canonical_options(&self, opts: &[wasmparser::CanonicalOption]) -> CanonicalOptions {
        let mut ret = CanonicalOptions::default();
        for opt in opts {
            match opt {
                wasmparser::CanonicalOption::UTF8 => {
                    ret.string_encoding = Some(StringEncoding::Utf8);
                }
                wasmparser::CanonicalOption::UTF16 => {
                    ret.string_encoding = Some(StringEncoding::Utf16);
                }
                wasmparser::CanonicalOption::CompactUTF16 => {
                    ret.string_encoding = Some(StringEncoding::CompactUtf16);
                }
                wasmparser::CanonicalOption::Into(instance) => {
                    let instance = InstanceIndex::from_u32(*instance);

                    // Note that the `unreachable!()` should not happen for
                    // components which have passed validation.
                    let memory = self
                        .lookup_core_source(instance, "memory")
                        .to_core_export(|i| match i {
                            EntityIndex::Memory(i) => i,
                            _ => unreachable!(),
                        });
                    let canonical_abi_free = self
                        .lookup_core_source(instance, "canonical_abi_free")
                        .to_core_export(|i| match i {
                            EntityIndex::Function(i) => i,
                            _ => unreachable!(),
                        });
                    let canonical_abi_realloc = self
                        .lookup_core_source(instance, "canonical_abi_realloc")
                        .to_core_export(|i| match i {
                            EntityIndex::Function(i) => i,
                            _ => unreachable!(),
                        });
                    ret.intrinsics = Some(Intrinsics {
                        memory,
                        canonical_abi_free,
                        canonical_abi_realloc,
                    })
                }
            }
        }
        return ret;
    }
}

impl CoreSource<'_> {
    fn to_core_export<T>(&self, get_index: impl FnOnce(EntityIndex) -> T) -> CoreExport<T> {
        match self {
            CoreSource::Index(instance, index) => CoreExport {
                instance: *instance,
                item: ExportItem::Index(get_index(*index)),
            },
            CoreSource::Export(instance, name) => CoreExport {
                instance: *instance,
                item: ExportItem::Name(name.to_string()),
            },
        }
    }
}
