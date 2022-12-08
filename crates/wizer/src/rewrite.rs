//! Final rewrite pass.

mod renumbering;

use crate::{
    info::{
        types_interner::{EntityType, Type},
        Module, ModuleContext,
    },
    snapshot::Snapshot,
    translate, FuncRenames, Wizer, DEFAULT_KEEP_INIT_FUNC,
};
use renumbering::Renumbering;
use std::{convert::TryFrom, iter};
use wasm_encoder::SectionId;

impl Wizer {
    /// Given the initialized snapshot, rewrite the Wasm so that it is already
    /// initialized.
    ///
    pub(crate) fn rewrite(
        &self,
        cx: &mut ModuleContext<'_>,
        store: &crate::Store,
        snapshot: &Snapshot,
        renames: &FuncRenames,
        has_wasi_initialize: bool,
    ) -> Vec<u8> {
        log::debug!("Rewriting input Wasm to pre-initialized state");

        if cx.uses_module_linking() {
            self.rewrite_with_module_linking(cx, store, snapshot, renames, has_wasi_initialize)
        } else {
            self.rewrite_without_module_linking(cx, store, snapshot, renames, has_wasi_initialize)
        }
    }

    /// Rewrite a root Wasm module that has no children and doesn't use module
    /// linking at all.
    fn rewrite_without_module_linking(
        &self,
        cx: &ModuleContext<'_>,
        store: &crate::Store,
        snapshot: &Snapshot,
        renames: &FuncRenames,
        has_wasi_initialize: bool,
    ) -> Vec<u8> {
        assert!(snapshot.instantiations.is_empty());

        let mut encoder = wasm_encoder::Module::new();
        let module = cx.root();

        // Encode the initialized data segments from the snapshot rather
        // than the original, uninitialized data segments.
        let mut data_section = if snapshot.data_segments.is_empty() {
            None
        } else {
            let mut data_section = wasm_encoder::DataSection::new();
            for seg in &snapshot.data_segments {
                data_section.active(
                    seg.memory_index,
                    wasm_encoder::Instruction::I32Const(seg.offset as i32),
                    seg.data(store).iter().copied(),
                );
            }
            Some(data_section)
        };

        // There are multiple places were we potentially need to check whether
        // we've added the data section already and if we haven't yet, then do
        // so. For example, the original Wasm might not have a data section at
        // all, and so we have to potentially add it at the end of iterating
        // over the original sections. This closure encapsulates all that
        // add-it-if-we-haven't-already logic in one place.
        let mut add_data_section = |module: &mut wasm_encoder::Module| {
            if let Some(data_section) = data_section.take() {
                module.section(&data_section);
            }
        };

        for section in module.raw_sections(cx) {
            match section {
                // Some tools expect the name custom section to come last, even
                // though custom sections are allowed in any order. Therefore,
                // make sure we've added our data section by now.
                s if is_name_section(s) => {
                    add_data_section(&mut encoder);
                    encoder.section(s);
                }

                // For the memory section, we update the minimum size of each
                // defined memory to the snapshot's initialized size for that
                // memory.
                s if s.id == SectionId::Memory.into() => {
                    let mut memories = wasm_encoder::MemorySection::new();
                    assert_eq!(module.defined_memories_len(cx), snapshot.memory_mins.len());
                    for ((_, mem), new_min) in module
                        .defined_memories(cx)
                        .zip(snapshot.memory_mins.iter().copied())
                    {
                        let mut mem = translate::memory_type(mem);
                        mem.minimum = new_min;
                        memories.memory(mem);
                    }
                    encoder.section(&memories);
                }

                // Encode the initialized global values from the snapshot,
                // rather than the original values.
                s if s.id == SectionId::Global.into() => {
                    let mut globals = wasm_encoder::GlobalSection::new();
                    for ((_, glob_ty), val) in
                        module.defined_globals(cx).zip(snapshot.globals.iter())
                    {
                        let glob_ty = translate::global_type(glob_ty);
                        globals.global(
                            glob_ty,
                            match val {
                                wasmtime::Val::I32(x) => wasm_encoder::Instruction::I32Const(*x),
                                wasmtime::Val::I64(x) => wasm_encoder::Instruction::I64Const(*x),
                                wasmtime::Val::F32(x) => {
                                    wasm_encoder::Instruction::F32Const(f32::from_bits(*x))
                                }
                                wasmtime::Val::F64(x) => {
                                    wasm_encoder::Instruction::F64Const(f64::from_bits(*x))
                                }
                                _ => unreachable!(),
                            },
                        );
                    }
                    encoder.section(&globals);
                }

                // Remove exports for the wizer initialization
                // function and WASI reactor _initialize function,
                // then perform any requested renames.
                s if s.id == SectionId::Export.into() => {
                    let mut exports = wasm_encoder::ExportSection::new();
                    for export in module.exports(cx) {
                        if !self.keep_init_func.unwrap_or(DEFAULT_KEEP_INIT_FUNC)
                            && (export.field == self.init_func
                                || (has_wasi_initialize && export.field == "_initialize"))
                        {
                            continue;
                        }

                        if !renames.rename_src_to_dst.contains_key(export.field)
                            && renames.rename_dsts.contains(export.field)
                        {
                            // A rename overwrites this export, and it is not
                            // renamed to another export, so skip it.
                            continue;
                        }

                        let field = renames
                            .rename_src_to_dst
                            .get(export.field)
                            .map_or(export.field, |f| f.as_str());

                        let export = translate::export(export.kind, export.index);
                        exports.export(field, export);
                    }
                    encoder.section(&exports);
                }

                // Skip the `start` function -- it's already been run!
                s if s.id == SectionId::Start.into() => {
                    continue;
                }

                s if s.id == SectionId::DataCount.into() => {
                    encoder.section(&wasm_encoder::DataCountSection {
                        count: u32::try_from(snapshot.data_segments.len()).unwrap(),
                    });
                }

                s if s.id == SectionId::Data.into() => {
                    // TODO: supporting bulk memory will require copying over
                    // any passive and declared segments.
                    add_data_section(&mut encoder);
                }

                s if s.id == SectionId::Module.into() => unreachable!(),
                s if s.id == SectionId::Instance.into() => unreachable!(),
                s if s.id == SectionId::Alias.into() => unreachable!(),

                s => {
                    encoder.section(s);
                }
            }
        }

        // Make sure that we've added our data section to the module.
        add_data_section(&mut encoder);
        encoder.finish()
    }

