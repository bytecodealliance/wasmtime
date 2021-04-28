//! Final rewrite pass.

mod renumbering;

use crate::{info::ModuleInfo, snapshot::Snapshot, translate, FuncRenames, Wizer};
use renumbering::Renumbering;
use std::{convert::TryFrom, iter};
use wasm_encoder::SectionId;

impl Wizer {
    /// Given the initialized snapshot, rewrite the Wasm so that it is already
    /// initialized.
    ///
    /// ## Code Shape
    ///
    /// With module linking, we rewrite each nested module into a *code module*
    /// that doesn't define any internal state (i.e. memories, globals, and
    /// nested instances) and imports a *state instance* that exports all of
    /// these things instead. For each instantiation of a nested module, we have
    /// a *state module* that defines the already-initialized state for that
    /// instantiation, we instantiate that state module to create one such state
    /// instance, and then use this as an import argument in the instantiation
    /// of the original code module. This way, we do not duplicate shared code
    /// bodies across multiple instantiations of the same module.
    ///
    /// Note that the root module is not split into a code module and state
    /// instance. We can, essentially, assume that there is only one instance of
    /// the root module, and rewrite it in place, without factoring the state
    /// out into a separate instance. This is because the root is not allowed to
    /// import any external state, so even if it were instantiated multiple
    /// times, it would still end up in the same place anyways. (This is kinda
    /// the whole reason why Wizer works at all.)
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
    /// (module $A
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
    ///   ;; State module for `$x`.
    ///   (module $x_state_module
    ///     (memory (export "__wizer_memory_0") (memory))
    ///     ;; Data segments to initialize the memory based on our snapshot
    ///     ;; would go here...
    ///
    ///     (global (export "__wizer_global_0")
    ///       (global (mut i32) (i32.const (; ...snapshot's initialized value goes here... ;)))
    ///     )
    ///   )
    ///
    ///   ;; State instance for `$x`.
    ///   (instance $x_state (instantiate $x_state_module))
    ///
    ///   ;; The instantiation of `$x` is now rewritten to use our state
    ///   ;; instance.
    ///   (instance $x (instantiate $B (import "__wizer_state" $x_state)))
    ///
    ///   ;; Same goes for the `$y` instantiation.
    ///   (module $y_state_module
    ///     (memory (export "__wizer_memory_0") (memory))
    ///     ;; Data segments...
    ///     (global (export "__wizer_global_0")
    ///       (global (mut i32) (i32.const (; ...snapshot's initialized value goes here... ;)))
    ///     )
    ///   )
    ///   (instance $y_state (instantiate $y_state_module))
    ///   (instance $y (instantiate $B (import "__wizer_state" $y_state)))
    ///
    ///   (memory $A_mem)
    ///   (global $A_glob (mut i32))
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
    /// the `ModuleInfo::id` we assigned during the instrumentation phase
    /// matches the module's place in the index space. The state modules,
    /// however, remain a nested tree, and we emit them in a traversal of the
    /// `Snapshot` instance tree. This is safe because, unlike code modules,
    /// each state module is only instantiated exactly once. The instantiations'
    /// references to nested modules become outer aliases pointing to the
    /// module's position in the parent's flat sequence of nested modules.
    pub(crate) fn rewrite(
        &self,
        snapshot: &Snapshot,
        info: &ModuleInfo,
        renames: &FuncRenames,
    ) -> Vec<u8> {
        log::debug!("Rewriting input Wasm to pre-initialized state");

        let mut root = wasm_encoder::Module::new();

        let types = make_complete_type_section(info);
        root.section(&types);

        let mut id_to_module_info = vec![];
        make_id_to_module_info(&mut id_to_module_info, info);

        let (code_modules, num_code_modules) = rewrite_code_modules(info, &id_to_module_info);
        // Only add the module section if there are multiple code modules,
        // so that we avoid introducing the module section when module
        // linking isn't in use.
        if num_code_modules > 0 {
            root.section(&code_modules);

            let state_modules =
                rewrite_state_modules(info, &id_to_module_info, &snapshot.instantiations);
            root.section(&state_modules);
        }

        self.rewrite_root(&mut root, info, snapshot, renames, num_code_modules);

        root.finish()
    }

