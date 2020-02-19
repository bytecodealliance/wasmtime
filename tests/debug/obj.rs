use anyhow::{Context as _, Result};
use std::fs::File;
use std::path::Path;
use target_lexicon::Triple;
use wasmtime::Strategy;
use wasmtime_cli::compile_to_obj;
use wasmtime_environ::CacheConfig;

pub fn compile_cranelift(
    wasm: &[u8],
    target: Option<Triple>,
    output: impl AsRef<Path>,
) -> Result<()> {
    let obj = compile_to_obj(
        wasm,
        target.as_ref(),
        Strategy::Cranelift,
        false,
        false,
        true,
        output
            .as_ref()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        &CacheConfig::new_cache_disabled(),
    )?;

    let file = File::create(output).context("failed to create object file")?;
    obj.write(file).context("failed to write object file")?;

    Ok(())
}