    /// Rewrite a module linking bundle.
    ///
    /// ## Code Shape
    ///
    /// With module linking, we rewrite each module in the original bundle into
    /// a *code module* that doesn't define any internal state (i.e. memories,
    /// globals, and nested instances) and instead imports a *state instance*
    /// that exports all of these things. For each instantiation, we have a
    /// *state module* that defines the already-initialized state for that
    /// instantiation, we instantiate that state module to create one such state
    /// instance, and then use this as an import argument in the instantiation
    /// of the original code module. This way, we do not duplicate shared code
    /// bodies across multiple instantiations of the same module.
    ///
    /// Note that the root module is also split out into a code module and state
    /// module, even though it is never explicitly instantiated inside the
    /// bundle.
    ///
    /// The new root is an "umbrella" module that defines all the types used
    /// within the whole bundle. Each nested module then aliases its types from
    /// the umbrella module. The umbrella module aliases all exports of the
    /// original root and re-exports them.
    ///
    /// For example, given this input Wasm module:
    ///
    /// ```wat
    /// (module $A
    ///   (module $B
    ///     (memory $B_mem)
    ///     (global $B_glob (mut i32))
    ///     (func (export "f") ...)
    ///   )
    ///
    ///   (instance $x (instantiate $B))
    ///   (instance $y (instantiate $B))
    ///
    ///   (memory $A_mem)
    ///   (global $A_glob (mut i32))
    ///
    ///   (func (export "g") ...)
    /// )
    /// ```
    ///
    /// and some post-initialization state, this rewrite pass will produce the
    /// following pre-initialized module:
    ///
    /// ```wat
    /// (module $Umbrella
    ///   (module $A
    ///     ;; Locally defined state is replaced by a state instance import.
    ///     (import "__wizer_state"
    ///       (instance
    ///         (export "__wizer_memory_0" (memory $A_mem))
    ///         (export "__wizer_global_0" (global $A_glob (mut i32)))
    ///         (export "__wizer_instance_0" (instance $x (export "f" (func))))
    ///         (export "__wizer_instance_1" (instance $y (export "f" (func))))
    ///       )
    ///     )
    ///     (func (export "g") ...)
    ///   )
    ///
    ///   (module $B
    ///     ;; Locally defined state is replaced by a state instance import.
    ///     (import "__wizer_state"
    ///       (instance
    ///         (export "__wizer_memory_0" (memory $B_mem))
    ///         (export "__wizer_global_0" (global $B_glob (mut i32)))
    ///       )
    ///     )
    ///     (func (export "f") ...)
    ///   )
    ///
    ///   ;; Instantiations are replaced with specialized state modules that get
    ///   ;; instantiated exactly once to produce state instances, and finally
    ///   ;; the original instantiations are rewritten into instantiations of
    ///   ;; the corresponding code module with the state instance as an import
    ///   ;; argument.
    ///
    ///   ;; State module for `$A`.
    ///   (module $A_state_module
    ///     ;; State module for `$x`.
    ///     (module $x_state_module
    ///       (memory (export "__wizer_memory_0") (memory))
    ///       ;; Data segments to initialize the memory based on our snapshot
    ///       ;; would go here...
    ///
    ///       (global (export "__wizer_global_0")
    ///         (global (mut i32) (i32.const (; ...snapshot's initialized value goes here... ;)))
    ///       )
    ///     )
    ///
    ///     ;; State instance for `$x`.
    ///     (instance $x_state (instantiate $x_state_module))
    ///
    ///     ;; The instantiation of `$x` is now rewritten to use our state
    ///     ;; instance.
    ///     (instance $x (instantiate $B (import "__wizer_state" $x_state)))
    ///
    ///     ;; Same goes for the `$y` instantiation.
    ///     (module $y_state_module
    ///       (memory (export "__wizer_memory_0") (memory))
    ///       ;; Data segments...
    ///       (global (export "__wizer_global_0")
    ///         (global (mut i32) (i32.const (; ...snapshot's initialized value goes here... ;)))
    ///       )
    ///     )
    ///     (instance $y_state (instantiate $y_state_module))
    ///     (instance $y (instantiate $B (import "__wizer_state" $y_state)))
    ///
    ///     (memory $A_mem)
    ///     (global $A_glob (mut i32))
    ///   )
    ///
    ///   ;; State instance for `$A`.
    ///   (instance $a_state (instantiate $A_state_module))
    ///
    ///   ;; The state is now joined with the code.
    ///   (instance $a_instance (instantiate $A (import "__wizer_state" $a_state)))
    ///
    ///   ;; And finally we re-export all of our old root's exports.
    ///   (export "g" (func $a_instance "g"))
    /// )
    /// ```
    ///
    /// ## Implementation
    ///
    /// To implement this transformation, we first do a pre-order walk of the
    /// module tree and emit the code modules as a flat sequence. Why a flat
    /// sequence? The code modules cannot contain nested instantiations, because
    /// nested instantiations are state that is not necessarily shared across
    /// all instantiations of the outer module. And if we are already lifting
    /// out nested instantiations, we need to also make nested modules available
    /// for those lifted instantiations, and the easiest way to do that is to
    /// flatten the code module tree (as opposed to re-exporting the nested
    /// modules under well-known symbols). The pre-order traversal ensures that
    /// the `Module::id` we assigned during the instrumentation phase matches
    /// the module's place in the index space.
    ///
    /// The state modules, however, remain a nested tree, and we emit them in a
    /// traversal of the `Snapshot` instance tree. We can do this because,
    /// unlike code modules, each state module is only instantiated exactly
    /// once. The instantiations' references to nested modules become outer
    /// aliases pointing to the module's position in the parent's flat sequence
    /// of nested modules.
    fn rewrite_with_module_linking(
        &self,
        cx: &mut ModuleContext<'_>,
        store: &crate::Store,
        snapshot: &Snapshot,
        renames: &FuncRenames,
        has_wasi_initialize: bool,
    ) -> Vec<u8> {
        let mut umbrella = wasm_encoder::Module::new();

        // Counts of various entities defined inside the umbrella module thus
        // far.
        let mut umbrella_funcs = 0;
        let mut umbrella_tables = 0;
        let mut umbrella_memories = 0;
        let mut umbrella_globals = 0;
        let mut umbrella_instances = 0;

        let (code_modules, num_code_modules) = rewrite_code_modules(cx);

        let root_state_module = rewrite_state_module(cx, store, cx.root(), &snapshot, 0);
        let mut modules = wasm_encoder::ModuleSection::new();
        modules.module(&root_state_module);
        let root_state_module_index = num_code_modules;

        // Imports that we will need to pass through from the umbrella to the
        // root instance.
        let import_sections: Vec<_> = cx
            .root()
            .initial_sections(cx)
            .filter(|s| s.id == SectionId::Import.into())
            .collect();

        // Instantiate the root state module by forwarding imports from the
        // umbrella to the root instantiation.
        //
        // There is some trickery for implicit instance imports. We forward the
        // implicit instance as an instantiation argument, since we can't
        // provide two-level instantiation arguments, which means we need to
        // determine when implicit imports are injected again.
        let mut instances = wasm_encoder::InstanceSection::new();
        let mut args = vec![];
        let mut imports = cx.root().imports(cx).iter().peekable();
        loop {
            let (module, is_two_level) = match imports.peek() {
                Some(imp) => (imp.module, imp.field.is_some()),
                None => break,
            };

            if is_two_level {
                args.push((module, wasm_encoder::Export::Instance(umbrella_instances)));
                umbrella_instances += 1;
            }
            while imports.peek().map_or(false, |imp| imp.module == module) {
                let imp = imports.next().unwrap();
                let export = match imp.ty {
                    wasmparser::ImportSectionEntryType::Function(_) => {
                        umbrella_funcs += 1;
                        wasm_encoder::Export::Function(umbrella_funcs - 1)
                    }
                    wasmparser::ImportSectionEntryType::Instance(_) => {
                        umbrella_instances += 1;
                        wasm_encoder::Export::Instance(umbrella_instances - 1)
                    }
                    _ => unreachable!(),
                };
                if !is_two_level {
                    args.push((module, export));
                }
            }
        }
        instances.instantiate(root_state_module_index, args.iter().cloned());
        let root_state_instance_index = umbrella_instances;
        umbrella_instances += 1;

        // Instantiate the root code module with the root state module.
        let root_module_index = cx.root().pre_order_index();
        args.push((
            "__wizer_state",
            wasm_encoder::Export::Instance(root_state_instance_index),
        ));
        instances.instantiate(root_module_index, args);
        let root_instance_index = umbrella_instances;
        umbrella_instances += 1;

        // Alias the root instance's exports and then re-export them.
        let mut aliases = wasm_encoder::AliasSection::new();
        let mut exports = wasm_encoder::ExportSection::new();
        for exp in cx.root().exports(cx) {
            if !self.keep_init_func.unwrap_or(DEFAULT_KEEP_INIT_FUNC)
                && (exp.field == self.init_func
                    || (has_wasi_initialize && exp.field == "_initialize"))
            {
                continue;
            }

            if !renames.rename_src_to_dst.contains_key(exp.field)
                && renames.rename_dsts.contains(exp.field)
            {
                // A rename overwrites this export, and it is not renamed to
                // another export, so skip it.
                continue;
            }

            let kind = translate::item_kind(exp.kind);
            aliases.instance_export(root_instance_index, kind, exp.field);

            let field = renames
                .rename_src_to_dst
                .get(exp.field)
                .map_or(exp.field, |f| f.as_str());
            exports.export(
                field,
                match kind {
                    wasm_encoder::ItemKind::Function => {
                        umbrella_funcs += 1;
                        wasm_encoder::Export::Function(umbrella_funcs - 1)
                    }
                    wasm_encoder::ItemKind::Table => {
                        umbrella_tables += 1;
                        wasm_encoder::Export::Table(umbrella_tables - 1)
                    }
                    wasm_encoder::ItemKind::Memory => {
                        umbrella_memories += 1;
                        wasm_encoder::Export::Memory(umbrella_memories - 1)
                    }
                    wasm_encoder::ItemKind::Global => {
                        umbrella_globals += 1;
                        wasm_encoder::Export::Global(umbrella_globals - 1)
                    }
                    wasm_encoder::ItemKind::Instance => {
                        umbrella_instances += 1;
                        wasm_encoder::Export::Instance(umbrella_instances - 1)
                    }
                    wasm_encoder::ItemKind::Module => unreachable!(),
                },
            );
        }

        // NB: We encode the types last, even though it is the first section we
        // place in the umbrella module, since adding state imports may need to
        // define new instance types.
        let types = umbrella_type_section(cx);

        // Now combine all our sections together in the umbrella module.
        umbrella.section(&types);
        umbrella.section(&code_modules);
        umbrella.section(&modules);
        for s in import_sections {
            umbrella.section(s);
        }
        umbrella.section(&instances);
        umbrella.section(&aliases);
        umbrella.section(&exports);

        umbrella.finish()
    }
}

