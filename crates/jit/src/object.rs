//! Object file generation.

use super::trampoline::build_trampoline;
use cranelift_frontend::FunctionBuilderContext;
use object::write::Object;
use serde::{Deserialize, Serialize};
use wasmtime_debug::DwarfSection;
use wasmtime_environ::entity::PrimaryMap;
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

    let mut trampolines = PrimaryMap::with_capacity(types.native_signatures.len());
    let mut cx = FunctionBuilderContext::new();
    // Build trampolines for every signature.
    //
    // TODO: for the module linking proposal this builds too many native
    // signatures. This builds trampolines for all signatures for all modules
    // for each module. That's a lot of trampolines! We should instead figure
    // out a way to share trampolines amongst all modules when compiling
    // module-linking modules.
    for (i, native_sig) in types.native_signatures.iter() {
        let func = build_trampoline(isa, &mut cx, native_sig, std::mem::size_of::<u128>())?;
        // Preserve trampoline function unwind info.
        if let Some(info) = &func.unwind_info {
            unwind_info.push(ObjectUnwindInfo::Trampoline(i, info.clone()))
        }
        trampolines.push(func);
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
