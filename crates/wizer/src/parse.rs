use crate::info::ModuleContext;
use wasmparser::{Encoding, Parser};
use wasmtime::{bail, error::Context as _};

/// Parse the given Wasm bytes into a `ModuleInfo` tree.
///
/// When `instrumented` is true, `__wizer_*` exports are required and used to
/// populate the `defined_global_exports` and `defined_memory_exports` fields
/// rather than being rejected.
pub(crate) fn parse<'a>(
    full_wasm: &'a [u8],
    instrumented: bool,
) -> wasmtime::Result<ModuleContext<'a>> {
    parse_with(
        full_wasm,
        &mut Parser::new(0).parse_all(full_wasm),
        instrumented,
    )
}

pub(crate) fn parse_with<'a>(
    full_wasm: &'a [u8],
    payloads: &mut impl Iterator<Item = wasmparser::Result<wasmparser::Payload<'a>>>,
    instrumented: bool,
) -> wasmtime::Result<ModuleContext<'a>> {
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
            ExportSection(exports) => export_section(&mut module, exports, instrumented)?,
            End { .. } => break,
            _ => {}
        }
    }

    Ok(module)
}

fn import_section<'a>(
    module: &mut ModuleContext<'a>,
    imports: wasmparser::ImportSectionReader<'a>,
) -> wasmtime::Result<()> {
    // Check that we can properly handle all imports.
    for imp in imports.into_imports() {
        let imp = imp?;

        if imp.module.starts_with("__wizer_") || imp.name.starts_with("__wizer_") {
            wasmtime::bail!(
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
) -> wasmtime::Result<()> {
    for ty_idx in funcs {
        module.push_function(ty_idx?);
    }
    Ok(())
}

fn table_section<'a>(
    module: &mut ModuleContext<'a>,
    tables: wasmparser::TableSectionReader<'a>,
) -> wasmtime::Result<()> {
    for table in tables {
        module.push_table(table?.ty);
    }
    Ok(())
}

fn memory_section<'a>(
    module: &mut ModuleContext<'a>,
    mems: wasmparser::MemorySectionReader<'a>,
) -> wasmtime::Result<()> {
    for m in mems {
        module.push_defined_memory(m?);
    }
    Ok(())
}

fn global_section<'a>(
    module: &mut ModuleContext<'a>,
    globals: wasmparser::GlobalSectionReader<'a>,
) -> wasmtime::Result<()> {
    for g in globals {
        module.push_defined_global(g?.ty);
    }
    Ok(())
}

fn export_section<'a>(
    module: &mut ModuleContext<'a>,
    exports: wasmparser::ExportSectionReader<'a>,
    instrumented: bool,
) -> wasmtime::Result<()> {
    let mut has_instrumentation: bool = false;
    let mut defined_global_exports = Vec::new();
    let mut defined_memory_exports = Vec::new();

    for export in exports {
        let export = export?;

        if export.name.starts_with("__wizer_") {
            if !instrumented {
                wasmtime::bail!(
                    "input Wasm module already exports entities named with the `__wizer_*` prefix"
                );
            }

            has_instrumentation = true;
            if export.name.starts_with("__wizer_global_")
                && export.kind == wasmparser::ExternalKind::Global
            {
                defined_global_exports.push((export.index, export.name.to_string()));
            } else if export.name.starts_with("__wizer_memory_")
                && export.kind == wasmparser::ExternalKind::Memory
            {
                defined_memory_exports.push(export.name.to_string());
            }
            continue;
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

    if instrumented {
        if !has_instrumentation {
            wasmtime::bail!("input Wasm module is not instrumented")
        }
        // Sort to match the order expected by defined_globals() and
        // defined_memories().
        defined_global_exports.sort_by_key(|(idx, _)| *idx);
        defined_memory_exports.sort_by_key(|name| {
            name.strip_prefix("__wizer_memory_")
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(0)
        });

        module.defined_global_exports = Some(defined_global_exports);
        module.defined_memory_exports = Some(defined_memory_exports);
    }

    Ok(())
}