/// Create a type section with everything in the whole interned types set.
fn umbrella_type_section(cx: &ModuleContext<'_>) -> wasm_encoder::TypeSection {
    let mut types = wasm_encoder::TypeSection::new();

    let interned_entity_to_encoder_entity = |ty: &EntityType| match ty {
        EntityType::Function(ty) => wasm_encoder::EntityType::Function(ty.index()),
        EntityType::Table(ty) => wasm_encoder::EntityType::Table(translate::table_type(*ty)),
        EntityType::Memory(ty) => wasm_encoder::EntityType::Memory(translate::memory_type(*ty)),
        EntityType::Global(ty) => wasm_encoder::EntityType::Global(translate::global_type(*ty)),
        EntityType::Module(ty) => wasm_encoder::EntityType::Module(ty.index()),
        EntityType::Instance(ty) => wasm_encoder::EntityType::Instance(ty.index()),
    };

    for (_index, ty) in cx.types().iter() {
        match ty {
            Type::Func(f) => types.function(
                f.params.iter().copied().map(translate::val_type),
                f.returns.iter().copied().map(translate::val_type),
            ),
            Type::Instance(inst) => types.instance(
                inst.exports
                    .iter()
                    .map(|(name, ty)| (name.as_ref(), interned_entity_to_encoder_entity(ty))),
            ),
            Type::Module(module) => types.module(
                module.imports.iter().map(|((module, name), ty)| {
                    (
                        module.as_ref(),
                        name.as_ref().map(|n| n.as_ref()),
                        interned_entity_to_encoder_entity(ty),
                    )
                }),
                module
                    .exports
                    .iter()
                    .map(|(name, ty)| (name.as_ref(), interned_entity_to_encoder_entity(ty))),
            ),
        };
    }

    types
}

