use crate::info::ModuleInfo;
use crate::stack_ext::StackExt;
use anyhow::{Context, Result};
use std::convert::TryFrom;
use wasm_encoder::SectionId;
use wasmparser::{SectionReader, SectionWithLimitedItems};

/// Parse the given Wasm bytes into a `ModuleInfo` tree.
pub(crate) fn parse<'a>(full_wasm: &'a [u8]) -> anyhow::Result<ModuleInfo<'a>> {
    log::debug!("Parsing the input Wasm");

    // The wasm we are currently parsing. This is advanced as the parser
    // consumes input.
    let mut wasm = full_wasm;

    // The counter for making unique-within-the-whole-bundle module identifiers
    // (i.e. the module's pre-order traversal index).
    let mut id_counter = 0;

    // The stack of module infos we are parsing. As we visit inner modules
    // during parsing, we push new entries, and when we finish processing them,
    // we pop them.
    let mut stack = vec![ModuleInfo::for_root()];

    // Stack of parsers for each module we are parsing. Has a parallel structure
    // to `stack`.
    let mut parsers = vec![wasmparser::Parser::new(0)];

    loop {
        assert_eq!(stack.len(), parsers.len());

        let (payload, consumed) = match parsers
            .top_mut()
            .parse(wasm, true)
            .context("failed to parse Wasm")?
        {
            wasmparser::Chunk::NeedMoreData(_) => anyhow::bail!("invalid Wasm module"),
            wasmparser::Chunk::Parsed { payload, consumed } => (payload, consumed),
        };
        wasm = &wasm[consumed..];

        use wasmparser::Payload::*;
        match payload {
            Version { .. } => continue,
            TypeSection(types) => type_section(&mut stack, full_wasm, types)?,
            ImportSection(imports) => import_section(&mut stack, full_wasm, imports)?,
            AliasSection(aliases) => alias_section(&mut stack, full_wasm, aliases)?,
            InstanceSection(instances) => instance_section(&mut stack, full_wasm, instances)?,
            ModuleSectionStart {
                range,
                size: _,
                count: _,
            } => {
                let info = stack.top_mut();
                info.add_raw_section(SectionId::Module, range, full_wasm);
            }
            ModuleSectionEntry { parser, range: _ } => {
                id_counter += 1;
                stack.push(ModuleInfo::for_inner(id_counter));
                parsers.push(parser);
            }
            FunctionSection(funcs) => function_section(&mut stack, full_wasm, funcs)?,
            TableSection(tables) => table_section(&mut stack, full_wasm, tables)?,
            MemorySection(mems) => memory_section(&mut stack, full_wasm, mems)?,
            GlobalSection(globals) => global_section(&mut stack, full_wasm, globals)?,
            ExportSection(exports) => export_section(&mut stack, full_wasm, exports)?,
            StartSection { func: _, range } => {
                stack
                    .top_mut()
                    .add_raw_section(SectionId::Start, range, full_wasm)
            }
            ElementSection(elems) => {
                stack
                    .top_mut()
                    .add_raw_section(SectionId::Element, elems.range(), full_wasm)
            }
            DataCountSection { .. } => unreachable!("validation rejects bulk memory"),
            DataSection(data) => {
                stack
                    .top_mut()
                    .add_raw_section(SectionId::Data, data.range(), full_wasm)
            }
            CustomSection { range, .. } => {
                stack
                    .top_mut()
                    .add_raw_section(SectionId::Custom, range, full_wasm)
            }
            CodeSectionStart {
                range,
                count: _,
                size,
            } => {
                parsers.top_mut().skip_section();
                wasm = &wasm[usize::try_from(size).unwrap()..];
                stack
                    .top_mut()
                    .add_raw_section(SectionId::Code, range, full_wasm)
            }
            CodeSectionEntry(_) => unreachable!(),
            UnknownSection { .. } => anyhow::bail!("unknown section"),
            EventSection(_) => anyhow::bail!("exceptions are not supported yet"),
            End => {
                let info = stack.pop().unwrap();
                parsers.pop();

                // If we finished parsing the root Wasm module, then we're done.
                if info.is_root() {
                    assert!(stack.is_empty());
                    assert!(parsers.is_empty());
                    return Ok(info);
                }

                // Otherwise, we need to add this module to its parent's module
                // section.
                let parent = stack.top_mut();
                parent.modules.push(info);
            }
        }
    }
}