    fn rewrite_root(
        &self,
        root: &mut wasm_encoder::Module,
        root_info: &ModuleInfo,
        snapshot: &Snapshot,
        renames: &FuncRenames,
        num_code_modules: u32,
    ) {
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
                    seg.data.iter().copied(),
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

        // A map from the original Wasm's instance numbering to the newly rewritten
        // instance numbering.
        let mut instance_renumbering = Renumbering::default();

        let mut instance_import_counts = root_info.instance_import_counts.iter().copied();
        let mut instantiations = root_info.instantiations.values().enumerate();
        let mut aliases = root_info.aliases.iter();

        for section in &root_info.raw_sections {
            match section {
                // Some tools expect the name custom section to come last, even
                // though custom sections are allowed in any order. Therefore,
                // make sure we've added our data section by now.
                s if is_name_section(s) => {
                    add_data_section(root);
                    root.section(s);
                }

                s if s.id == SectionId::Custom.into() => {
                    root.section(s);
                }

                // These were already added in `make_complete_type_section`.
                s if s.id == SectionId::Type.into() => {
                    continue;
                }

                // These were already taken care of in `rewrite_code_modules`.
                s if s.id == SectionId::Module.into() => {
                    continue;
                }

                // Import sections are just copied over, but we additionally
                // must make sure that our count of how many instances are
                // currently in this module's instance export space and map from
                // old instance numbering to new instance numbering are
                // correctly updated for any instances that were imported in
                // this section.
                s if s.id == SectionId::Import.into() => {
                    root.section(s);

                    let instance_import_count = instance_import_counts.next().unwrap();
                    for _ in 0..instance_import_count {
                        instance_renumbering.add_import();
                    }
                }

                // Instantiations from the original Wasm become two
                // instantiations in the rewritten Wasm:
                //
                // 1. First, we instantiate the state module for this instance
                //    to create the rewritten state instance.
                //
                // 2. Then, we instantiate this instance's code module, passing
                //    it the state instance and any other import arguments it
                //    originally had. This, finally, is the rewritten version of
                //    the original instance.
                //
                // Because there are two instances, where previously there was
                // one, we are forced to renumber the instance index space.
                s if s.id == SectionId::Instance.into() => {
                    let mut instances = wasm_encoder::InstanceSection::new();
                    let count = wasmparser::InstanceSectionReader::new(s.data, 0)
                        .unwrap()
                        .get_count();
                    for (nth_defined_inst, (module_id, instance_args)) in instantiations
                        .by_ref()
                        .take(usize::try_from(count).unwrap())
                    {
                        // Instantiate the state module.
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
                        instances.instantiate(
                            num_code_modules + u32::try_from(nth_defined_inst).unwrap(),
                            args,
                        );
                        let state_instance_index = instance_renumbering.define_new();

                        // Instantiate the code module with our state instance
                        // and the original import arguments.
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
                        instances.instantiate(module_id - 1, args);
                        instance_renumbering.define_both();
                    }
                    root.section(&instances);
                }

                // For the alias section, we update instance export aliases to
                // use the new instance numbering.
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
                            wasmparser::Alias::OuterType { .. }
                            | wasmparser::Alias::OuterModule { .. } => {
                                unreachable!(
                                    "the root can't alias any outer entities because there are \
                                     no entities outside the root module"
                                )
                            }
                        }
                    }
                    root.section(&section);
                }

                s if s.id == SectionId::Function.into() => {
                    root.section(s);
                }

                s if s.id == SectionId::Table.into() => {
                    root.section(s);
                }

                // For the memory section, we update the minimum size of each
                // defined memory to the snapshot's initialized size for that
                // memory.
                s if s.id == SectionId::Memory.into() => {
                    let mut memories = wasm_encoder::MemorySection::new();
                    assert_eq!(root_info.defined_memories_len(), snapshot.memory_mins.len());
                    for (mem, new_min) in root_info
                        .defined_memories()
                        .zip(snapshot.memory_mins.iter().copied())
                    {
                        let mut mem = translate::memory_type(mem);
                        mem.limits.min = new_min;
                        memories.memory(mem);
                    }
                    root.section(&memories);
                }

