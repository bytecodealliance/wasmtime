use crate::info::{
    types_interner::{EntityType, InstanceType, Type, TypeId},
    Module, ModuleContext,
};
use crate::stack_ext::StackExt;
use anyhow::{Context, Result};
use std::convert::TryFrom;
use wasm_encoder::SectionId;
use wasmparser::{SectionReader, SectionWithLimitedItems};

struct StackEntry {
    parser: wasmparser::Parser,
    module: Module,
}

/// Parse the given Wasm bytes into a `ModuleInfo` tree.
pub(crate) fn parse<'a>(full_wasm: &'a [u8]) -> anyhow::Result<ModuleContext<'a>> {
    log::debug!("Parsing the input Wasm");

    let mut cx = ModuleContext::new();

    // The wasm we are currently parsing. This is advanced as the parser
    // consumes input.
    let mut wasm = full_wasm;

    let mut stack = vec![StackEntry {
        parser: wasmparser::Parser::new(0),
        module: cx.root(),
    }];

    loop {
        let (payload, consumed) = match stack
            .top_mut()
            .parser
            .parse(wasm, true)
            .context("failed to parse Wasm")?
        {
            wasmparser::Chunk::NeedMoreData(_) => unreachable!(),
            wasmparser::Chunk::Parsed { payload, consumed } => (payload, consumed),
        };
        wasm = &wasm[consumed..];

        use wasmparser::Payload::*;
        match payload {
            Version { .. } => {}
            TypeSection(types) => type_section(&mut cx, &mut stack, full_wasm, types)?,
            ImportSection(imports) => import_section(&mut cx, &mut stack, full_wasm, imports)?,
            AliasSection(aliases) => alias_section(&mut cx, &mut stack, full_wasm, aliases)?,
            InstanceSection(instances) => {
                instance_section(&mut cx, &mut stack, full_wasm, instances)?
            }
            ModuleSectionStart {
                range,
                size: _,
                count: _,
            } => {
                stack.top_mut().module.add_raw_section(
                    &mut cx,
                    SectionId::Module,
                    range,
                    full_wasm,
                );
            }
            ModuleSectionEntry { parser, range: _ } => {
                stack.push(StackEntry {
                    parser,
                    module: Module::new_defined(&mut cx),
                });
            }
            FunctionSection(funcs) => function_section(&mut cx, &mut stack, full_wasm, funcs)?,
            TableSection(tables) => table_section(&mut cx, &mut stack, full_wasm, tables)?,
            MemorySection(mems) => memory_section(&mut cx, &mut stack, full_wasm, mems)?,
            GlobalSection(globals) => global_section(&mut cx, &mut stack, full_wasm, globals)?,
            ExportSection(exports) => export_section(&mut cx, &mut stack, full_wasm, exports)?,
            StartSection { func: _, range } => {
                stack
                    .top_mut()
                    .module
                    .add_raw_section(&mut cx, SectionId::Start, range, full_wasm)
            }
            ElementSection(elems) => stack.top_mut().module.add_raw_section(
                &mut cx,
                SectionId::Element,
                elems.range(),
                full_wasm,
            ),
            DataCountSection { range, .. } => stack.top_mut().module.add_raw_section(
                &mut cx,
                SectionId::DataCount,
                range,
                full_wasm,
            ),
            DataSection(data) => stack.top_mut().module.add_raw_section(
                &mut cx,
                SectionId::Data,
                data.range(),
                full_wasm,
            ),
            CustomSection { range, .. } => {
                stack
                    .top_mut()
                    .module
                    .add_raw_section(&mut cx, SectionId::Custom, range, full_wasm)
            }
            CodeSectionStart {
                range,
                count: _,
                size,
            } => {
                wasm = &wasm[usize::try_from(size).unwrap()..];
                let entry = stack.top_mut();
                entry.parser.skip_section();
                entry
                    .module
                    .add_raw_section(&mut cx, SectionId::Code, range, full_wasm)
            }
            CodeSectionEntry(_) => unreachable!(),
            UnknownSection { .. } => anyhow::bail!("unknown section"),
            EventSection(_) => anyhow::bail!("exceptions are not supported yet"),
            End => {
                let entry = stack.pop().unwrap();

                // If we finished parsing the root Wasm module, then we're done.
                if entry.module.is_root() {
                    assert!(stack.is_empty());
                    return Ok(cx);
                }

                // Otherwise, we need to add this module to its parent's module
                // section.
                let parent = stack.top_mut();
                parent.module.push_child_module(&mut cx, entry.module);
            }
        }
    }
}

