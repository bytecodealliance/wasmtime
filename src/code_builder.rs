use wasmtime::Result;
use wasmtime_cli_flags::CommonOptions;

/// Apply the `-C unsafe-intrinsics` and `-C compile-time-builtin` options in
/// `common` to `code`.
pub fn configure_code_builder(
    common: &CommonOptions,
    code: &mut wasmtime::CodeBuilder<'_>,
) -> Result<()> {
    if let Some(name) = &common.codegen.unsafe_intrinsics {
        // SAFETY: the user opted into this by passing flags that are documented
        // as unsafe.
        unsafe {
            code.expose_unsafe_intrinsics(name.0.clone());
        }
    }

    #[cfg(feature = "compile-time-builtins")]
    for builtin in &common.codegen.compile_time_builtin {
        // SAFETY: the user opted into this by passing flags that are documented
        // as unsafe.
        unsafe {
            code.compile_time_builtins_binary_or_text_file(builtin.name.clone(), &builtin.path)?;
        }
    }

    #[cfg(not(feature = "compile-time-builtins"))]
    wasmtime::ensure!(
        common.codegen.compile_time_builtin.is_empty(),
        "support for compile-time-builtins disabled at compile time",
    );

    Ok(())
}
