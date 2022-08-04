//! Identification and creation of fused adapter modules in Wasmtime.
//!
//! A major piece of the component model is the ability for core wasm modules to
//! talk to each other through the use of lifted and lowered functions. For
//! example one core wasm module can export a function which is lifted. Another
//! component could import that lifted function, lower it, and pass it as the
//! import to another core wasm module. This is what Wasmtime calls "adapter
//! fusion" where two core wasm functions are coming together through the
//! component model.
//!
//! There are a few ingredients during adapter fusion:
//!
//! * A core wasm function which is "lifted".
//! * A "lift type" which is the type that the component model function had in
//!   the original component
//! * A "lower type" which is the type that the component model function has
//!   in the destination component (the one the uses `canon lower`)
//! * Configuration options for both the lift and the lower operations such as
//!   memories, reallocs, etc.
//!
//! With these ingredients combined Wasmtime must produce a function which
//! connects the two components through the options specified. The fused adapter
//! performs tasks such as validation of passed values, copying data between
//! linear memories, etc.
//!
//! Wasmtime's current implementation of fused adapters is designed to reduce
//! complexity elsewhere as much as possible while also being suitable for being
//! used as a polyfill for the component model in JS environments as well. To
//! that end Wasmtime implements a fused adapter with another wasm module that
//! it itself generates on the fly. The usage of WebAssembly for fused adapters
//! has a number of advantages:
//!
//! * There is no need to create a raw Cranelift-based compiler. This is where
//!   majority of "unsafety" lives in Wasmtime so reducing the need to lean on
//!   this or audit another compiler is predicted to weed out a whole class of
//!   bugs in the fused adapter compiler.
//!
//! * As mentioned above generation of WebAssembly modules means that this is
//!   suitable for use in JS environments. For example a hypothetical tool which
//!   polyfills a component onto the web today would need to do something for
//!   adapter modules, and ideally the adapters themselves are speedy. While
//!   this could all be written in JS the adapting process is quite nontrivial
//!   so sharing code with Wasmtime would be ideal.
//!
//! * Using WebAssembly insulates the implementation to bugs to a certain
//!   degree. While logic bugs are still possible it should be much more
//!   difficult to have segfaults or things like that. With adapters exclusively
//!   executing inside a WebAssembly sandbox like everything else the failure
//!   modes to the host at least should be minimized.
//!
//! * Integration into the runtime is relatively simple, the adapter modules are
//!   just another kind of wasm module to instantiate and wire up at runtime.
//!   The goal is that the `GlobalInitializer` list that is processed at runtime
//!   will have all of its `Adapter`-using variants erased by the time it makes
//!   its way all the way up to Wasmtime. This means that the support in
//!   Wasmtime prior to adapter modules is actually the same as the support
//!   after adapter modules are added, keeping the runtime fiddly bits quite
//!   minimal.
//!
//! This isn't to say that this approach isn't without its disadvantages of
//! course. For now though this seems to be a reasonable set of tradeoffs for
//! the development stage of the component model proposal.
//!
//! ## Creating adapter modules
//!
//! With WebAssembly itself being used to implement fused adapters, Wasmtime
//! still has the question of how to organize the adapter functions into actual
//! wasm modules.
//!
//! The first thing you might reach for is to put all the adapters into the same
//! wasm module. This cannot be done, however, because some adapters may depend
//! on other adapters (transitively) to be created. This means that if
//! everything were in the same module there would be no way to instantiate the
//! module. An example of this dependency is an adapter (A) used to create a
//! core wasm instance (M) whose exported memory is then referenced by another
//! adapter (B). In this situation the adapter B cannot be in the same module
//! as adapter A because B needs the memory of M but M is created with A which
//! would otherwise create a circular dependency.
//!
//! The second possibility of organizing adapter modules would be to place each
//! fused adapter into its own module. Each `canon lower` would effectively
//! become a core wasm module instantiation at that point. While this works it's
//! currently believed to be a bit too fine-grained. For example it would mean
//! that importing a dozen lowered functions into a module could possibly result
//! in up to a dozen different adapter modules. While this possibility could
//! work it has been ruled out as "probably too expensive at runtime".
//!
//! Thus the purpose and existence of this module is now evident -- this module
//! exists to identify what exactly goes into which adapter module. This will
//! evaluate the `GlobalInitializer` lists coming out of the `inline` pass and
//! insert `InstantiateModule` entries for where adapter modules should be
//! created.
//!
//! ## Partitioning adapter modules
//!
//! Currently this module does not attempt to be really all that fancy about
//! grouping adapters into adapter modules. The main idea is that most items
//! within an adapter module are likely to be close together since they're
//! theoretically going to be used for an instantiation of a core wasm module
//! just after the fused adapter was declared. With that in mind the current
//! algorithm is a one-pass approach to partitioning everything into adapter
//! modules.
//!
//! As the `GlobalInitializer` list is iterated over the last adapter module
//! created is recorded. Each adapter module, when created, records the index
//! space limits at the time of its creation. If a new adapter is found which
//! depends on an item after the original adapter module was created then the
//! prior adapter module is finished and a new one is started. Adapters only
//! ever attempt to get inserted into the most recent adapter module, no
//! searching is currently done to try to fit adapters into a prior adapter
//! module.
//!
//! During this remapping process the `RuntimeInstanceIndex` for all instances
//! is also updated. Insertion of an adapter module will increase all further
//! instance indices by one so this must be accounted for in various
//! references.