fn type_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    mut types: wasmparser::TypeSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Type, types.range(), full_wasm);

    // Parse out types, as we will need them later when processing
    // instance imports.
    let count = usize::try_from(types.get_count()).unwrap();
    for _ in 0..count {
        let ty = types.read()?;
        match ty {
            wasmparser::TypeDef::Func(_) | wasmparser::TypeDef::Instance(_) => {
                module.push_type(cx, ty);
            }

            // We need to disallow module imports, even within nested modules
            // that only ever have other nested modules supplied as
            // arguments. Two different modules could be supplied for two
            // different instantiations of the module-importing module, but then
            // after we export all modules' globals in our instrumentation
            // phase, those two different module arguments could become
            // type-incompatible with each other:
            //
            // ```
            // (module
            //
            //   ;; Module A exports and `f` function. Internally it has
            //   ;; one global.
            //   (module $A
            //     (global $g ...)
            //     (func (export "f") ...))
            //
            //   ;; Module B has an identical interface as A. Internally
            //   ;; it has two globals.
            //   (module $B
            //     (global $g ...)
            //     (global $h ...)
            //     (func (export "f") ...))
            //
            //   ;; Module C imports any module that exports an `f`
            //   ;; function. It instantiates this imported module.
            //   (module $C
            //     (import "env" "module"
            //       (module (export "f" (func)))
            //     (instance 0)))
            //
            //   ;; C is instantiated with both A and B.
            //   (instance $C (import "env" "module" $A))
            //   (instance $C (import "env" "module" $B))
            // )
            // ```
            //
            // After this instrumentation pass, we need to make module C
            // transitively export all of the globals from its inner
            // instances. Which means that the module type used in the module
            // import needs to specify how many modules are in the imported
            // module, but in our two instantiations, we have two different
            // numbers of globals defined in each module! The only way to
            // resolve this would be to duplicate and specialize module C for
            // each instantiation, which we don't want to do for complexity and
            // code size reasons.
            //
            // Since module types are only used with importing and exporting
            // modules, which we don't intend to support as described above, we
            // can disallow module types to reject all of them in one fell
            // swoop.
            wasmparser::TypeDef::Module(_) => Err(anyhow::anyhow!(
                "wizer does not support importing or exporting modules"
            )
            .context("module types are not supported"))?,
        }
    }

    Ok(())
}

fn import_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    mut imports: wasmparser::ImportSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    stack
        .top_mut()
        .module
        .add_raw_section(cx, SectionId::Import, imports.range(), full_wasm);

    let mut instance_import_count = 0;

    // Two-level imports implicitly create an instance import. That is, this
    //
    //    (import "env" "f" (func))
    //    (import "env" "g" (func))
    //
    // is implicitly translated into roughly
    //
    //    (import "env" (instance (export "f" (func))
    //                            (export "g" (func))))
    //    (alias 0 "f")
    //    (alias 0 "g")
    //
    // However not that this is _not_ a WAT-level desugaring where we only have
    // to deal with the expanded form! We have to perform this translation
    // ourselves as we parse the imports.
    //
    // This variable keeps track of the implicit instance import that we are
    // currently building. Whenever we see a consecutive run of two-level
    // imports for the same module, we coalesce them into an implicit instance
    // import.
    let mut implicit_instance_import: Option<(&str, InstanceType)> = None;

    // Check that we can properly handle all imports.
    let count = imports.get_count();
    for _ in 0..count {
        let imp = imports.read()?;

        if imp.module.starts_with("__wizer_")
            || imp.field.map_or(false, |f| f.starts_with("__wizer_"))
        {
            anyhow::bail!(
                "input Wasm module already imports entities named with the `__wizer_*` prefix"
            );
        }

        match (implicit_instance_import.as_mut(), imp.field) {
            (Some((implicit_module, instance_ty)), Some(field))
                if *implicit_module == imp.module =>
            {
                let ty = module.entity_type(cx, imp.ty);
                let old = instance_ty.exports.insert(field.into(), ty);
                debug_assert!(old.is_none(), "checked by validation");
            }
            _ => {
                if let Some((_, instance_ty)) = implicit_instance_import.take() {
                    module.push_implicit_instance(cx, instance_ty);
                    instance_import_count += 1;
                }
                if let Some(field) = imp.field {
                    let field_ty = module.entity_type(cx, imp.ty);
                    let instance_ty = InstanceType {
                        exports: Some((field.into(), field_ty)).into_iter().collect(),
                    };
                    implicit_instance_import = Some((imp.module, instance_ty));
                }
            }
        }

        check_import_type(
            cx,
            stack.top().module.types(cx),
            stack.top().module.is_root(),
            &module.entity_type(cx, imp.ty),
        )?;
        if let wasmparser::ImportSectionEntryType::Instance(_) = imp.ty {
            instance_import_count += 1;
        }
        module.push_import(cx, imp);
    }

    if let Some((_, instance_ty)) = implicit_instance_import.take() {
        module.push_implicit_instance(cx, instance_ty);
        instance_import_count += 1;
    }

    module.push_instance_import_count(cx, instance_import_count);
    Ok(())
}