fn is_name_section(s: &wasm_encoder::RawSection) -> bool {
    s.id == SectionId::Custom.into() && {
        let mut reader = wasmparser::BinaryReader::new(s.data);
        matches!(reader.read_string(), Ok("name"))
    }
}

/// Rewrite nested modules into a flat sequence, and where they import their
/// state, rather than define it locally.
///
/// Returns the modules encoded in a module section and total number of code
/// modules defined.
fn rewrite_code_modules(cx: &mut ModuleContext) -> (wasm_encoder::ModuleSection, u32) {
    let mut code_modules = wasm_encoder::ModuleSection::new();
    let mut num_code_modules = 0;

    cx.root().pre_order(cx, |cx, info| {
        if info.get_aliased(cx).is_some() {
            // Add a dummy module. This isn't ever actually used, since we will
            // instead resolve the alias at the use sites and then use the
            // aliased referent instead. If we had an alias kind like "alias a
            // module from this index space" we would use that here. But we have
            // to add an entry to the module index space to preserve our
            // invariant that a code module is at its pre-order index in the
            // umbrella's module index space.
            code_modules.module(&wasm_encoder::Module::new());
            num_code_modules += 1;
            return;
        }

        let mut module = wasm_encoder::Module::new();

        // We generally try to avoid renumbering entities in Wizer --
        // particularly any entities referenced from the code section, where
        // renumbering could change the size of a LEB128 index and break DWARF
        // debug info offsets -- because it means we can't copy whole sections
        // from the original, input Wasm module. But we run into a conflicting
        // constraints here with regards to instances:
        //
        // 1. Ideally we would import our state instance last, so that we don't
        //    perturb with our instance index space.
        //
        // 2. Locally-defined instances are state, and therefore must be pulled
        //    out of these code modules into our imported state instance, and
        //    then referenced via alias.
        //
        // (1) and (2) are in conflict because we can't create aliases of
        // instances on the imported state instance until *after* the state
        // instance is imported, which means we need to import our state
        // instance first, which means we are forced to perturb the instance
        // index space.
        //
        // Therefore, the first thing we add to each code module is an import
        // section that imports the state instance. We need to explicitly
        // rewrite all references to these instances (e.g. instance export
        // aliases) to add one to their index so that they refer to the correct
        // instance again. Luckily instances are never referenced from the code
        // section, so DWARF debug info doesn't get invalidated.
        //
        // Finally, importing the state instance requires that we define the
        // state instance's type. We really don't want to renumber types because
        // those *are* referenced from the code section via `call_indirect`. To
        // avoid renumbering types, we do a first pass over this module's types
        // and build out a full type section with the same numbering as the
        // original module, and then append the state import's type at the end.
        let type_aliases = make_aliased_type_section(cx, info, 0);
        module.section(&type_aliases);
        let sections = make_state_import(cx, info);
        for s in sections {
            module.section(&s);
        }

        // Now rewrite the initial sections one at a time.
        //
        // Note that the initial sections can occur repeatedly and in any
        // order. This means that we only ever add, for example, `n` imports to
        // the rewritten module when a particular import section defines `n`
        // imports. We do *not* add all imports all at once. This is because
        // imports and aliases might be interleaved, and adding all imports all
        // at once could perturb entity numbering.
        let mut sections = info.raw_sections(cx).iter();
        let mut imports = info.imports(cx).iter();
        let mut instantiations = 0..info.instantiations(cx).len();
        let mut aliases = info.aliases(cx).iter();
        let mut first_non_initial_section = None;
        for section in sections.by_ref() {
            match section {
                // We handled this above.
                s if s.id == SectionId::Type.into() => continue,

                // These are handled in subsequent steps of this pre-order
                // traversal.
                s if s.id == SectionId::Module.into() => continue,

                s if s.id == SectionId::Import.into() => {
                    let count = wasmparser::ImportSectionReader::new(s.data, 0)
                        .unwrap()
                        .get_count();
                    let mut section = wasm_encoder::ImportSection::new();
                    for imp in imports.by_ref().take(usize::try_from(count).unwrap()) {
                        section.import(imp.module, imp.field, translate::entity_type(imp.ty));
                    }
                    module.section(&section);
                }

                // The actual instantiations are pulled out and handled in
                // `rewrite_instantiations` and then we get them here via the
                // state import. We need to bring them into scope via instance
                // export aliases, however.
                s if s.id == SectionId::Instance.into() => {
                    let count = wasmparser::InstanceSectionReader::new(s.data, 0)
                        .unwrap()
                        .get_count();
                    let mut section = wasm_encoder::AliasSection::new();
                    for idx in instantiations
                        .by_ref()
                        .take(usize::try_from(count).unwrap())
                    {
                        // Our imported state instance is always instance 0.
                        let from_instance = 0;
                        let name = format!("__wizer_instance_{}", idx);
                        section.instance_export(
                            from_instance,
                            wasm_encoder::ItemKind::Instance,
                            &name,
                        );
                    }
                    module.section(&section);
                }

                s if s.id == SectionId::Alias.into() => {
                    let count = wasmparser::AliasSectionReader::new(s.data, 0)
                        .unwrap()
                        .get_count();
                    let mut section = wasm_encoder::AliasSection::new();
                    for alias in aliases.by_ref().take(usize::try_from(count).unwrap()) {
                        match alias {
                            // We don't make any instantiations so we don't need
                            // modules here anymore.
                            wasmparser::Alias::OuterModule { .. } => continue,
                            // We already created a complete type section,
                            // including any aliases, above.
                            wasmparser::Alias::OuterType { .. } => continue,
                            // Copy over instance export aliases.
                            // however.
                            wasmparser::Alias::InstanceExport {
                                instance,
                                kind,
                                export,
                            } => {
                                // We need to add one to the instance's index
                                // because our state instance import shifted
                                // everything off by one.
                                let from_instance = instance + 1;
                                section.instance_export(
                                    from_instance,
                                    translate::item_kind(*kind),
                                    export,
                                );
                            }
                        }
                    }
                    module.section(&section);
                }

                s => {
                    assert!(first_non_initial_section.is_none());
                    first_non_initial_section = Some(s);
                    break;
                }
            }
        }

        // We don't define the memories from the original memory section, but we
        // do add instance export aliases for each of them from our imported
        // state instance. These aliases need to be in an alias section, which
        // is an initial section and must come before the rest of the
        // non-initial sections. But it must also come *after* any memories that
        // might have been imported, so that we don't mess up the
        // numbering. Therefore we add these aliases here, after we've processed
        // the initial sections, but before we start with the rest of the
        // sections.
        if info.defined_memories_index(cx).is_some() {
            let mut section = wasm_encoder::AliasSection::new();
            for (i, _) in info.defined_memories(cx).enumerate() {
                // Our state instance is always instance 0.
                let from_instance = 0;
                let name = format!("__wizer_memory_{}", i);
                section.instance_export(from_instance, wasm_encoder::ItemKind::Memory, &name);
            }
            module.section(&section);
        }

        // Globals are handled the same way as memories.
        if info.defined_globals_index(cx).is_some() {
            let mut section = wasm_encoder::AliasSection::new();
            for (i, _) in info.defined_globals(cx).enumerate() {
                let from_instance = 0;
                let name = format!("__wizer_global_{}", i);
                section.instance_export(from_instance, wasm_encoder::ItemKind::Global, &name);
            }
            module.section(&section);
        }

        // Process the rest of the non-initial sections.
        for section in first_non_initial_section.into_iter().chain(sections) {
            match section {
                // We replaced these with instance export aliases from our state
                // instance above.
                s if s.id == SectionId::Memory.into() => continue,
                s if s.id == SectionId::Global.into() => continue,

                // We ignore the original data segments. We don't define
                // memories anymore and state instances will define their own
                // data segments based on the snapshot.
                s if s.id == SectionId::Data.into() => continue,
                s if s.id == SectionId::DataCount.into() => continue,

                // The start function has already been run!
                s if s.id == SectionId::Start.into() => continue,

                // Finally, everything else is copied over as-is!
                s => {
                    module.section(s);
                }
            }
        }

        code_modules.module(&module);
        num_code_modules += 1;
    });

    (code_modules, num_code_modules)
}

