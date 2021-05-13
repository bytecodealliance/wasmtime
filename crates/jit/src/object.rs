//! Object file generation.

use super::trampoline::build_trampoline;
use cranelift_frontend::FunctionBuilderContext;
use object::write::Object;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use wasmtime_debug::DwarfSection;
use wasmtime_environ::isa::{unwind::UnwindInfo, TargetIsa};
use wasmtime_environ::wasm::{FuncIndex, SignatureIndex};
use wasmtime_environ::{CompiledFunctions, ModuleTranslation, TypeTables};
use wasmtime_obj::{ObjectBuilder, ObjectBuilderTarget};

pub use wasmtime_obj::utils;

/// Unwind information for object files functions (including trampolines).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectUnwindInfo {
    Func(FuncIndex, UnwindInfo),
    Trampoline(SignatureIndex, UnwindInfo),
}

// Builds ELF image from the module `Compilation`.
pub(crate) fn build_object(
    isa: &dyn TargetIsa,
    translation: &ModuleTranslation,
    types: &TypeTables,
    funcs: &CompiledFunctions,
    dwarf_sections: Vec<DwarfSection>,
) -> Result<(Object, Vec<ObjectUnwindInfo>), anyhow::Error> {
    const CODE_SECTION_ALIGNMENT: u64 = 0x1000;

    let mut unwind_info = Vec::new();

    // Preserve function unwind info.
    unwind_info.extend(funcs.iter().filter_map(|(index, func)| {
        func.unwind_info
            .as_ref()
            .map(|info| ObjectUnwindInfo::Func(translation.module.func_index(index), info.clone()))
    }));

    // Build trampolines for every signature that can be used by this module.
    let signatures = translation
        .module
        .functions
        .iter()
        .filter_map(|(i, sig)| match translation.module.defined_func_index(i) {
            Some(i) if !translation.module.possibly_exported_funcs.contains(&i) => None,
            _ => Some(*sig),
        })
        .collect::<BTreeSet<_>>();
    let mut trampolines = Vec::with_capacity(signatures.len());
    let mut cx = FunctionBuilderContext::new();
    for i in signatures {
        let native_sig = wasmtime_cranelift::indirect_signature(isa, &types, i);
        let func = build_trampoline(isa, &mut cx, &native_sig, std::mem::size_of::<u128>())?;
        // Preserve trampoline function unwind info.
        if let Some(info) = &func.unwind_info {
            unwind_info.push(ObjectUnwindInfo::Trampoline(i, info.clone()))
        }
        trampolines.push((i, func));
    }

    let target = ObjectBuilderTarget::new(isa.triple().architecture)?;
    let mut builder = ObjectBuilder::new(target, &translation.module, funcs);
    builder
        .set_code_alignment(CODE_SECTION_ALIGNMENT)
        .set_trampolines(trampolines)
        .set_dwarf_sections(dwarf_sections);
    let obj = builder.build()?;

    Ok((obj, unwind_info))
}