fn check_import_type(
    cx: &ModuleContext,
    types: &[TypeId],
    is_root: bool,
    ty: &EntityType,
) -> Result<()> {
    match ty {
        EntityType::Function(_) => Ok(()),
        EntityType::Instance(inst_ty) => {
            // We allow importing instances that only export things that are
            // acceptable imports. This is equivalent to a two-layer import.
            match cx.types().get(*inst_ty) {
                Type::Instance(inst_ty) => {
                    for ty in inst_ty.exports.values() {
                        check_import_type(cx, types, is_root, ty)?;
                    }
                    Ok(())
                }
                _ => unreachable!(),
            }
        }
        EntityType::Memory(mem_ty) => match mem_ty {
            wasmparser::MemoryType::M32 { limits: _, shared } => {
                anyhow::ensure!(!shared, "shared memories are not supported by Wizer yet");
                anyhow::ensure!(
                    !is_root,
                    "memory imports are not allowed in the root Wasm module"
                );
                Ok(())
            }
            wasmparser::MemoryType::M64 { .. } => {
                anyhow::bail!("the memory64 proposal is not supported by Wizer yet")
            }
        },
        EntityType::Table(_) | EntityType::Global(_) => {
            anyhow::ensure!(
                !is_root,
                "table and global imports are not allowed in the root Wasm module"
            );
            Ok(())
        }
        EntityType::Module(_) => {
            unreachable!();
        }
    }
}

fn alias_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    mut aliases: wasmparser::AliasSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Alias, aliases.range(), full_wasm);

    // Clone any aliases over into this module's index spaces.
    for _ in 0..aliases.get_count() {
        let alias = aliases.read()?;
        match &alias {
            wasmparser::Alias::OuterType {
                relative_depth,
                index,
            } => {
                let relative_depth = usize::try_from(*relative_depth).unwrap();
                // NB: `- 2` rather than `- 1` because
                // `relative_depth=0` means this module's immediate
                // parent, not this module itself.
                let ty = stack[stack.len() - 2 - relative_depth]
                    .module
                    .type_id_at(cx, *index);
                module.push_aliased_type(cx, ty);
            }
            wasmparser::Alias::OuterModule {
                relative_depth,
                index,
            } => {
                let relative_depth = usize::try_from(*relative_depth).unwrap();
                // Ditto regarding `- 2`.
                let alias_of = stack[stack.len() - 2 - relative_depth]
                    .module
                    .child_module_at(cx, *index);
                let aliased = Module::new_aliased(cx, alias_of);
                module.push_child_module(cx, aliased);
            }
            wasmparser::Alias::InstanceExport {
                instance,
                kind,
                export,
            } => match kind {
                wasmparser::ExternalKind::Module => {
                    anyhow::bail!("exported modules are not supported yet")
                }
                wasmparser::ExternalKind::Instance => {
                    let inst_ty = match module.instance_export(cx, *instance, export) {
                        Some(EntityType::Instance(i)) => *i,
                        _ => unreachable!(),
                    };
                    module.push_aliased_instance(cx, inst_ty);
                }
                wasmparser::ExternalKind::Function => {
                    let func_ty = match module.instance_export(cx, *instance, export) {
                        Some(EntityType::Function(ty)) => *ty,
                        _ => unreachable!(),
                    };
                    module.push_function(cx, func_ty);
                }
                wasmparser::ExternalKind::Table => {
                    let table_ty = match module.instance_export(cx, *instance, export) {
                        Some(EntityType::Table(ty)) => *ty,
                        _ => unreachable!(),
                    };
                    module.push_table(cx, table_ty);
                }
                wasmparser::ExternalKind::Memory => {
                    let ty = match module.instance_export(cx, *instance, export) {
                        Some(EntityType::Memory(ty)) => *ty,
                        _ => unreachable!(),
                    };
                    module.push_imported_memory(cx, ty);
                }
                wasmparser::ExternalKind::Global => {
                    let ty = match module.instance_export(cx, *instance, export) {
                        Some(EntityType::Global(ty)) => *ty,
                        _ => unreachable!(),
                    };
                    module.push_imported_global(cx, ty);
                }
                wasmparser::ExternalKind::Event => {
                    unreachable!("validation should reject the exceptions proposal")
                }
                wasmparser::ExternalKind::Type => unreachable!("can't export types"),
            },
        }
        module.push_alias(cx, alias);
    }

    Ok(())
}