fn type_section<'a>(
    stack: &mut Vec<ModuleInfo<'a>>,
    full_wasm: &'a [u8],
    mut types: wasmparser::TypeSectionReader<'a>,
) -> anyhow::Result<()> {
    let info = stack.top_mut();
    info.add_raw_section(SectionId::Type, types.range(), full_wasm);

    // Parse out types, as we will need them later when processing
    // instance imports.
    let count = usize::try_from(types.get_count()).unwrap();
    info.types.reserve(count);
    for _ in 0..count {
        let ty = types.read()?;
        match ty {
            wasmparser::TypeDef::Func(_) | wasmparser::TypeDef::Instance(_) => {
                info.types.push(ty);
            }
            wasmparser::TypeDef::Module(_) => Err(anyhow::anyhow!(
                "wizer does not support importing or exporting modules"
            )
            .context("module types are not supported"))?,
        }
    }

    Ok(())
}

fn import_section<'a>(
    stack: &mut Vec<ModuleInfo<'a>>,
    full_wasm: &'a [u8],
    mut imports: wasmparser::ImportSectionReader<'a>,
) -> anyhow::Result<()> {
    stack
        .top_mut()
        .add_raw_section(SectionId::Import, imports.range(), full_wasm);

    let mut instance_import_count = 0;

    // Check that we can properly handle all imports.
    let count = imports.get_count();
    for _ in 0..count {
        let imp = imports.read()?;
        stack.top_mut().imports.push(imp);
        check_import_type(&stack.top().types, stack.top().is_root(), &imp.ty)?;

        if imp.module.starts_with("__wizer_")
            || imp.field.map_or(false, |f| f.starts_with("__wizer_"))
        {
            anyhow::bail!(
                "input Wasm module already imports entities named with the `__wizer_*` prefix"
            );
        }

        // Add the import to the appropriate index space for our current module.
        match imp.ty {
            wasmparser::ImportSectionEntryType::Memory(ty) => {
                assert!(stack.top().defined_memories_index.is_none());
                stack.top_mut().memories.push(ty);
            }
            wasmparser::ImportSectionEntryType::Global(ty) => {
                assert!(stack.top().defined_globals_index.is_none());
                stack.top_mut().globals.push(ty);
            }
            wasmparser::ImportSectionEntryType::Instance(ty_idx) => {
                let info = stack.top_mut();
                let ty = match &info.types[usize::try_from(ty_idx).unwrap()] {
                    wasmparser::TypeDef::Instance(ty) => ty.clone(),
                    _ => unreachable!(),
                };
                info.instances.push(ty);
                instance_import_count += 1;
            }
            wasmparser::ImportSectionEntryType::Function(func_ty) => {
                stack.top_mut().functions.push(func_ty);
            }
            wasmparser::ImportSectionEntryType::Table(ty) => {
                stack.top_mut().tables.push(ty);
            }

            wasmparser::ImportSectionEntryType::Module(_) => {
                unreachable!("should have been rejected by `check_import_type`")
            }
            wasmparser::ImportSectionEntryType::Event(_) => {
                unreachable!("should have been rejected by validation")
            }
        }
    }

    stack
        .top_mut()
        .instance_import_counts
        .push(instance_import_count);

    Ok(())
}

