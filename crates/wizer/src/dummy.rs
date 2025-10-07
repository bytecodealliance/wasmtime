//! Dummy implementations of things that a Wasm module can import.
//!
//! Forked from `wasmtime/crates/fuzzing/src/oracles/dummy.rs`.

use anyhow::{Result, anyhow};
use wasmtime::*;

/// Create dummy imports for instantiating the module.
pub fn dummy_imports(
    store: &mut crate::Store,
    module: &wasmtime::Module,
    linker: &mut crate::Linker,
) -> Result<()> {
    log::debug!("Creating dummy imports");

    for imp in module.imports() {
        let name = imp.name();
        if linker.get(&mut *store, imp.module(), name).is_some() {
            // Already defined, must be part of WASI.
            continue;
        }
        let val = dummy_extern(
            &mut *store,
            imp.ty(),
            &format!("'{}' '{}'", imp.module(), name),
        )?;
        linker.define(&mut *store, imp.module(), name, val).unwrap();
    }

    Ok(())
}

/// Construct a dummy `Extern` from its type signature
pub fn dummy_extern(store: &mut crate::Store, ty: ExternType, name: &str) -> Result<Extern> {
    Ok(match ty {
        ExternType::Func(func_ty) => Extern::Func(dummy_func(store, func_ty, name)),
        ExternType::Global(_) => {
            anyhow::bail!("Error: attempted to import unknown global: {}", name)
        }
        ExternType::Table(_) => anyhow::bail!("Error: attempted to import unknown table: {}", name),
        ExternType::Memory(_) => {
            anyhow::bail!("Error: attempted to import unknown memory: {}", name)
        }
        ExternType::Tag(_) => {
            anyhow::bail!("Error: attempted to import unknown tag: {}", name)
        }
    })
}

/// Construct a dummy function for the given function type
pub fn dummy_func(store: &mut crate::Store, ty: FuncType, name: &str) -> Func {
    let name = name.to_string();
    Func::new(store, ty.clone(), move |_caller, _params, _results| {
        Err(anyhow!(
            "Error: attempted to call an unknown imported function: {}\n\
             \n\
             You cannot call arbitrary imported functions during Wizer initialization.",
            name,
        ))
    })
}
