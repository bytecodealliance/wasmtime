use anyhow::Context;
use std::convert::TryFrom;
use std::fs::File;
use std::path::Path;
use wasi_c2::{
    virt::pipe::{ReadPipe, WritePipe},
    WasiCtx,
};
use wasmtime::{Config, Engine, Linker, Module, Store};

pub fn instantiate(data: &[u8], bin_name: &str, workspace: Option<&Path>) -> anyhow::Result<()> {
    let stdout = WritePipe::new_in_memory();
    let stderr = WritePipe::new_in_memory();

    let r = {
        let store = Store::default();

        // Create our wasi context.
        // Additionally register any preopened directories if we have them.
        let mut builder = wasi_c2::WasiCtx::builder();

        builder
            .arg(bin_name)?
            .arg(".")?
            .stdin(Box::new(ReadPipe::from(Vec::new())))
            .stdout(Box::new(stdout.clone()))
            .stderr(Box::new(stderr.clone()));

        if let Some(workspace) = workspace {
            let dirfd =
                File::open(workspace).context(format!("error while preopening {:?}", workspace))?;
            let preopen_dir = unsafe { cap_std::fs::Dir::from_std_file(dirfd) };
            builder.preopened_dir(Box::new(preopen_dir), ".")?;
        }

        let snapshot1 = wasi_c2_wasmtime::Wasi::new(&store, builder.build()?);

        let mut linker = Linker::new(&store);

        snapshot1.add_to_linker(&mut linker)?;

        let module = Module::new(store.engine(), &data).context("failed to create wasm module")?;
        let instance = linker.instantiate(&module)?;
        let start = instance.get_func("_start").unwrap();
        let with_type = start.get0::<()>()?;
        with_type().map_err(anyhow::Error::from)
    };

    match r {
        Ok(()) => Ok(()),
        Err(trap) => {
            let stdout = stdout
                .try_into_inner()
                .expect("sole ref to stdout")
                .into_inner();
            if !stdout.is_empty() {
                println!("guest stdout:\n{}\n===", String::from_utf8_lossy(&stdout));
            }
            let stderr = stderr
                .try_into_inner()
                .expect("sole ref to stderr")
                .into_inner();
            if !stderr.is_empty() {
                println!("guest stderr:\n{}\n===", String::from_utf8_lossy(&stderr));
            }
            Err(trap.context(format!("error while testing Wasm module '{}'", bin_name,)))
        }
    }
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