/// Make the equivalent of the given module's type section from outer type
/// aliases that bring types from the umbrella module's types space into this
/// code module's types space.
fn make_aliased_type_section(
    cx: &ModuleContext<'_>,
    module: Module,
    depth: u32,
) -> wasm_encoder::AliasSection {
    let mut aliases = wasm_encoder::AliasSection::new();
    for ty in module.types(cx) {
        aliases.outer_type(depth, ty.index());
    }
    aliases
}

/// Make an import section that imports a code module's state instance import.
fn make_state_import(
    cx: &mut ModuleContext<'_>,
    module: Module,
) -> Vec<impl wasm_encoder::Section> {
    let mut sections = vec![];

    // Define the state instance's type.
    let state_instance_ty = module.define_state_instance_type(cx);

    // Alias the state instance type from the umbrella.
    let mut alias = wasm_encoder::AliasSection::new();
    alias.outer_type(0, state_instance_ty.index());
    sections.push(StateImportSection::Alias(alias));

    let state_instance_type_index = u32::try_from(module.types(cx).len()).unwrap();
    module.push_aliased_type(cx, state_instance_ty);

    // Define the import of the state instance, using the type
    // we just defined.
    let mut imports = wasm_encoder::ImportSection::new();
    imports.import(
        "__wizer_state",
        None,
        wasm_encoder::EntityType::Instance(state_instance_type_index),
    );
    sections.push(StateImportSection::Import(imports));

    return sections;

    enum StateImportSection {
        Alias(wasm_encoder::AliasSection),
        Import(wasm_encoder::ImportSection),
    }

    impl wasm_encoder::Section for StateImportSection {
        fn id(&self) -> u8 {
            match self {
                StateImportSection::Alias(s) => s.id(),
                StateImportSection::Import(s) => s.id(),
            }
        }

        fn encode<S>(&self, sink: &mut S)
        where
            S: Extend<u8>,
        {
            match self {
                StateImportSection::Alias(s) => s.encode(sink),
                StateImportSection::Import(s) => s.encode(sink),
            }
        }
    }
}