use crate::component::translate::*;
use crate::fact::Module;
use wasmparser::WasmFeatures;

/// Information about fused adapters within a component.
#[derive(Default)]
pub struct Adapters {
    /// List of all fused adapters identified which are assigned an index and
    /// contain various metadata about them as well.
    pub adapters: PrimaryMap<AdapterIndex, Adapter>,
}

/// Metadata information about a fused adapter.
pub struct Adapter {
    /// The type used when the original core wasm function was lifted.
    ///
    /// Note that this could be different than `lower_ty` (but still matches
    /// according to subtyping rules).
    pub lift_ty: TypeFuncIndex,
    /// Canonical ABI options used when the function was lifted.
    pub lift_options: AdapterOptions,
    /// The type used when the function was lowered back into a core wasm
    /// function.
    ///
    /// Note that this could be different than `lift_ty` (but still matches
    /// according to subtyping rules).
    pub lower_ty: TypeFuncIndex,
    /// Canonical ABI options used when the function was lowered.
    pub lower_options: AdapterOptions,
    /// The original core wasm function which was lifted.
    pub func: CoreDef,
}

/// Configuration options which can be specified as part of the canonical ABI
/// in the component model.
#[derive(Clone)]
pub struct AdapterOptions {
    /// The Wasmtime-assigned component instance index where the options were
    /// originally specified.
    pub instance: RuntimeComponentInstanceIndex,
    /// How strings are encoded.
    pub string_encoding: StringEncoding,
    /// An optional memory definition supplied.
    pub memory: Option<CoreExport<MemoryIndex>>,
    /// If `memory` is specified, whether it's a 64-bit memory.
    pub memory64: bool,
    /// An optional definition of `realloc` to used.
    pub realloc: Option<CoreDef>,
    /// An optional definition of a `post-return` to use.
    pub post_return: Option<CoreDef>,
}