fn instance_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    mut instances: wasmparser::InstanceSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Instance, instances.range(), full_wasm);

    // Record the instantiations made in this module, and which modules were
    // instantiated.
    for _ in 0..instances.get_count() {
        let inst = instances.read()?;
        let module_index = inst.module();
        let child_module = module.child_module_at(cx, module_index);
        let inst_ty = child_module.define_instance_type(cx);

        let mut instance_args_reader = inst.args()?;
        let instance_args_count = usize::try_from(instance_args_reader.get_count()).unwrap();
        let mut instance_args = Vec::with_capacity(instance_args_count);
        for _ in 0..instance_args_count {
            instance_args.push(instance_args_reader.read()?);
        }

        module.push_defined_instance(cx, inst_ty, child_module, instance_args);
    }

    Ok(())
}

fn function_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    mut funcs: wasmparser::FunctionSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Function, funcs.range(), full_wasm);

    let count = usize::try_from(funcs.get_count()).unwrap();
    for _ in 0..count {
        let ty_idx = funcs.read()?;
        let ty = module.type_id_at(cx, ty_idx);
        module.push_function(cx, ty);
    }
    Ok(())
}

fn table_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    mut tables: wasmparser::TableSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Table, tables.range(), full_wasm);

    let count = usize::try_from(tables.get_count()).unwrap();
    for _ in 0..count {
        module.push_table(cx, tables.read()?);
    }
    Ok(())
}

fn memory_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    mut mems: wasmparser::MemorySectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Memory, mems.range(), full_wasm);

    let count = usize::try_from(mems.get_count()).unwrap();
    for _ in 0..count {
        let m = mems.read()?;
        module.push_defined_memory(cx, m);
    }
    Ok(())
}

fn global_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    mut globals: wasmparser::GlobalSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Global, globals.range(), full_wasm);

    let count = usize::try_from(globals.get_count()).unwrap();
    for _ in 0..count {
        let g = globals.read()?;
        module.push_defined_global(cx, g.ty);
    }
    Ok(())
}

fn export_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    mut exports: wasmparser::ExportSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Export, exports.range(), full_wasm);

    for _ in 0..exports.get_count() {
        let export = exports.read()?;

        if export.field.starts_with("__wizer_") {
            anyhow::bail!(
                "input Wasm module already exports entities named with the `__wizer_*` prefix"
            );
        }

        match export.kind {
            wasmparser::ExternalKind::Module => {
                anyhow::bail!("Wizer does not support importing and exporting modules")
            }
            wasmparser::ExternalKind::Type | wasmparser::ExternalKind::Event => {
                unreachable!("checked in validation")
            }
            wasmparser::ExternalKind::Function
            | wasmparser::ExternalKind::Table
            | wasmparser::ExternalKind::Memory
            | wasmparser::ExternalKind::Global
            | wasmparser::ExternalKind::Instance => {
                module.push_export(cx, export);
            }
        }
    }
    Ok(())
}