/// Create the state module the given module instantiation and recursively do
/// the same for its nested instantiations.
fn rewrite_state_module(
    cx: &ModuleContext<'_>,
    store: &crate::Store,
    info: Module,
    snapshot: &Snapshot,
    depth: u32,
) -> wasm_encoder::Module {
    let mut state_module = wasm_encoder::Module::new();
    let mut exports = wasm_encoder::ExportSection::new();

    // If there are nested instantiations, then define the nested state
    // modules and then instantiate them.
    assert_eq!(info.instantiations(cx).len(), snapshot.instantiations.len());
    if !snapshot.instantiations.is_empty() {
        // We create nested instantiations such that each state module has
        // the following module index space:
        //
        // [
        //     alias instantiation[0]'s code module,
        //     alias instantiation[1]'s code module,
        //     ...
        //     alias instantiation[N]'s code module,
        //     define instantiation[0]'s state module,
        //     define instantiation[1]'s state module,
        //     ...
        //     define instantiation[N]'s state module,
        // ]
        //
        // That is, the `i`th nested instantiation's code module is the `i`th
        // module in the index space, and its state module is at index `N+i`.
        //
        // We define all the code module aliases up front. Nested instantiations
        // may require aliasing instance exports from earlier instantiations, so
        // we interleave those in the same order that they appeared in the
        // original Wasm binary below.
        let mut alias_section = wasm_encoder::AliasSection::new();
        for (module, _) in info.instantiations(cx).values() {
            let module = module.get_aliased(cx).unwrap_or(*module);

            // Because we flatten the code modules into the umbrella module with
            // a pre-order traversal, this instantiation's code module is the
            // `module.pre_order_index()`th module in the root module's module
            // index space.
            let code_module_index_in_root = module.pre_order_index();
            alias_section.outer_module(depth, code_module_index_in_root);
        }
        state_module.section(&alias_section);

        // The instance index space is more complicated than the module index
        // space because of potential instance imports and aliasing imported
        // instance's exported nested instances. These imported/aliased
        // instances can then be used as arguments to a nested instantiation,
        // and then the resulting instance can also be used as an argument to
        // further nested instantiations. To handle all this, we use a
        // `Renumbering` map for tracking instance indices.
        let mut instance_renumbering = Renumbering::default();

        let aliased_types = make_aliased_type_section(cx, info, depth);
        state_module.section(&aliased_types);

        let mut instance_import_counts = info.instance_import_counts(cx).iter().copied();
        let mut aliases = info.aliases(cx).iter();
        let mut instantiations = info.instantiations(cx).values().enumerate();

        for section in info.initial_sections(cx) {
            match section {
                // Handled above.
                s if s.id == SectionId::Type.into() => continue,

                // Copy the imports over and update our renumbering for any
                // imported instances.
                s if s.id == SectionId::Import.into() => {
                    state_module.section(s);
                    let instance_import_count = instance_import_counts.next().unwrap();
                    for _ in 0..instance_import_count {
                        instance_renumbering.add_import();
                    }
                }

                // Update instance export aliases to use the numbered instance
                // indices. Also update the renumbering for any aliased
                // instances brought into scope.
                s if s.id == SectionId::Alias.into() => {
                    let count = wasmparser::AliasSectionReader::new(s.data, 0)
                        .unwrap()
                        .get_count();
                    let mut section = wasm_encoder::AliasSection::new();
                    for alias in aliases.by_ref().take(usize::try_from(count).unwrap()) {
                        match alias {
                            wasmparser::Alias::InstanceExport {
                                instance,
                                kind,
                                export,
                            } => {
                                section.instance_export(
                                    instance_renumbering.lookup(*instance),
                                    translate::item_kind(*kind),
                                    export,
                                );
                                // If this brought a new instance into our
                                // instance index space, update our renumbering
                                // map.
                                if let wasmparser::ExternalKind::Instance = kind {
                                    instance_renumbering.add_alias();
                                }
                            }
                            // Handled by `make_complete_type_section`.
                            wasmparser::Alias::OuterType { .. } => continue,
                            // Ignore these because we alias only the modules we
                            // need for nested instantiations below.
                            wasmparser::Alias::OuterModule { .. } => continue,
                        }
                    }
                    state_module.section(&section);
                }

                // We alias only the modules we need for nested instantiations
                // below.
                s if s.id == SectionId::Module.into() => continue,

                // For each nested instantiation in this section, alias its code
                // module, define its state module, instantiate the state module
                // to create the state instance, instantiate the code module
                // with the state instance, and finally export the code+state
                // instance.
                s if s.id == SectionId::Instance.into() => {
                    let count = wasmparser::InstanceSectionReader::new(s.data, 0)
                        .unwrap()
                        .get_count();
                    let mut instance_section = wasm_encoder::InstanceSection::new();
                    let mut module_section = wasm_encoder::ModuleSection::new();
                    for (i, (module, instance_args)) in instantiations
                        .by_ref()
                        .take(usize::try_from(count).unwrap())
                    {
                        // Define the state module for this instantiation.
                        let state_module = rewrite_state_module(
                            cx,
                            store,
                            *module,
                            &snapshot.instantiations[i],
                            depth + 1,
                        );
                        module_section.module(&state_module);

                        // Instantiate the state module to create the state
                        // instance.
                        let args: Vec<_> = instance_args
                            .iter()
                            .map(|arg| {
                                let mut arg = translate::instance_arg(arg);
                                if let (_name, wasm_encoder::Export::Instance(ref mut index)) = arg
                                {
                                    *index = instance_renumbering.lookup(*index);
                                }
                                arg
                            })
                            .collect();
                        instance_section.instantiate(
                            u32::try_from(snapshot.instantiations.len() + i).unwrap(),
                            args,
                        );
                        let state_instance_index = instance_renumbering.define_new();

                        // Then instantiate the associated code module, passing it this
                        // state instance and whatever other arguments it expects.
                        let args: Vec<_> = iter::once((
                            "__wizer_state",
                            wasm_encoder::Export::Instance(state_instance_index),
                        ))
                        .chain(instance_args.iter().map(|arg| {
                            let mut arg = translate::instance_arg(arg);
                            if let (_name, wasm_encoder::Export::Instance(ref mut index)) = arg {
                                *index = instance_renumbering.lookup(*index);
                            }
                            arg
                        }))
                        .collect();
                        instance_section.instantiate(u32::try_from(i).unwrap(), args);
                        let (_, code_and_state_instance_index) = instance_renumbering.define_both();

                        // Add the export for this nested instance.
                        let name = format!("__wizer_instance_{}", i);
                        exports.export(
                            &name,
                            wasm_encoder::Export::Instance(
                                u32::try_from(code_and_state_instance_index).unwrap(),
                            ),
                        );
                    }
                    state_module.section(&module_section);
                    state_module.section(&instance_section);
                }

                _ => unreachable!(),
            }
        }
    }

    // Add defined memories.
    assert_eq!(info.defined_memories_len(cx), snapshot.memory_mins.len());
    if info.defined_memories_index(cx).is_some() {
        let mut memories = wasm_encoder::MemorySection::new();
        for (i, (new_min, (_, mem))) in snapshot
            .memory_mins
            .iter()
            .copied()
            .zip(info.defined_memories(cx))
            .enumerate()
        {
            let mut mem = translate::memory_type(mem);
            assert!(new_min >= mem.minimum);
            assert!(new_min <= mem.maximum.unwrap_or(u64::MAX));
            mem.minimum = new_min;
            memories.memory(mem);

            let name = format!("__wizer_memory_{}", i);
            exports.export(
                &name,
                wasm_encoder::Export::Memory(u32::try_from(i).unwrap()),
            );
        }
        state_module.section(&memories);
    }

    // Add defined globals.
    assert_eq!(info.defined_globals_len(cx), snapshot.globals.len());
    if info.defined_globals_index(cx).is_some() {
        let mut globals = wasm_encoder::GlobalSection::new();
        for (i, (val, (_, glob_ty))) in snapshot
            .globals
            .iter()
            .zip(info.defined_globals(cx))
            .enumerate()
        {
            let glob_ty = translate::global_type(glob_ty);
            globals.global(
                glob_ty,
                match val {
                    wasmtime::Val::I32(x) => wasm_encoder::Instruction::I32Const(*x),
                    wasmtime::Val::I64(x) => wasm_encoder::Instruction::I64Const(*x),
                    wasmtime::Val::F32(x) => {
                        wasm_encoder::Instruction::F32Const(f32::from_bits(*x))
                    }
                    wasmtime::Val::F64(x) => {
                        wasm_encoder::Instruction::F64Const(f64::from_bits(*x))
                    }
                    _ => unreachable!(),
                },
            );

            let name = format!("__wizer_global_{}", i);
            exports.export(
                &name,
                wasm_encoder::Export::Global(u32::try_from(i).unwrap()),
            );
        }
        state_module.section(&globals);
    }

    state_module.section(&exports);

    // Add data segments.
    if !snapshot.data_segments.is_empty() {
        let mut data = wasm_encoder::DataSection::new();
        for seg in &snapshot.data_segments {
            data.active(
                seg.memory_index,
                wasm_encoder::Instruction::I32Const(seg.offset as i32),
                seg.data(store).iter().copied(),
            );
        }
        state_module.section(&data);
    }

    state_module
}