impl<'data> Translator<'_, 'data> {
    /// Modifies the list of `GlobalInitializer` entries within a
    /// `Component`with `InstantiateModule::Adapter` entries where necessary.
    ///
    /// This is the entrypoint of functionality within this module which
    /// performs all the work of identifying adapter usages and organizing
    /// everything into adapter modules.
    pub(super) fn insert_adapter_module_initializers(
        &mut self,
        component: &mut Component,
        adapters: &mut Adapters,
    ) {
        let mut state = PartitionAdapterModules {
            to_process: Vec::new(),
            cur_idx: 0,
            adapter_modules: PrimaryMap::new(),
            items: DefinedItems::default(),
            instance_map: PrimaryMap::with_capacity(component.num_runtime_instances as usize),
        };
        state.run(component, adapters);

        // Next, in reverse, insert all of the adapter modules into the actual
        // initializer list. Note that the iteration order is important here to
        // ensure that all the `at_initializer_index` listed is valid for each
        // entry.
        let mut adapter_map = PrimaryMap::with_capacity(adapters.adapters.len());
        for _ in adapters.adapters.iter() {
            adapter_map.push(None);
        }
        for (_, module) in state.adapter_modules.into_iter().rev() {
            let index = module.at_initializer_index;
            let instantiate = self.compile_adapter_module(module, adapters, &mut adapter_map);
            let init = GlobalInitializer::InstantiateModule(instantiate);
            component.initializers.insert(index, init);
        }

        // Finally all references to `CoreDef::Adapter` are rewritten to their
        // corresponding `CoreDef::Export` as identified within `adapter_map`.
        for init in component.initializers.iter_mut() {
            map_adapter_references(init, &adapter_map);
        }
    }

    fn compile_adapter_module(
        &mut self,
        module_parts: AdapterModuleParts,
        adapters: &Adapters,
        adapter_map: &mut PrimaryMap<AdapterIndex, Option<CoreExport<EntityIndex>>>,
    ) -> InstantiateModule {
        // Use the `fact::Module` builder to create a new wasm module which
        // represents all of the adapters specified here.
        let mut module = Module::new(
            self.types.component_types(),
            self.tunables.debug_adapter_modules,
        );
        let mut names = Vec::with_capacity(module_parts.adapters.len());
        for adapter in module_parts.adapters.iter() {
            let name = format!("adapter{}", adapter.as_u32());
            module.adapt(&name, &adapters.adapters[*adapter]);
            names.push(name);
        }
        let wasm = module.encode();
        let args = module.imports().to_vec();

        // Extend the lifetime of the owned `wasm: Vec<u8>` on the stack to a
        // higher scope defined by our original caller. That allows to transform
        // `wasm` into `&'data [u8]` which is much easier to work with here.
        let wasm = &*self.scope_vec.push(wasm);
        if log::log_enabled!(log::Level::Trace) {
            match wasmprinter::print_bytes(wasm) {
                Ok(s) => log::trace!("generated adapter module:\n{}", s),
                Err(e) => log::trace!("failed to print adapter module: {}", e),
            }
        }

        // With the wasm binary this is then pushed through general translation,
        // validation, etc. Note that multi-memory is specifically enabled here
        // since the adapter module is highly likely to use that if anything is
        // actually indirected through memory.
        let mut validator = Validator::new_with_features(WasmFeatures {
            multi_memory: true,
            ..*self.validator.features()
        });
        let translation = ModuleEnvironment::new(
            self.tunables,
            &mut validator,
            self.types.module_types_builder(),
        )
        .translate(Parser::new(0), wasm)
        .expect("invalid adapter module generated");

        // And with all metadata available about the generated module a map can
        // be built from adapter index to the precise export in the module that
        // was generated.
        for (adapter, name) in module_parts.adapters.iter().zip(&names) {
            assert!(adapter_map[*adapter].is_none());
            let index = translation.module.exports[name];
            adapter_map[*adapter] = Some(CoreExport {
                instance: module_parts.index,
                item: ExportItem::Index(index),
            });
        }

        // Finally the module translation is saved in the list of static
        // modules to get fully compiled later and the `InstantiateModule`
        // representation of this adapter module is returned.
        let static_index = self.static_modules.push(translation);
        InstantiateModule::Static(static_index, args.into())
    }
}

struct PartitionAdapterModules {
    /// Stack of remaining elements to process
    to_process: Vec<ToProcess>,

    /// Index of the current `GlobalInitializer` being processed.
    cur_idx: usize,

    /// Information about all fused adapter modules that have been created so
    /// far.
    ///
    /// This is modified whenever a fused adapter is used.
    adapter_modules: PrimaryMap<AdapterModuleIndex, AdapterModuleParts>,

    /// Map from "old runtime instance index" to "new runtime instance index".
    ///
    /// This map is populated when instances are created to account for prior
    /// adapter modules having been created. This effectively tracks an offset
    /// for each index.
    instance_map: PrimaryMap<RuntimeInstanceIndex, RuntimeInstanceIndex>,