fn check_import_type(
    types: &[wasmparser::TypeDef],
    is_root: bool,
    ty: &wasmparser::ImportSectionEntryType,
) -> Result<()> {
    match ty {
        wasmparser::ImportSectionEntryType::Module(_) => {
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
            anyhow::bail!("imported modules are not supported by Wizer")
        }
        wasmparser::ImportSectionEntryType::Instance(inst_ty_index) => {
            // We allow importing instances that only export things that are
            // acceptable imports. This is equivalent to a two-layer import.
            match &types[usize::try_from(*inst_ty_index).unwrap()] {
                wasmparser::TypeDef::Instance(inst_ty) => {
                    for e in inst_ty.exports.iter() {
                        check_import_type(&types, is_root, &e.ty)?;
                    }
                    Ok(())
                }
                _ => unreachable!(),
            }
        }
        wasmparser::ImportSectionEntryType::Memory(mem_ty) => match mem_ty {
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
        wasmparser::ImportSectionEntryType::Event(_) => {
            unreachable!("validation should have rejected the exceptions proposal")
        }
        wasmparser::ImportSectionEntryType::Function(_) => Ok(()),
        wasmparser::ImportSectionEntryType::Table(_)
        | wasmparser::ImportSectionEntryType::Global(_) => {
            anyhow::ensure!(
                !is_root,
                "table and global imports are not allowed in the root Wasm module"
            );
            Ok(())
        }
    }
}

fn alias_section<'a>(
    stack: &mut Vec<ModuleInfo<'a>>,
    full_wasm: &'a [u8],
    mut aliases: wasmparser::AliasSectionReader<'a>,
) -> anyhow::Result<()> {
    stack
        .top_mut()
        .add_raw_section(SectionId::Alias, aliases.range(), full_wasm);

    // Clone any aliases over into this module's index spaces.
    for _ in 0..aliases.get_count() {
        let alias = aliases.read()?;
        match &alias {
            wasmparser::Alias::OuterType {
                relative_depth,
                index,
            } => {
                let relative_depth = usize::try_from(*relative_depth).unwrap();
                let index = usize::try_from(*index).unwrap();
                // NB: `- 2` rather than `- 1` because
                // `relative_depth=0` means this module's immediate
                // parent, not this module itself.
                let ty = stack[stack.len() - 2 - relative_depth].types[index].clone();
                stack.top_mut().types.push(ty);
            }
            wasmparser::Alias::OuterModule {
                relative_depth,
                index,
            } => {
                let relative_depth = usize::try_from(*relative_depth).unwrap();
                let index = usize::try_from(*index).unwrap();
                // Ditto regarding `- 2`.
                let module = stack[stack.len() - 2 - relative_depth].modules[index].clone();
                stack.top_mut().modules.push(module);
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
                    let info = stack.top_mut();
                    let inst_ty_idx = match info.instance_export(*instance, export) {
                        Some(wasmparser::ImportSectionEntryType::Instance(i)) => i,
                        _ => unreachable!(),
                    };
                    let inst_ty = match &info.types[usize::try_from(inst_ty_idx).unwrap()] {
                        wasmparser::TypeDef::Instance(ty) => ty.clone(),
                        _ => unreachable!(),
                    };
                    info.instances.push(inst_ty);
                }
                wasmparser::ExternalKind::Function => {
                    let info = stack.top_mut();
                    let func_ty_idx = match info.instance_export(*instance, export) {
                        Some(wasmparser::ImportSectionEntryType::Function(i)) => i,
                        _ => unreachable!(),
                    };
                    info.functions.push(func_ty_idx);
                }
                wasmparser::ExternalKind::Table => {
                    let info = stack.top_mut();
                    let table_ty = match info.instance_export(*instance, export) {
                        Some(wasmparser::ImportSectionEntryType::Table(ty)) => ty,
                        _ => unreachable!(),
                    };
                    info.tables.push(table_ty);
                }
                wasmparser::ExternalKind::Memory => {
                    let info = stack.top_mut();
                    assert!(info.defined_memories_index.is_none());
                    let ty = match info.instance_export(*instance, export) {
                        Some(wasmparser::ImportSectionEntryType::Memory(ty)) => ty,
                        _ => unreachable!(),
                    };
                    info.memories.push(ty);
                }
                wasmparser::ExternalKind::Global => {
                    let info = stack.top_mut();
                    assert!(info.defined_globals_index.is_none());
                    let ty = match info.instance_export(*instance, export) {
                        Some(wasmparser::ImportSectionEntryType::Global(ty)) => ty,
                        _ => unreachable!(),
                    };
                    info.globals.push(ty);
                }
                wasmparser::ExternalKind::Event => {
                    unreachable!("validation should reject the exceptions proposal")
                }
                wasmparser::ExternalKind::Type => unreachable!("can't export types"),
            },
        }
        stack.top_mut().aliases.push(alias);
    }

    Ok(())
}

