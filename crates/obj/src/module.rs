use crate::context::layout_vmcontext;
use crate::data_segment::{declare_data_segment, emit_data_segment};
use crate::function::{declare_functions, emit_functions};
use crate::table::{declare_table, emit_table};
use faerie::{Artifact, Decl, Link};
use wasmtime_environ::isa::TargetFrontendConfig;
use wasmtime_environ::{Compilation, DataInitializer, Module, Relocations};

fn emit_vmcontext_init(
    obj: &mut Artifact,
    module: &Module,
    target_config: &TargetFrontendConfig,
) -> Result<(), String> {
    let (data, table_relocs) = layout_vmcontext(module, target_config);
    obj.declare_with("_vmcontext_init", Decl::data().global(), data.to_vec())
        .map_err(|err| format!("{}", err))?;
    for reloc in table_relocs.iter() {
        let target_name = format!("_table_{}", reloc.index);
        obj.link(Link {
            from: "_vmcontext_init",
            to: &target_name,
            at: reloc.offset as u64,
        })
        .map_err(|err| format!("{}", err))?;
    }
    Ok(())
}

/// Emits a module that has been emitted with the `wasmtime-environ` environment
/// implementation to a native object file.
pub fn emit_module(
    obj: &mut Artifact,
    module: &Module,
    compilation: &Compilation,
    relocations: &Relocations,
    data_initializers: &[DataInitializer],
    target_config: &TargetFrontendConfig,
) -> Result<(), String> {
    declare_functions(obj, module, relocations)?;

    for (i, initializer) in data_initializers.iter().enumerate() {
        declare_data_segment(obj, initializer, i)?;
    }

    for i in 0..module.table_plans.len() {
        declare_table(obj, i)?;
    }

    emit_functions(obj, module, compilation, relocations)?;

    for (i, initializer) in data_initializers.iter().enumerate() {
        emit_data_segment(obj, initializer, i)?;
    }

    for i in 0..module.table_plans.len() {
        emit_table(obj, i)?;
    }

    emit_vmcontext_init(obj, module, target_config)?;

    Ok(())
}