    /// Current limits of index spaces.
    items: DefinedItems,
}

/// Entries in the `PartitionAdapterModules::to_process` array.
enum ToProcess {
    /// An adapter needs its own dependencies processed. This will map the
    /// fields of `Adapter` above for the specified index.
    Adapter(AdapterIndex),
    /// An adapter has had its dependencies fully processed (transitively) and
    /// the adapter now needs to be inserted into a module.
    AddAdapterToModule(AdapterIndex),
    /// A global initializer needs to be remapped.
    GlobalInitializer(usize),
    /// An export needs to be remapped.
    Export(usize),
    /// A global initializer which creates an instance has had all of its
    /// arguments processed and now the instance number needs to be recorded.
    PushInstance,
}

/// Custom index type used exclusively for the `adapter_modules` map above.
#[derive(Copy, Clone, PartialEq, Eq)]
struct AdapterModuleIndex(u32);
cranelift_entity::entity_impl!(AdapterModuleIndex);

struct AdapterModuleParts {
    /// The runtime index that will be assigned to this adapter module when it's
    /// instantiated.
    index: RuntimeInstanceIndex,
    /// The index in the `GlobalInitializer` list that this adapter module will
    /// get inserted at.
    at_initializer_index: usize,
    /// Items that were available when this adapter module was created.
    items_at_initializer: DefinedItems,
    /// Adapters that have been inserted into this module, guaranteed to be
    /// non-empty.
    adapters: Vec<AdapterIndex>,
}

#[derive(Default, Clone)]
struct DefinedItems {
    /// Number of core wasm instances created so far.
    ///
    /// Note that this does not count adapter modules created, only the
    /// instance index space before adapter modules were inserted.
    instances: u32,
    /// Number of host-lowered functions seen so far.
    lowerings: u32,
    /// Number of "always trap" functions seen so far.
    always_trap: u32,
    /// Map of whether adapters have been inserted into an adapter module yet.
    adapter_to_module: PrimaryMap<AdapterIndex, Option<AdapterModuleIndex>>,
}

impl PartitionAdapterModules {
    /// Process the list of global `initializers` and partitions adapters into
    /// adapter modules which will get inserted into the provided list in a
    /// later pass.
    fn run(&mut self, component: &mut Component, adapters: &mut Adapters) {
        // This function is designed to be an iterative loop which models
        // recursion in the `self.to_process` array instead of on the host call
        // stack. The reason for this is that adapters need recursive processing
        // since the argument to an adapter can hypothetically be an adapter
        // itself (albeit silly but still valid). This recursive nature of
        // adapters means that a component could be crafted to have an
        // arbitrarily deep recursive dependeny chain for any one adapter. To
        // avoid consuming host stack space the storage for this dependency
        // chain is placed on the heap.
        //
        // The `self.to_process` list is a FIFO queue of what to process next.
        // Initially seeded with all the global initializer indexes this is
        // pushed to during processing to recursively handle adapters and
        // similar.
        assert!(self.to_process.is_empty());
        assert!(self.items.adapter_to_module.is_empty());

        // Initially record all adapters as having no module which will get
        // filled in over time.
        for _ in adapters.adapters.iter() {
            self.items.adapter_to_module.push(None);
        }

        // Seed the worklist of what to process with the list of global
        // initializers and exports, but in reverse order since this is a LIFO
        // queue.  Afterwards all of the items to process are handled in a loop.
        for i in (0..component.exports.len()).rev() {
            self.to_process.push(ToProcess::Export(i));
        }
        for i in (0..component.initializers.len()).rev() {
            self.to_process.push(ToProcess::GlobalInitializer(i));
        }

        while let Some(to_process) = self.to_process.pop() {
            match to_process {
                ToProcess::GlobalInitializer(i) => {
                    assert!(i <= self.cur_idx + 1);
                    self.cur_idx = i;
                    self.global_initializer(&mut component.initializers[i]);
                }

                ToProcess::Export(i) => {
                    self.cur_idx = component.initializers.len();
                    self.export(&mut component.exports[i]);
                }

                ToProcess::PushInstance => {
                    // A new runtime instance is being created here so insert an
                    // entry into the remapping map for instance indexes. This
                    // instance's index is offset by the number of adapter modules
                    // created prior.
                    self.instance_map
                        .push(RuntimeInstanceIndex::from_u32(self.items.instances));
                    self.items.instances += 1;
                }

                ToProcess::Adapter(idx) => {
                    let info = &mut adapters.adapters[idx];
                    self.process_core_def(&mut info.func);
                    self.process_options(&mut info.lift_options);
                    self.process_options(&mut info.lower_options);
                }

                ToProcess::AddAdapterToModule(idx) => {
                    // If this adapter has already been assigned to a module
                    // then there's no need to do anything else here.
                    //
                    // This can happen when a core wasm instance is created with
                    // an adapter as the argument multiple times for example.
                    if self.items.adapter_to_module[idx].is_some() {
                        continue;
                    }

                    // If an adapter module is already in progress and
                    // everything this adapter depends on was available at the
                    // time of creation of that adapter module, then this
                    // adapter can go in that module.
                    if let Some((module_idx, module)) = self.adapter_modules.last_mut() {
                        let info = &adapters.adapters[idx];
                        if module.items_at_initializer.contains(info) {
                            self.items.adapter_to_module[idx] = Some(module_idx);
                            module.adapters.push(idx);
                            continue;
                        }
                    }

                    // ... otherwise a new adapter module is started. Note that
                    // the instance count is bumped here to model the
                    // instantiation of the adapter module.
                    let module = AdapterModuleParts {
                        index: RuntimeInstanceIndex::from_u32(self.items.instances),
                        at_initializer_index: self.cur_idx,
                        items_at_initializer: self.items.clone(),
                        adapters: vec![idx],
                    };
                    let index = self.adapter_modules.push(module);
                    self.items.adapter_to_module[idx] = Some(index);
                    self.items.instances += 1;
                }
            }
        }
    }