fn instance_section<'a>(
    stack: &mut Vec<ModuleInfo<'a>>,
    full_wasm: &'a [u8],
    mut instances: wasmparser::InstanceSectionReader<'a>,
) -> anyhow::Result<()> {
    stack
        .top_mut()
        .add_raw_section(SectionId::Instance, instances.range(), full_wasm);

    // Record the instantiations made in this module, and which modules were
    // instantiated.
    let info = stack.top_mut();
    for _ in 0..instances.get_count() {
        let inst = instances.read()?;
        let module_index: usize = usize::try_from(inst.module()).unwrap();
        let module = &info.modules[module_index];
        let module_id = module.id;
        let inst_ty = module.instance_type();

        let mut import_args_reader = inst.args()?;
        let import_args_count = usize::try_from(import_args_reader.get_count()).unwrap();
        let mut import_args = Vec::with_capacity(import_args_count);
        for _ in 0..import_args_count {
            import_args.push(import_args_reader.read()?);
        }

        info.instantiations.insert(
            u32::try_from(info.instances.len()).unwrap(),
            (module_id, import_args),
        );
        info.instances.push(inst_ty);
    }

    Ok(())
}

fn function_section<'a>(
    stack: &mut Vec<ModuleInfo<'a>>,
    full_wasm: &'a [u8],
    mut funcs: wasmparser::FunctionSectionReader<'a>,
) -> anyhow::Result<()> {
    stack
        .top_mut()
        .add_raw_section(SectionId::Function, funcs.range(), full_wasm);

    let info = stack.top_mut();
    let count = usize::try_from(funcs.get_count()).unwrap();
    info.functions.reserve(count);
    for _ in 0..count {
        let ty = funcs.read()?;
        info.functions.push(ty);
    }
    Ok(())
}

fn table_section<'a>(
    stack: &mut Vec<ModuleInfo<'a>>,
    full_wasm: &'a [u8],
    mut tables: wasmparser::TableSectionReader<'a>,
) -> anyhow::Result<()> {
    stack
        .top_mut()
        .add_raw_section(SectionId::Table, tables.range(), full_wasm);

    let info = stack.top_mut();
    let count = usize::try_from(tables.get_count()).unwrap();
    info.tables.reserve(count);
    for _ in 0..count {
        info.tables.push(tables.read()?);
    }
    Ok(())
}

fn memory_section<'a>(
    stack: &mut Vec<ModuleInfo<'a>>,
    full_wasm: &'a [u8],
    mut mems: wasmparser::MemorySectionReader<'a>,
) -> anyhow::Result<()> {
    let info = stack.top_mut();
    info.add_raw_section(SectionId::Memory, mems.range(), full_wasm);

    assert!(info.defined_memories_index.is_none());
    info.defined_memories_index = Some(u32::try_from(info.memories.len()).unwrap());

    let count = usize::try_from(mems.get_count()).unwrap();
    info.memories.reserve(count);
    for _ in 0..count {
        let m = mems.read()?;
        info.memories.push(m);
    }
    Ok(())
}

fn global_section<'a>(
    stack: &mut Vec<ModuleInfo<'a>>,
    full_wasm: &'a [u8],
    mut globals: wasmparser::GlobalSectionReader<'a>,
) -> anyhow::Result<()> {
    let info = stack.top_mut();
    info.add_raw_section(SectionId::Global, globals.range(), full_wasm);

    assert!(info.defined_globals_index.is_none());
    info.defined_globals_index = Some(u32::try_from(info.globals.len()).unwrap());

    let count = usize::try_from(globals.get_count()).unwrap();
    info.globals.reserve(count);
    for _ in 0..count {
        let g = globals.read()?;
        info.globals.push(g.ty);
    }

    Ok(())
}

fn export_section<'a>(
    stack: &mut Vec<ModuleInfo<'a>>,
    full_wasm: &'a [u8],
    mut exports: wasmparser::ExportSectionReader<'a>,
) -> anyhow::Result<()> {
    stack
        .top_mut()
        .add_raw_section(SectionId::Export, exports.range(), full_wasm);

    let info = stack.top_mut();
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
                info.exports.push(export.clone());
            }
        }
    }
    Ok(())
}
