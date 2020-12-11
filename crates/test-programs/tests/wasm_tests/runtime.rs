use anyhow::Context;
use std::convert::TryFrom;
use std::fs::File;
use std::path::Path;
use wasi_c2::WasiCtx;
use wasmtime::{Linker, Module, Store};

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
    let mut builder = wasi_c2::WasiCtx::builder();

    builder.arg(bin_name)?.arg(".")?.inherit_stdio();

    /*
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
    */
    /*
        // The nonstandard thing we do with `WasiCtxBuilder` is to ensure that
        // `stdin` is always an unreadable pipe. This is expected in the test suite
        // where `stdin` is never ready to be read. In some CI systems, however,
        // stdin is closed which causes tests to fail.
        let (reader, _writer) = os_pipe::pipe()?;
        let file = reader_to_file(reader);
        let handle = OsOther::try_from(file).context("failed to create OsOther from PipeReader")?;
        builder.stdin(handle);
    */
    let snapshot1 = wasi_c2_wasmtime::Wasi::new(&store, builder.build()?);

    let mut linker = Linker::new(&store);

    snapshot1.add_to_linker(&mut linker)?;

    let module = Module::new(store.engine(), &data).context("failed to create wasm module")?;

    linker
        .module("", &module)
        .and_then(|m| m.get_default(""))
        .and_then(|f| f.get0::<()>())
        .and_then(|f| f().map_err(Into::into))
        .context(format!("error while testing Wasm module '{}'", bin_name,))
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