                // Encode the initialized global values from the snapshot,
                // rather than the original values.
                s if s.id == SectionId::Global.into() => {
                    let mut globals = wasm_encoder::GlobalSection::new();
                    for (glob_ty, val) in root_info.defined_globals().zip(snapshot.globals.iter()) {
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
                    root.section(&globals);
                }

                // Remove the initialization function's export and perform any
                // requested renames.
                s if s.id == SectionId::Export.into() => {
                    let mut exports = wasm_encoder::ExportSection::new();
                    for export in &root_info.exports {
                        if export.field == self.init_func {
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

                        let mut export = translate::export(export.kind, export.index);
                        if let wasm_encoder::Export::Instance(ref mut index) = export {
                            *index = instance_renumbering.lookup(*index);
                        }

                        exports.export(field, export);
                    }
                    root.section(&exports);
                }

                // Skip the `start` function -- it's already been run!
                s if s.id == SectionId::Start.into() => {
                    continue;
                }

                s if s.id == SectionId::Element.into() => {
                    root.section(s);
                }

                s if s.id == SectionId::Data.into() => {
                    // TODO: supporting bulk memory will require copying over
                    // any passive and declared segments.
                    add_data_section(root);
                }

                s if s.id == SectionId::Code.into() => {
                    root.section(s);
                }

                _ => unreachable!(),
            }
        }

