use anyhow::{bail, Context};
use std::fs::File;
use std::path::Path;
use wasi_common::VirtualDirEntry;
use wasmtime::{Instance, Module, Store};

#[derive(Clone, Copy, Debug)]
pub enum PreopenType {
    /// Preopens should be satisfied with real OS files.
    OS,
    /// Preopens should be satisfied with virtual files.
    Virtual,
}

pub fn instantiate(
    data: &[u8],
    bin_name: &str,
    workspace: Option<&Path>,
    preopen_type: PreopenType,
) -> anyhow::Result<()> {
    let store = Store::default();

    // Create our wasi context with pretty standard arguments/inheritance/etc.
    // Additionally register any preopened directories if we have them.
    let mut builder = wasi_common::WasiCtxBuilder::new();

    builder.arg(bin_name).arg(".").inherit_stdio();

    if let Some(workspace) = workspace {
        match preopen_type {
            PreopenType::OS => {
                let preopen_dir = wasi_common::preopen_dir(workspace)
                    .context(format!("error while preopening {:?}", workspace))?;
                builder.preopened_dir(preopen_dir, ".");
            }
            PreopenType::Virtual => {
                // we can ignore the workspace path for virtual preopens because virtual preopens
                // don't exist in the filesystem anyway - no name conflict concerns.
                builder.preopened_virt(VirtualDirEntry::empty_directory(), ".");
            }
        }
    }

    // The nonstandard thing we do with `WasiCtxBuilder` is to ensure that
    // `stdin` is always an unreadable pipe. This is expected in the test suite
    // where `stdin` is never ready to be read. In some CI systems, however,
    // stdin is closed which causes tests to fail.
    let (reader, _writer) = os_pipe::pipe()?;
    builder.stdin(reader_to_file(reader));
    let snapshot1 = wasmtime_wasi::Wasi::new(&store, builder.build()?);
    let module = Module::new(&store, &data).context("failed to create wasm module")?;
    let imports = module
        .imports()
        .iter()
        .map(|i| {
            let field_name = i.name();
            if let Some(export) = snapshot1.get_export(field_name) {
                Ok(export.clone().into())
            } else {
                bail!(
                    "import {} was not found in module {}",
                    field_name,
                    i.module(),
                )
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let instance = Instance::new(&module, &imports).context(format!(
        "error while instantiating Wasm module '{}'",
        bin_name,
    ))?;

    let export = instance
        .get_export("_start")
        .context("expected a _start export")?
        .clone();

    export
        .func()
        .context("expected export to be a func")?
        .call(&[])?;

    Ok(())
}

#[cfg(unix)]
fn reader_to_file(reader: os_pipe::PipeReader) -> File {
    use std::os::unix::prelude::*;
    unsafe { File::from_raw_fd(reader.into_raw_fd()) }
}

#[cfg(windows)]
fn reader_to_file(reader: os_pipe::PipeReader) -> File {
    use std::os::windows::prelude::*;
    unsafe { File::from_raw_handle(reader.into_raw_handle()) }
}
