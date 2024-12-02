use crate::component::*;
use crate::prelude::*;
use crate::ScopeVec;
use crate::{
    EngineOrModuleTypeIndex, EntityIndex, ModuleEnvironment, ModuleInternedTypeIndex,
    ModuleTranslation, ModuleTypesBuilder, PrimaryMap, Tunables, TypeConvert, WasmHeapType,
    WasmResult, WasmValType,
};
use anyhow::anyhow;
use anyhow::{bail, Result};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::mem;
use wasmparser::component_types::{
    AliasableResourceId, ComponentCoreModuleTypeId, ComponentEntityType, ComponentFuncTypeId,
    ComponentInstanceTypeId,
};
use wasmparser::types::Types;
use wasmparser::{Chunk, ComponentImportName, Encoding, Parser, Payload, Validator};

mod adapt;
pub use self::adapt::*;
mod inline;

/// Structure used to translate a component and parse it.
pub struct Translator<'a, 'data> {
    /// The current component being translated.
    ///
    /// This will get swapped out as translation traverses the body of a
    /// component and a sub-component is entered or left.
    result: Translation<'data>,

    /// Current state of parsing a binary component. Note that like `result`
    /// this will change as the component is traversed.
    parser: Parser,

    /// Stack of lexical scopes that are in-progress but not finished yet.
    ///
    /// This is pushed to whenever a component is entered and popped from
    /// whenever a component is left. Each lexical scope also contains
    /// information about the variables that it is currently required to close
    /// over which is threaded into the current in-progress translation of
    /// the sub-component which pushed a scope here.
    lexical_scopes: Vec<LexicalScope<'data>>,

    /// The validator in use to verify that the raw input binary is a valid
    /// component.
    validator: &'a mut Validator,

    /// Type information shared for the entire component.
    ///
    /// This builder is also used for all core wasm modules found to intern
    /// signatures across all modules.
    types: PreInliningComponentTypes<'a>,

    /// The compiler configuration provided by the embedder.
    tunables: &'a Tunables,

    /// Auxiliary location to push generated adapter modules onto.
    scope_vec: &'data ScopeVec<u8>,

    /// Completely translated core wasm modules that have been found so far.
    ///
    /// Note that this translation only involves learning about type
    /// information and functions are not actually compiled here.
    static_modules: PrimaryMap<StaticModuleIndex, ModuleTranslation<'data>>,

    /// Completely translated components that have been found so far.
    ///
    /// As frames are popped from `lexical_scopes` their completed component
    /// will be pushed onto this list.
    static_components: PrimaryMap<StaticComponentIndex, Translation<'data>>,
}

/// Representation of the syntactic scope of a component meaning where it is
/// and what its state is at in the binary format.
///
/// These scopes are pushed and popped when a sub-component starts being
/// parsed and finishes being parsed. The main purpose of this frame is to
/// have a `ClosedOverVars` field which encapsulates data that is inherited
/// from the scope specified into the component being translated just beneath
/// it.
///
/// This structure exists to implement outer aliases to components and modules.
/// When a component or module is closed over then that means it needs to be
/// inherited in a sense to the component which actually had the alias. This is
/// achieved with a deceptively simple scheme where each parent of the
/// component with the alias will inherit the component from the desired
/// location.
///
/// For example with a component structure that looks like:
///
/// ```wasm
/// (component $A
///     (core module $M)
///     (component $B
///         (component $C
///             (alias outer $A $M (core module))
///         )
///     )
/// )
/// ```
///
/// here the `C` component is closing over `M` located in the root component
/// `A`. When `C` is being translated the `lexical_scopes` field will look like
/// `[A, B]`. When the alias is encountered (for module index 0) this will
/// place a `ClosedOverModule::Local(0)` entry into the `closure_args` field of
/// `A`'s frame. This will in turn give a `ModuleUpvarIndex` which is then
/// inserted into `closure_args` in `B`'s frame. This produces yet another
/// `ModuleUpvarIndex` which is finally inserted into `C`'s module index space
/// via `LocalInitializer::AliasModuleUpvar` with the last index.
///
/// All of these upvar indices and such are interpreted in the "inline" phase
/// of compilation and not at runtime. This means that when `A` is being
/// instantiated one of its initializers will be
/// `LocalInitializer::ComponentStatic`. This starts to create `B` and the
/// variables captured for `B` are listed as local module 0, or `M`. This list
/// is then preserved in the definition of the component `B` and later reused
/// by `C` again to finally get access to the closed over component.
///
/// Effectively the scopes are managed hierarchically where a reference to an
/// outer variable automatically injects references into all parents up to
/// where the reference is. This variable scopes are the processed during
/// inlining where a component definition is a reference to the static
/// component information (`Translation`) plus closed over variables
/// (`ComponentClosure` during inlining).
struct LexicalScope<'data> {
    /// Current state of translating the `translation` below.
    parser: Parser,
    /// Current state of the component's translation as found so far.
    translation: Translation<'data>,
    /// List of captures that `translation` will need to process to create the
    /// sub-component which is directly beneath this lexical scope.
    closure_args: ClosedOverVars,
}