        // Make sure that we've added our data section to the module.
        add_data_section(root);
    }
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
fn rewrite_code_modules(
    root_info: &ModuleInfo,
    id_to_module_info: &Vec<&ModuleInfo>,
) -> (wasm_encoder::ModuleSection, u32) {
    let mut modules = wasm_encoder::ModuleSection::new();
    let mut num_code_modules = 0;

    root_info.pre_order(|info| {
        // The root module is handled by `rewrite_root`; we are only dealing
        // with nested children here.
        if info.is_root() {
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
        let mut types = make_complete_type_section(info);
        let import = make_state_import(info, &mut types, id_to_module_info);
        module.section(&types);
        module.section(&import);

        // Now rewrite the initial sections one at a time.
        //
        // Note that the initial sections can occur repeatedly and in any
        // order. This means that we only ever add, for example, `n` imports to
        // the rewritten module when a particular import section defines `n`
        // imports. We do *not* add all imports all at once. This is because
        // imports and aliases might be interleaved, and adding all imports all
        // at once could perturb entity numbering.
        let mut sections = info.raw_sections.iter();
        let mut imports = info.imports.iter();
        let mut instantiations = 0..info.instantiations.len();
        let mut aliases = info.aliases.iter();
        let mut first_non_initial_section = None;
        for section in sections.by_ref() {
            match section {
                // We handled this in `make_complete_type_section` above.
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
        if let Some(defined_memories_index) = info.defined_memories_index {
            let mut section = wasm_encoder::AliasSection::new();
            let num_defined_memories =
                info.memories.len() - usize::try_from(defined_memories_index).unwrap();
            for mem in 0..num_defined_memories {
                // Our state instance is always instance 0.
                let from_instance = 0;
                let name = format!("__wizer_memory_{}", mem);
                section.instance_export(from_instance, wasm_encoder::ItemKind::Memory, &name);
            }
            module.section(&section);
        }

        // Globals are handled the same way as memories.
        if let Some(defined_globals_index) = info.defined_globals_index {
            let mut section = wasm_encoder::AliasSection::new();
            let num_defined_globals =
                info.globals.len() - usize::try_from(defined_globals_index).unwrap();
            for mem in 0..num_defined_globals {
                // Our state instance is always instance 0.
                let from_instance = 0;
                let name = format!("__wizer_global_{}", mem);
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

        modules.module(&module);
        num_code_modules += 1;
    });

    (modules, num_code_modules)
}

/// Flatten the `ModuleInfo` tree into a vector that maps each module id
/// (i.e. pre-order index) to the associated module info.
fn make_id_to_module_info<'a>(id_to_info: &mut Vec<&'a ModuleInfo<'a>>, info: &'a ModuleInfo<'a>) {
    debug_assert_eq!(u32::try_from(id_to_info.len()).unwrap(), info.id);
    id_to_info.push(info);
    for m in &info.modules {
        make_id_to_module_info(id_to_info, m);
    }
}

/// Make a single complete type section for the given module info, regardless of
/// how many initial type sections these types might have been defined within in
/// the original module's serialization.
fn make_complete_type_section(info: &ModuleInfo) -> wasm_encoder::TypeSection {
    let mut types = wasm_encoder::TypeSection::new();
    for ty in &info.types {
        match ty {
            wasmparser::TypeDef::Func(func_ty) => {
                types.function(
                    func_ty.params.iter().map(|ty| translate::val_type(*ty)),
                    func_ty.returns.iter().map(|ty| translate::val_type(*ty)),
                );
            }
            wasmparser::TypeDef::Instance(inst_ty) => {
                types.instance(
                    inst_ty
                        .exports
                        .iter()
                        .map(|e| (e.name, translate::entity_type(e.ty))),
                );
            }
            wasmparser::TypeDef::Module(_) => {
                unreachable!(
                    "we don't support importing/exporting modules so don't have to deal \
                     with module types"
                )
            }
        }
    }
    types
}

/// Make an import section that imports a code module's state instance import.
fn make_state_import(
    info: &ModuleInfo,
    types: &mut wasm_encoder::TypeSection,
    id_to_module_info: &Vec<&ModuleInfo>,
) -> wasm_encoder::ImportSection {
    let mut num_types = u32::try_from(info.types.len()).unwrap();

    // Define instance types for each of the instances that we
    // previously instantiated locally so that we can refer to
    // these types in the state instance's type.
    let instance_types = info
        .instantiations
        .values()
        .map(|(m, _)| {
            id_to_module_info[usize::try_from(*m).unwrap()]
                .define_instance_type(&mut num_types, types)
        })
        .collect::<Vec<_>>();

    // Define the state instance's type.
    let state_instance_exports = info
        .defined_globals()
        .enumerate()
        .map(|(i, g)| {
            (
                format!("__wizer_global_{}", i),
                wasm_encoder::EntityType::Global(translate::global_type(g)),
            )
        })
        .chain(info.defined_memories().enumerate().map(|(i, m)| {
            (
                format!("__wizer_memory_{}", i),
                wasm_encoder::EntityType::Memory(translate::memory_type(m)),
            )
        }))
        .chain(instance_types.iter().enumerate().map(|(i, type_index)| {
            (
                format!("__wizer_instance_{}", i),
                wasm_encoder::EntityType::Instance(*type_index),
            )
        }))
        .collect::<Vec<_>>();
    let state_instance_type_index = num_types;
    types.instance(
        state_instance_exports
            .iter()
            .map(|(name, e)| (name.as_str(), *e)),
    );

    // Define the import of the state instance, using the type
    // we just defined.
    let mut imports = wasm_encoder::ImportSection::new();
    imports.import(
        "__wizer_state",
        None,
        wasm_encoder::EntityType::Instance(state_instance_type_index),
    );
    imports
}

/// Define the state modules for each instantiation.
///
/// These are modules that just define the memories/globals/nested instances of
/// a particular instantiation and initialize them to the snapshot's state. They
/// have no imports and export all of their internal state entities.
///
/// This does *not* instantiate the state modules in the resulting module
/// section, just defines them (although nested state modules within these top
/// level-state modules are instantiated inside these top-level state
/// modules). That is because instantiation is handled differently depending on
/// if the instantiation happens directly inside the root module (see the
/// handling of instance sections in `rewrite_root`) or in a deeply nested
/// module (in which case it is instantiated by its parent state module,
/// i.e. another recursive invocation of this function that is one frame up the
/// stack).
fn rewrite_state_modules(
    info: &ModuleInfo,
    id_to_module_info: &Vec<&ModuleInfo>,
    snapshots: &[Snapshot],
) -> wasm_encoder::ModuleSection {
    let mut modules = wasm_encoder::ModuleSection::new();

    assert_eq!(snapshots.len(), info.instantiations.len());
    for (snapshot, (module_id, _)) in snapshots.iter().zip(info.instantiations.values()) {
        let module_info = &id_to_module_info[usize::try_from(*module_id).unwrap()];
        let state_module = rewrite_one_state_module(module_info, id_to_module_info, snapshot, 0);
        modules.module(&state_module);
    }

    modules
}

fn rewrite_one_state_module(
    info: &ModuleInfo,
    id_to_module_info: &Vec<&ModuleInfo>,
    snapshot: &Snapshot,
    depth: u32,
) -> wasm_encoder::Module {
    let mut state_module = wasm_encoder::Module::new();
    let mut exports = wasm_encoder::ExportSection::new();

    // If there are nested instantiations, then define the nested state
    // modules and then instantiate them.
    assert_eq!(info.instantiations.len(), snapshot.instantiations.len());
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
        // The instance index space is more complicated because of potential
        // instance imports and aliasing imported instance's exported nested
        // instances. These imported/aliased instances can then be used as
        // arguments to a nested instantiation, and then the resulting instance
        // can also be used as an argument to further nested instantiations. To
        // handle all this, we use a `Renumbering` map for tracking instance
        // indices.
        let mut instance_renumbering = Renumbering::default();

        let types = make_complete_type_section(&info);
        state_module.section(&types);

        let mut instance_import_counts = info.instance_import_counts.iter().copied();
        let mut aliases = info.aliases.iter();
        let mut instantiations = info.instantiations.values().enumerate();

        for section in info.initial_sections() {
            match section {
                // Handled by `make_complete_type_section` above.
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
                    let mut alias_section = wasm_encoder::AliasSection::new();
                    let mut instance_section = wasm_encoder::InstanceSection::new();
                    let mut module_section = wasm_encoder::ModuleSection::new();
                    for (i, (module_id, instance_args)) in instantiations
                        .by_ref()
                        .take(usize::try_from(count).unwrap())
                    {
                        // Alias this instantiation's code module.
                        //
                        // Because we flatten the code modules into the root
                        // with a pre-order traversal, the module id is the
                        // module's pre-order index, and the root module is not
                        // in the flattened list, this instantiation's code
                        // module is the `module_id - 1`th module in the root
                        // module's module index space.
                        let root_module_index = *module_id - 1;
                        alias_section.outer_module(depth, root_module_index);

                        // Define the state module for this instantiation.
                        let state_module = rewrite_one_state_module(
                            id_to_module_info[usize::try_from(*module_id).unwrap()],
                            id_to_module_info,
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
                    state_module.section(&alias_section);
                    state_module.section(&module_section);
                    state_module.section(&instance_section);
                }

                _ => unreachable!(),
            }
        }
    }

    // Add defined memories.
    assert_eq!(info.defined_memories_len(), snapshot.memory_mins.len());
    if info.defined_memories_index.is_some() {
        let mut memories = wasm_encoder::MemorySection::new();
        for (i, (new_min, mem)) in snapshot
            .memory_mins
            .iter()
            .copied()
            .zip(info.defined_memories())
            .enumerate()
        {
            let mut mem = translate::memory_type(mem);
            assert!(new_min >= mem.limits.min);
            assert!(new_min <= mem.limits.max.unwrap_or(u32::MAX));
            mem.limits.min = new_min;
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
    assert_eq!(info.defined_globals_len(), snapshot.globals.len());
    if info.defined_globals_index.is_some() {
        let mut globals = wasm_encoder::GlobalSection::new();
        for (i, (val, glob_ty)) in snapshot
            .globals
            .iter()
            .zip(info.defined_globals())
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
                seg.data.iter().copied(),
            );
        }
        state_module.section(&data);
    }

    state_module
}
