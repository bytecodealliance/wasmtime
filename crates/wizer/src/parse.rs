use crate::info::{
    types_interner::{EntityType, TypeId},
    Module, ModuleContext,
};
use crate::stack_ext::StackExt;
use anyhow::{Context, Result};
use std::convert::TryFrom;
use wasm_encoder::SectionId;

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
            CustomSection(c) => stack.top_mut().module.add_raw_section(
                &mut cx,
                SectionId::Custom,
                c.range(),
                full_wasm,
            ),
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
            TagSection(_) => anyhow::bail!("exceptions are not supported yet"),
            End(_) => {
                let entry = stack.pop().unwrap();

                // If we finished parsing the root Wasm module, then we're done.
                assert!(entry.module.is_root());
                assert!(stack.is_empty());
                return Ok(cx);
            }

            ComponentTypeSection(_)
            | ComponentImportSection(_)
            | ComponentExportSection(_)
            | ComponentStartSection { .. }
            | ComponentAliasSection(_)
            | CoreTypeSection(_)
            | InstanceSection(_)
            | ComponentInstanceSection(_)
            | ComponentCanonicalSection(_)
            | ModuleSection { .. }
            | ComponentSection { .. } => {
                unreachable!()
            }
        }
    }
}

fn type_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    types: wasmparser::TypeSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Type, types.range(), full_wasm);

    // Parse out types, as we will need them later when processing
    // instance imports.
    for group in types {
        for ty in group?.into_types() {
            match ty.composite_type {
                ty @ wasmparser::CompositeType::Func(_) => {
                    module.push_type(cx, ty);
                }
                wasmparser::CompositeType::Array(_) => todo!(),
                wasmparser::CompositeType::Struct(_) => todo!(),
            }
        }
    }

    Ok(())
}

fn import_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    imports: wasmparser::ImportSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    stack
        .top_mut()
        .module
        .add_raw_section(cx, SectionId::Import, imports.range(), full_wasm);

    // Check that we can properly handle all imports.
    for imp in imports {
        let imp = imp?;

        if imp.module.starts_with("__wizer_") || imp.name.starts_with("__wizer_") {
            anyhow::bail!(
                "input Wasm module already imports entities named with the `__wizer_*` prefix"
            );
        }

        check_import_type(
            cx,
            stack.top().module.types(cx),
            stack.top().module.is_root(),
            &module.entity_type(cx, imp.ty),
        )?;
        module.push_import(cx, imp);
    }
    Ok(())
}

fn check_import_type(
    _cx: &ModuleContext,
    _types: &[TypeId],
    is_root: bool,
    ty: &EntityType,
) -> Result<()> {
    match ty {
        EntityType::Function(_) => Ok(()),
        EntityType::Memory(mem_ty) => {
            anyhow::ensure!(
                !mem_ty.shared,
                "shared memories are not supported by Wizer yet"
            );
            anyhow::ensure!(
                !mem_ty.memory64,
                "the memory64 proposal is not supported by Wizer yet"
            );
            anyhow::ensure!(
                !is_root,
                "memory imports are not allowed in the root Wasm module"
            );
            Ok(())
        }
        EntityType::Table(_) | EntityType::Global(_) => {
            anyhow::ensure!(
                !is_root,
                "table and global imports are not allowed in the root Wasm module"
            );
            Ok(())
        }
    }
}

fn function_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    funcs: wasmparser::FunctionSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Function, funcs.range(), full_wasm);

    for ty_idx in funcs {
        let ty = module.type_id_at(cx, ty_idx?);
        module.push_function(cx, ty);
    }
    Ok(())
}

fn table_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    tables: wasmparser::TableSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Table, tables.range(), full_wasm);

    for table in tables {
        module.push_table(cx, table?.ty);
    }
    Ok(())
}

fn memory_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    mems: wasmparser::MemorySectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Memory, mems.range(), full_wasm);

    for m in mems {
        module.push_defined_memory(cx, m?);
    }
    Ok(())
}

fn global_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    globals: wasmparser::GlobalSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Global, globals.range(), full_wasm);

    for g in globals {
        module.push_defined_global(cx, g?.ty);
    }
    Ok(())
}

fn export_section<'a>(
    cx: &mut ModuleContext<'a>,
    stack: &mut Vec<StackEntry>,
    full_wasm: &'a [u8],
    exports: wasmparser::ExportSectionReader<'a>,
) -> anyhow::Result<()> {
    let module = stack.top().module;
    module.add_raw_section(cx, SectionId::Export, exports.range(), full_wasm);

    for export in exports {
        let export = export?;

        if export.name.starts_with("__wizer_") {
            anyhow::bail!(
                "input Wasm module already exports entities named with the `__wizer_*` prefix"
            );
        }

        match export.kind {
            wasmparser::ExternalKind::Tag => {
                unreachable!("checked in validation")
            }
            wasmparser::ExternalKind::Func
            | wasmparser::ExternalKind::Table
            | wasmparser::ExternalKind::Memory
            | wasmparser::ExternalKind::Global => {
                module.push_export(cx, export);
            }
        }
    }
    Ok(())
}