    fn global_initializer(&mut self, init: &mut GlobalInitializer) {
        match init {
            GlobalInitializer::InstantiateModule(module) => {
                // Enqueue a bump of the instance count, but this only happens
                // after all the arguments have been processed below. Given the
                // LIFO nature of `self.to_process` this will be handled after
                // all arguments are recursively processed.
                self.to_process.push(ToProcess::PushInstance);

                match module {
                    InstantiateModule::Static(_, args) => {
                        for def in args.iter_mut() {
                            self.process_core_def(def);
                        }
                    }
                    InstantiateModule::Import(_, args) => {
                        for (_, map) in args {
                            for (_, def) in map {
                                self.process_core_def(def);
                            }
                        }
                    }
                }
            }

            GlobalInitializer::ExtractRealloc(e) => self.process_core_def(&mut e.def),
            GlobalInitializer::ExtractPostReturn(e) => self.process_core_def(&mut e.def),

            // Update items available as they're defined
            GlobalInitializer::LowerImport(_) => self.items.lowerings += 1,
            GlobalInitializer::AlwaysTrap(_) => self.items.always_trap += 1,

            GlobalInitializer::ExtractMemory(memory) => {
                self.process_core_export(&mut memory.export);
            }

            // Nothing is defined or referenced by these initializers that we
            // need to worry about here.
            GlobalInitializer::SaveStaticModule(_) => {}
            GlobalInitializer::SaveModuleImport(_) => {}
        }
    }

    fn export(&mut self, export: &mut Export) {
        match export {
            Export::LiftedFunction { func, .. } => {
                self.process_core_def(func);
            }
            Export::Instance(exports) => {
                for (_, export) in exports {
                    self.export(export);
                }
            }
            Export::Module(_) => {}
        }
    }

    fn process_options(&mut self, opts: &mut AdapterOptions) {
        if let Some(memory) = &mut opts.memory {
            self.process_core_export(memory);
        }
        if let Some(def) = &mut opts.realloc {
            self.process_core_def(def);
        }
        if let Some(def) = &mut opts.post_return {
            self.process_core_def(def);
        }
    }