/// A "local" translation of a component.
///
/// This structure is used as a sort of in-progress translation of a component.
/// This is not `Component` which is the final form as consumed by Wasmtime
/// at runtime. Instead this is a fairly simple representation of a component
/// where almost everything is ordered as a list of initializers. The binary
/// format is translated to a list of initializers here which is later processed
/// during "inlining" to produce a final component with the final set of
/// initializers.
#[derive(Default)]
struct Translation<'data> {
    /// Instructions which form this component.
    ///
    /// There is one initializer for all members of each index space, and all
    /// index spaces are incrementally built here as the initializer list is
    /// processed.
    initializers: Vec<LocalInitializer<'data>>,

    /// The list of exports from this component, as pairs of names and an
    /// index into an index space of what's being exported.
    exports: IndexMap<&'data str, ComponentItem>,

    /// Type information produced by `wasmparser` for this component.
    ///
    /// This type information is available after the translation of the entire
    /// component has finished, e.g. for the `inline` pass, but beforehand this
    /// is set to `None`.
    types: Option<Types>,
}

// NB: the type information contained in `LocalInitializer` should always point
// to `wasmparser`'s type information, not Wasmtime's. Component types cannot be
// fully determined due to resources until instantiations are known which is
// tracked during the inlining phase. This means that all type information below
// is straight from `wasmparser`'s passes.
enum LocalInitializer<'data> {
    // imports
    Import(ComponentImportName<'data>, ComponentEntityType),

    // canonical function sections
    Lower {
        func: ComponentFuncIndex,
        lower_ty: ComponentFuncTypeId,
        canonical_abi: ModuleInternedTypeIndex,
        options: LocalCanonicalOptions,
    },
    Lift(ComponentFuncTypeId, FuncIndex, LocalCanonicalOptions),

    // resources
    Resource(AliasableResourceId, WasmValType, Option<FuncIndex>),
    ResourceNew(AliasableResourceId, ModuleInternedTypeIndex),
    ResourceRep(AliasableResourceId, ModuleInternedTypeIndex),
    ResourceDrop(AliasableResourceId, ModuleInternedTypeIndex),

    // core wasm modules
    ModuleStatic(StaticModuleIndex, ComponentCoreModuleTypeId),

    // core wasm module instances
    ModuleInstantiate(ModuleIndex, HashMap<&'data str, ModuleInstanceIndex>),
    ModuleSynthetic(HashMap<&'data str, EntityIndex>),

    // components
    ComponentStatic(StaticComponentIndex, ClosedOverVars),

    // component instances
    ComponentInstantiate(
        ComponentIndex,
        HashMap<&'data str, ComponentItem>,
        ComponentInstanceTypeId,
    ),
    ComponentSynthetic(HashMap<&'data str, ComponentItem>, ComponentInstanceTypeId),

    // alias section
    AliasExportFunc(ModuleInstanceIndex, &'data str),
    AliasExportTable(ModuleInstanceIndex, &'data str),
    AliasExportGlobal(ModuleInstanceIndex, &'data str),
    AliasExportMemory(ModuleInstanceIndex, &'data str),
    AliasComponentExport(ComponentInstanceIndex, &'data str),
    AliasModule(ClosedOverModule),
    AliasComponent(ClosedOverComponent),

    // export section
    Export(ComponentItem),
}

/// The "closure environment" of components themselves.
///
/// For more information see `LexicalScope`.
#[derive(Default)]
struct ClosedOverVars {
    components: PrimaryMap<ComponentUpvarIndex, ClosedOverComponent>,
    modules: PrimaryMap<ModuleUpvarIndex, ClosedOverModule>,
}

/// Description how a component is closed over when the closure variables for
/// a component are being created.
///
/// For more information see `LexicalScope`.
enum ClosedOverComponent {
    /// A closed over component is coming from the local component's index
    /// space, meaning a previously defined component is being captured.
    Local(ComponentIndex),
    /// A closed over component is coming from our own component's list of
    /// upvars. This list was passed to us by our enclosing component, which
    /// will eventually have bottomed out in closing over a `Local` component
    /// index for some parent component.
    Upvar(ComponentUpvarIndex),
}

/// Same as `ClosedOverComponent`, but for modules.
enum ClosedOverModule {
    Local(ModuleIndex),
    Upvar(ModuleUpvarIndex),
}

/// Representation of canonical ABI options.
struct LocalCanonicalOptions {
    string_encoding: StringEncoding,
    memory: Option<MemoryIndex>,
    realloc: Option<FuncIndex>,
    post_return: Option<FuncIndex>,
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
        scope_vec: &'data ScopeVec<u8>,
    ) -> Self {
        let mut parser = Parser::new(0);
        parser.set_features(*validator.features());
        Self {
            result: Translation::default(),
            tunables,
            validator,
            types: PreInliningComponentTypes::new(types),
            parser,
            lexical_scopes: Vec::new(),
            static_components: Default::default(),
            static_modules: Default::default(),
            scope_vec,
        }
    }

    /// Translates the binary `component`.
    ///
    /// This is the workhorse of compilation which will parse all of
    /// `component` and create type information for Wasmtime and such. The
    /// `component` does not have to be valid and it will be validated during
    /// compilation.
    ///
    /// The result of this function is a tuple of the final component's
    /// description plus a list of core wasm modules found within the
    /// component. The component's description actually erases internal
    /// components, instances, etc, as much as it can. Instead `Component`
    /// retains a flat list of initializers (no nesting) which was created
    /// as part of compilation from the nested structure of the original
    /// component.
    ///
    /// The list of core wasm modules found is provided to allow compiling
    /// modules externally in parallel. Additionally initializers in
    /// `Component` may refer to the modules in the map returned by index.
    ///
    /// # Errors
    ///
    /// This function will return an error if the `component` provided is
    /// invalid.
    pub fn translate(
        mut self,
        component: &'data [u8],
    ) -> Result<(
        ComponentTranslation,
        PrimaryMap<StaticModuleIndex, ModuleTranslation<'data>>,
    )> {
        // First up wasmparser is used to actually perform the translation and
        // validation of this component. This will produce a list of core wasm
        // modules in addition to components which are found during the
        // translation process. When doing this only a `Translation` is created
        // which is a simple representation of a component.
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
        assert!(remaining.is_empty());
        assert!(self.lexical_scopes.is_empty());

        // ... after translation initially finishes the next pass is performed
        // which we're calling "inlining". This will "instantiate" the root
        // component, following nested component instantiations, creating a
        // global list of initializers along the way. This phase uses the simple
        // initializers in each component to track dataflow of host imports and
        // internal references to items throughout a component at compile-time.
        // The produce initializers in the final `Component` are intended to be
        // much simpler than the original component and more efficient for
        // Wasmtime to process at runtime as well (e.g. no string lookups as
        // most everything is done through indices instead).
        let mut component = inline::run(
            self.types.types_mut_for_inlining(),
            &self.result,
            &self.static_modules,
            &self.static_components,
        )?;
        self.partition_adapter_modules(&mut component);
        let translation =
            component.finish(self.types.types_mut_for_inlining(), self.result.types_ref())?;
        Ok((translation, self.static_modules))
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
            }

            Payload::End(offset) => {
                assert!(self.result.types.is_none());
                self.result.types = Some(self.validator.end(offset)?);

                // Exit the current lexical scope. If there is no parent (no
                // frame currently on the stack) then translation is finished.
                // Otherwise that means that a nested component has been
                // completed and is recorded as such.
                let LexicalScope {
                    parser,
                    translation,
                    closure_args,
                } = match self.lexical_scopes.pop() {
                    Some(frame) => frame,
                    None => return Ok(Action::Done),
                };
                self.parser = parser;
                let component = mem::replace(&mut self.result, translation);
                let static_idx = self.static_components.push(component);
                self.result
                    .initializers
                    .push(LocalInitializer::ComponentStatic(static_idx, closure_args));
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
                let mut component_type_index =
                    self.validator.types(0).unwrap().component_type_count();
                self.validator.component_type_section(&s)?;

                // Look for resource types and if a local resource is defined
                // then an initializer is added to define that resource type and
                // reference its destructor.
                let types = self.validator.types(0).unwrap();
                for ty in s {
                    match ty? {
                        wasmparser::ComponentType::Resource { rep, dtor } => {
                            let rep = self.types.convert_valtype(rep);
                            let id = types
                                .component_any_type_at(component_type_index)
                                .unwrap_resource();
                            let dtor = dtor.map(FuncIndex::from_u32);
                            self.result
                                .initializers
                                .push(LocalInitializer::Resource(id, rep, dtor));
                        }

                        // no extra processing needed
                        wasmparser::ComponentType::Defined(_)
                        | wasmparser::ComponentType::Func(_)
                        | wasmparser::ComponentType::Instance(_)
                        | wasmparser::ComponentType::Component(_) => {}
                    }

                    component_type_index += 1;
                }
            }
            Payload::CoreTypeSection(s) => {
                self.validator.core_type_section(&s)?;
            }

            // Processing the import section at this point is relatively simple
            // which is to simply record the name of the import and the type
            // information associated with it.
            Payload::ComponentImportSection(s) => {
                self.validator.component_import_section(&s)?;
                for import in s {
                    let import = import?;
                    let types = self.validator.types(0).unwrap();
                    let ty = types
                        .component_entity_type_of_import(import.name.0)
                        .unwrap();
                    self.result
                        .initializers
                        .push(LocalInitializer::Import(import.name, ty));
                }
            }

            // Entries in the canonical section will get initializers recorded
            // with the listed options for lifting/lowering.
            Payload::ComponentCanonicalSection(s) => {
                let mut core_func_index = self.validator.types(0).unwrap().function_count();
                self.validator.component_canonical_section(&s)?;
                for func in s {
                    let types = self.validator.types(0).unwrap();
                    let init = match func? {
                        wasmparser::CanonicalFunction::Lift {
                            type_index,
                            core_func_index,
                            options,
                        } => {
                            let ty = types.component_any_type_at(type_index).unwrap_func();
                            let func = FuncIndex::from_u32(core_func_index);
                            let options = self.canonical_options(&options);
                            LocalInitializer::Lift(ty, func, options)
                        }
                        wasmparser::CanonicalFunction::Lower {
                            func_index,
                            options,
                        } => {
                            let lower_ty = types.component_function_at(func_index);
                            let func = ComponentFuncIndex::from_u32(func_index);
                            let options = self.canonical_options(&options);
                            let canonical_abi = self.core_func_signature(core_func_index)?;

                            core_func_index += 1;
                            LocalInitializer::Lower {
                                func,
                                options,
                                canonical_abi,
                                lower_ty,
                            }
                        }
                        wasmparser::CanonicalFunction::ResourceNew { resource } => {
                            let resource = types.component_any_type_at(resource).unwrap_resource();
                            let ty = self.core_func_signature(core_func_index)?;
                            core_func_index += 1;
                            LocalInitializer::ResourceNew(resource, ty)
                        }
                        wasmparser::CanonicalFunction::ResourceDrop { resource } => {
                            let resource = types.component_any_type_at(resource).unwrap_resource();
                            let ty = self.core_func_signature(core_func_index)?;
                            core_func_index += 1;
                            LocalInitializer::ResourceDrop(resource, ty)
                        }
                        wasmparser::CanonicalFunction::ResourceRep { resource } => {
                            let resource = types.component_any_type_at(resource).unwrap_resource();
                            let ty = self.core_func_signature(core_func_index)?;
                            core_func_index += 1;
                            LocalInitializer::ResourceRep(resource, ty)
                        }

                        wasmparser::CanonicalFunction::ThreadSpawn { .. }
                        | wasmparser::CanonicalFunction::ThreadHwConcurrency => {
                            bail!("unsupported intrinsic")
                        }
                    };
                    self.result.initializers.push(init);
                }
            }

            // Core wasm modules are translated inline directly here with the
            // `ModuleEnvironment` from core wasm compilation. This will return
            // to the caller the size of the module so it knows how many bytes
            // of the input are skipped.
            //
            // Note that this is just initial type translation of the core wasm
            // module and actual function compilation is deferred until this
            // entire process has completed.
            Payload::ModuleSection {
                parser,
                unchecked_range,
            } => {
                let index = self.validator.types(0).unwrap().module_count();
                self.validator.module_section(&unchecked_range)?;
                let translation = ModuleEnvironment::new(
                    self.tunables,
                    self.validator,
                    self.types.module_types_builder(),
                )
                .translate(
                    parser,
                    component
                        .get(unchecked_range.start..unchecked_range.end)
                        .ok_or_else(|| {
                            anyhow!(
                                "section range {}..{} is out of bounds (bound = {})",
                                unchecked_range.start,
                                unchecked_range.end,
                                component.len()
                            )
                            .context("wasm component contains an invalid module section")
                        })?,
                )?;
                let static_idx = self.static_modules.push(translation);
                let types = self.validator.types(0).unwrap();
                let ty = types.module_at(index);
                self.result
                    .initializers
                    .push(LocalInitializer::ModuleStatic(static_idx, ty));
                return Ok(Action::Skip(unchecked_range.end - unchecked_range.start));
            }

            // When a sub-component is found then the current translation state
            // is pushed onto the `lexical_scopes` stack. This will subsequently
            // get popped as part of `Payload::End` processing above.
            //
            // Note that the set of closure args for this new lexical scope
            // starts empty since it will only get populated if translation of
            // the nested component ends up aliasing some outer module or
            // component.
            Payload::ComponentSection {
                parser,
                unchecked_range,
            } => {
                self.validator.component_section(&unchecked_range)?;
                self.lexical_scopes.push(LexicalScope {
                    parser: mem::replace(&mut self.parser, parser),
                    translation: mem::take(&mut self.result),
                    closure_args: ClosedOverVars::default(),
                });
            }

            // Both core wasm instances and component instances record
            // initializers of what form of instantiation is performed which
            // largely just records the arguments given from wasmparser into a
            // `HashMap` for processing later during inlining.
            Payload::InstanceSection(s) => {
                self.validator.instance_section(&s)?;
                for instance in s {
                    let init = match instance? {
                        wasmparser::Instance::Instantiate { module_index, args } => {
                            let index = ModuleIndex::from_u32(module_index);
                            self.instantiate_module(index, &args)
                        }
                        wasmparser::Instance::FromExports(exports) => {
                            self.instantiate_module_from_exports(&exports)
                        }
                    };
                    self.result.initializers.push(init);
                }
            }
            Payload::ComponentInstanceSection(s) => {
                let mut index = self.validator.types(0).unwrap().component_instance_count();
                self.validator.component_instance_section(&s)?;
                for instance in s {
                    let types = self.validator.types(0).unwrap();
                    let ty = types.component_instance_at(index);
                    let init = match instance? {
                        wasmparser::ComponentInstance::Instantiate {
                            component_index,
                            args,
                        } => {
                            let index = ComponentIndex::from_u32(component_index);
                            self.instantiate_component(index, &args, ty)?
                        }
                        wasmparser::ComponentInstance::FromExports(exports) => {
                            self.instantiate_component_from_exports(&exports, ty)?
                        }
                    };
                    self.result.initializers.push(init);
                    index += 1;
                }
            }

            // Exports don't actually fill out the `initializers` array but
            // instead fill out the one other field in a `Translation`, the
            // `exports` field (as one might imagine). This for now simply
            // records the index of what's exported and that's tracked further
            // later during inlining.
            Payload::ComponentExportSection(s) => {
                self.validator.component_export_section(&s)?;
                for export in s {
                    let export = export?;
                    let item = self.kind_to_item(export.kind, export.index)?;
                    let prev = self.result.exports.insert(export.name.0, item);
                    assert!(prev.is_none());
                    self.result
                        .initializers
                        .push(LocalInitializer::Export(item));
                }
            }

            Payload::ComponentStartSection { start, range } => {
                self.validator.component_start_section(&start, &range)?;
                unimplemented!("component start section");
            }

            // Aliases of instance exports (either core or component) will be
            // recorded as an initializer of the appropriate type with outer
            // aliases handled specially via upvars and type processing.
            Payload::ComponentAliasSection(s) => {
                self.validator.component_alias_section(&s)?;
                for alias in s {
                    let init = match alias? {
                        wasmparser::ComponentAlias::InstanceExport {
                            kind: _,
                            instance_index,
                            name,
                        } => {
                            let instance = ComponentInstanceIndex::from_u32(instance_index);
                            LocalInitializer::AliasComponentExport(instance, name)
                        }
                        wasmparser::ComponentAlias::Outer { kind, count, index } => {
                            self.alias_component_outer(kind, count, index);
                            continue;
                        }
                        wasmparser::ComponentAlias::CoreInstanceExport {
                            kind,
                            instance_index,
                            name,
                        } => {
                            let instance = ModuleInstanceIndex::from_u32(instance_index);
                            self.alias_module_instance_export(kind, instance, name)
                        }
                    };
                    self.result.initializers.push(init);
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

    fn instantiate_module(
        &mut self,
        module: ModuleIndex,
        raw_args: &[wasmparser::InstantiationArg<'data>],
    ) -> LocalInitializer<'data> {
        let mut args = HashMap::with_capacity(raw_args.len());
        for arg in raw_args {
            match arg.kind {
                wasmparser::InstantiationArgKind::Instance => {
                    let idx = ModuleInstanceIndex::from_u32(arg.index);
                    args.insert(arg.name, idx);
                }
            }
        }
        LocalInitializer::ModuleInstantiate(module, args)
    }

    /// Creates a synthetic module from the list of items currently in the
    /// module and their given names.
    fn instantiate_module_from_exports(
        &mut self,
        exports: &[wasmparser::Export<'data>],
    ) -> LocalInitializer<'data> {
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
                wasmparser::ExternalKind::Tag => unimplemented!("wasm exceptions"),
            };
            map.insert(export.name, idx);
        }
        LocalInitializer::ModuleSynthetic(map)
    }

    fn instantiate_component(
        &mut self,
        component: ComponentIndex,
        raw_args: &[wasmparser::ComponentInstantiationArg<'data>],
        ty: ComponentInstanceTypeId,
    ) -> Result<LocalInitializer<'data>> {
        let mut args = HashMap::with_capacity(raw_args.len());
        for arg in raw_args {
            let idx = self.kind_to_item(arg.kind, arg.index)?;
            args.insert(arg.name, idx);
        }

        Ok(LocalInitializer::ComponentInstantiate(component, args, ty))
    }

    /// Creates a synthetic module from the list of items currently in the
    /// module and their given names.
    fn instantiate_component_from_exports(
        &mut self,
        exports: &[wasmparser::ComponentExport<'data>],
        ty: ComponentInstanceTypeId,
    ) -> Result<LocalInitializer<'data>> {
        let mut map = HashMap::with_capacity(exports.len());
        for export in exports {
            let idx = self.kind_to_item(export.kind, export.index)?;
            map.insert(export.name.0, idx);
        }

        Ok(LocalInitializer::ComponentSynthetic(map, ty))
    }

    fn kind_to_item(
        &mut self,
        kind: wasmparser::ComponentExternalKind,
        index: u32,
    ) -> Result<ComponentItem> {
        Ok(match kind {
            wasmparser::ComponentExternalKind::Func => {
                let index = ComponentFuncIndex::from_u32(index);
                ComponentItem::Func(index)
            }
            wasmparser::ComponentExternalKind::Module => {
                let index = ModuleIndex::from_u32(index);
                ComponentItem::Module(index)
            }
            wasmparser::ComponentExternalKind::Instance => {
                let index = ComponentInstanceIndex::from_u32(index);
                ComponentItem::ComponentInstance(index)
            }
            wasmparser::ComponentExternalKind::Component => {
                let index = ComponentIndex::from_u32(index);
                ComponentItem::Component(index)
            }
            wasmparser::ComponentExternalKind::Value => {
                unimplemented!("component values");
            }
            wasmparser::ComponentExternalKind::Type => {
                let types = self.validator.types(0).unwrap();
                let ty = types.component_any_type_at(index);
                ComponentItem::Type(ty)
            }
        })
    }

    fn alias_module_instance_export(
        &mut self,
        kind: wasmparser::ExternalKind,
        instance: ModuleInstanceIndex,
        name: &'data str,
    ) -> LocalInitializer<'data> {
        match kind {
            wasmparser::ExternalKind::Func => LocalInitializer::AliasExportFunc(instance, name),
            wasmparser::ExternalKind::Memory => LocalInitializer::AliasExportMemory(instance, name),
            wasmparser::ExternalKind::Table => LocalInitializer::AliasExportTable(instance, name),
            wasmparser::ExternalKind::Global => LocalInitializer::AliasExportGlobal(instance, name),
            wasmparser::ExternalKind::Tag => {
                unimplemented!("wasm exceptions");
            }
        }
    }

    fn alias_component_outer(
        &mut self,
        kind: wasmparser::ComponentOuterAliasKind,
        count: u32,
        index: u32,
    ) {
        match kind {
            wasmparser::ComponentOuterAliasKind::CoreType
            | wasmparser::ComponentOuterAliasKind::Type => {}

            // For more information about the implementation of outer aliases
            // see the documentation of `LexicalScope`. Otherwise though the
            // main idea here is that the data to close over starts as `Local`
            // and then transitions to `Upvar` as its inserted into the parents
            // in order from target we're aliasing back to the current
            // component.
            wasmparser::ComponentOuterAliasKind::CoreModule => {
                let index = ModuleIndex::from_u32(index);
                let mut module = ClosedOverModule::Local(index);
                let depth = self.lexical_scopes.len() - (count as usize);
                for frame in self.lexical_scopes[depth..].iter_mut() {
                    module = ClosedOverModule::Upvar(frame.closure_args.modules.push(module));
                }

                // If the `module` is still `Local` then the `depth` was 0 and
                // it's an alias into our own space. Otherwise it's switched to
                // an upvar and will index into the upvar space. Either way
                // it's just plumbed directly into the initializer.
                self.result
                    .initializers
                    .push(LocalInitializer::AliasModule(module));
            }
            wasmparser::ComponentOuterAliasKind::Component => {
                let index = ComponentIndex::from_u32(index);
                let mut component = ClosedOverComponent::Local(index);
                let depth = self.lexical_scopes.len() - (count as usize);
                for frame in self.lexical_scopes[depth..].iter_mut() {
                    component =
                        ClosedOverComponent::Upvar(frame.closure_args.components.push(component));
                }

                self.result
                    .initializers
                    .push(LocalInitializer::AliasComponent(component));
            }
        }
    }

    fn canonical_options(&self, opts: &[wasmparser::CanonicalOption]) -> LocalCanonicalOptions {
        let mut ret = LocalCanonicalOptions {
            string_encoding: StringEncoding::Utf8,
            memory: None,
            realloc: None,
            post_return: None,
        };
        for opt in opts {
            match opt {
                wasmparser::CanonicalOption::UTF8 => {
                    ret.string_encoding = StringEncoding::Utf8;
                }
                wasmparser::CanonicalOption::UTF16 => {
                    ret.string_encoding = StringEncoding::Utf16;
                }
                wasmparser::CanonicalOption::CompactUTF16 => {
                    ret.string_encoding = StringEncoding::CompactUtf16;
                }
                wasmparser::CanonicalOption::Memory(idx) => {
                    let idx = MemoryIndex::from_u32(*idx);
                    ret.memory = Some(idx);
                }
                wasmparser::CanonicalOption::Realloc(idx) => {
                    let idx = FuncIndex::from_u32(*idx);
                    ret.realloc = Some(idx);
                }
                wasmparser::CanonicalOption::PostReturn(idx) => {
                    let idx = FuncIndex::from_u32(*idx);
                    ret.post_return = Some(idx);
                }
            }
        }
        return ret;
    }

    /// Get the interned type index for the `index`th core function.
    fn core_func_signature(&mut self, index: u32) -> WasmResult<ModuleInternedTypeIndex> {
        let types = self.validator.types(0).unwrap();
        let id = types.core_function_at(index);
        self.types.module_types_builder().intern_type(types, id)
    }
}

impl Translation<'_> {
    fn types_ref(&self) -> wasmparser::types::TypesRef<'_> {
        self.types.as_ref().unwrap().as_ref()
    }
}

/// A small helper module which wraps a `ComponentTypesBuilder` and attempts
/// to disallow access to mutable access to the builder before the inlining
/// pass.
///
/// Type information in this translation pass must be preserved at the
/// wasmparser layer of abstraction rather than being lowered into Wasmtime's
/// own type system. Only during inlining are types fully assigned because
/// that's when resource types become available as it's known which instance
/// defines which resource, or more concretely the same component instantiated
/// twice will produce two unique resource types unlike one as seen by
/// wasmparser within the component.
mod pre_inlining {
    use super::*;

    pub struct PreInliningComponentTypes<'a> {
        types: &'a mut ComponentTypesBuilder,
    }

    impl<'a> PreInliningComponentTypes<'a> {
        pub fn new(types: &'a mut ComponentTypesBuilder) -> Self {
            Self { types }
        }

        pub fn module_types_builder(&mut self) -> &mut ModuleTypesBuilder {
            self.types.module_types_builder_mut()
        }

        pub fn types(&self) -> &ComponentTypesBuilder {
            self.types
        }

        // NB: this should in theory only be used for the `inline` phase of
        // translation.
        pub fn types_mut_for_inlining(&mut self) -> &mut ComponentTypesBuilder {
            self.types
        }
    }

    impl TypeConvert for PreInliningComponentTypes<'_> {
        fn lookup_heap_type(&self, index: wasmparser::UnpackedIndex) -> WasmHeapType {
            self.types.lookup_heap_type(index)
        }

        fn lookup_type_index(&self, index: wasmparser::UnpackedIndex) -> EngineOrModuleTypeIndex {
            self.types.lookup_type_index(index)
        }
    }
}
use pre_inlining::PreInliningComponentTypes;
