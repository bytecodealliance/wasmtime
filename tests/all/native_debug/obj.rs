use anyhow::{Context as _, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use target_lexicon::Triple;
use wasmtime::{CodeBuilder, Config, Engine};

pub fn compile_cranelift(
    wasm: &[u8],
    path: Option<&Path>,
    target: Option<Triple>,
    output: impl AsRef<Path>,
) -> Result<()> {
    let mut config = Config::new();
    config.debug_info(true);
    if let Some(target) = target {
        config.target(&target.to_string())?;
    }
    let engine = Engine::new(&config)?;
    let module = CodeBuilder::new(&engine)
        .wasm_binary_or_text(wasm, path)?
        .compile_module()?;
    let bytes = module.serialize()?;

    let mut file = File::create(output).context("failed to create object file")?;
    file.write_all(&bytes)
        .context("failed to write object file")?;

    Ok(())
}