    fn process_core_def(&mut self, def: &mut CoreDef) {
        match def {
            CoreDef::Adapter(idx) => {
                // The `to_process` queue is a LIFO queue so first enqueue the
                // addition of this adapter into a module followed by the
                // processing of the adapter itself. This means that the
                // adapter's own dependencies will be processed before the
                // adapter is added to a module.
                self.to_process.push(ToProcess::AddAdapterToModule(*idx));
                self.to_process.push(ToProcess::Adapter(*idx));
            }

            CoreDef::Export(e) => self.process_core_export(e),

            // These are ignored since they don't contain a reference to an
            // adapter which may need to be inserted into a module.
            CoreDef::Lowered(_) | CoreDef::AlwaysTrap(_) | CoreDef::InstanceFlags(_) => {}
        }
    }

    fn process_core_export<T>(&mut self, export: &mut CoreExport<T>) {
        // Remap the instance index referenced here as necessary to account
        // for any adapter modules that needed creating in the meantime.
        export.instance = self.instance_map[export.instance];
    }
}

impl DefinedItems {
    fn contains(&self, info: &Adapter) -> bool {
        self.contains_options(&info.lift_options)
            && self.contains_options(&info.lower_options)
            && self.contains_def(&info.func)
    }

    fn contains_options(&self, options: &AdapterOptions) -> bool {
        let AdapterOptions {
            instance: _,
            string_encoding: _,
            memory64: _,
            memory,
            realloc,
            post_return,
        } = options;

        if let Some(mem) = memory {
            if !self.contains_export(mem) {
                return false;
            }
        }

        if let Some(def) = realloc {
            if !self.contains_def(def) {
                return false;
            }
        }

        if let Some(def) = post_return {
            if !self.contains_def(def) {
                return false;
            }
        }

        true
    }

    fn contains_def(&self, options: &CoreDef) -> bool {
        match options {
            CoreDef::Export(e) => self.contains_export(e),
            CoreDef::AlwaysTrap(i) => i.as_u32() < self.always_trap,
            CoreDef::Lowered(i) => i.as_u32() < self.lowerings,
            CoreDef::Adapter(idx) => self.adapter_to_module[*idx].is_some(),
            CoreDef::InstanceFlags(_) => true,
        }
    }

    fn contains_export<T>(&self, export: &CoreExport<T>) -> bool {
        // This `DefinedItems` index space will contain `export` if the
        // instance referenced has already been instantiated. The actual item
        // that `export` points to doesn't need to be tested since it comes
        // from the instance regardless.
        export.instance.as_u32() < self.instances
    }
}

/// Rewrites all instances of `CoreDef::Adapter` within the `init` initializer
/// provided to `CoreExport` according to the `map` provided.
///
/// This is called after all adapter modules have been constructed and the
/// core wasm function for each adapter has been identified.
fn map_adapter_references(
    init: &mut GlobalInitializer,
    map: &PrimaryMap<AdapterIndex, Option<CoreExport<EntityIndex>>>,
) {
    let map_core_def = |def: &mut CoreDef| {
        let adapter = match def {
            CoreDef::Adapter(idx) => *idx,
            _ => return,
        };
        *def = CoreDef::Export(
            map[adapter]
                .clone()
                .expect("adapter should have been instantiated"),
        );
    };
    match init {
        GlobalInitializer::InstantiateModule(module) => match module {
            InstantiateModule::Static(_, args) => {
                for def in args.iter_mut() {
                    map_core_def(def);
                }
            }
            InstantiateModule::Import(_, args) => {
                for (_, map) in args {
                    for (_, def) in map {
                        map_core_def(def);
                    }
                }
            }
        },

        GlobalInitializer::ExtractRealloc(e) => map_core_def(&mut e.def),
        GlobalInitializer::ExtractPostReturn(e) => map_core_def(&mut e.def),

        // Nothing to map here
        GlobalInitializer::LowerImport(_)
        | GlobalInitializer::AlwaysTrap(_)
        | GlobalInitializer::ExtractMemory(_) => {}
        GlobalInitializer::SaveStaticModule(_) => {}
        GlobalInitializer::SaveModuleImport(_) => {}
    }
}
