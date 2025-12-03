use crate::info::ModuleContext;
use anyhow::{Context, bail};
use wasmparser::{Encoding, Parser};

/// Parse the given Wasm bytes into a `ModuleInfo` tree.
pub(crate) fn parse<'a>(full_wasm: &'a [u8]) -> anyhow::Result<ModuleContext<'a>> {
    parse_with(full_wasm, &mut Parser::new(0).parse_all(full_wasm))
}

pub(crate) fn parse_with<'a>(
    full_wasm: &'a [u8],
    payloads: &mut impl Iterator<Item = wasmparser::Result<wasmparser::Payload<'a>>>,
) -> anyhow::Result<ModuleContext<'a>> {
    log::debug!("Parsing the input Wasm");

    let mut module = ModuleContext::default();

    while let Some(payload) = payloads.next() {
        use wasmparser::Payload::*;

        let payload = payload.context("failed to parse Wasm")?;

        if let Some((id, range)) = payload.as_section() {
            module.add_raw_section(id, range, full_wasm);
        }

        match payload {
            Version {
                encoding: Encoding::Component,
                ..
            } => {
                bail!("expected a core module, found a component");
            }
            ImportSection(imports) => import_section(&mut module, imports)?,
            FunctionSection(funcs) => function_section(&mut module, funcs)?,
            TableSection(tables) => table_section(&mut module, tables)?,
            MemorySection(mems) => memory_section(&mut module, mems)?,
            GlobalSection(globals) => global_section(&mut module, globals)?,
            ExportSection(exports) => export_section(&mut module, exports)?,
            End { .. } => break,
            _ => {}
        }
    }

    Ok(module)
}

fn import_section<'a>(
    module: &mut ModuleContext<'a>,
    imports: wasmparser::ImportSectionReader<'a>,
) -> anyhow::Result<()> {
    // Check that we can properly handle all imports.
    for imp in imports {
        let imp = imp?;

        if imp.module.starts_with("__wizer_") || imp.name.starts_with("__wizer_") {
            anyhow::bail!(
                "input Wasm module already imports entities named with the `__wizer_*` prefix"
            );
        }

        module.push_import(imp);
    }
    Ok(())
}

fn function_section<'a>(
    module: &mut ModuleContext<'a>,
    funcs: wasmparser::FunctionSectionReader<'a>,
) -> anyhow::Result<()> {
    for ty_idx in funcs {
        module.push_function(ty_idx?);
    }
    Ok(())
}

fn table_section<'a>(
    module: &mut ModuleContext<'a>,
    tables: wasmparser::TableSectionReader<'a>,
) -> anyhow::Result<()> {
    for table in tables {
        module.push_table(table?.ty);
    }
    Ok(())
}

fn memory_section<'a>(
    module: &mut ModuleContext<'a>,
    mems: wasmparser::MemorySectionReader<'a>,
) -> anyhow::Result<()> {
    for m in mems {
        module.push_defined_memory(m?);
    }
    Ok(())
}

fn global_section<'a>(
    module: &mut ModuleContext<'a>,
    globals: wasmparser::GlobalSectionReader<'a>,
) -> anyhow::Result<()> {
    for g in globals {
        module.push_defined_global(g?.ty);
    }
    Ok(())
}

fn export_section<'a>(
    module: &mut ModuleContext<'a>,
    exports: wasmparser::ExportSectionReader<'a>,
) -> anyhow::Result<()> {
    for export in exports {
        let export = export?;

        if export.name.starts_with("__wizer_") {
            anyhow::bail!(
                "input Wasm module already exports entities named with the `__wizer_*` prefix"
            );
        }

        match export.kind {
            wasmparser::ExternalKind::Tag
            | wasmparser::ExternalKind::Func
            | wasmparser::ExternalKind::FuncExact
            | wasmparser::ExternalKind::Table
            | wasmparser::ExternalKind::Memory
            | wasmparser::ExternalKind::Global => {
                module.push_export(export);
            }
        }
    }
    Ok(())
}
