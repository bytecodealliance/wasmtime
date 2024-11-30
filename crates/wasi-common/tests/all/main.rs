use anyhow::Result;
use std::path::Path;
use tempfile::TempDir;
use wasi_common::pipe::WritePipe;
use wasmtime::{Linker, Module, Store};

pub fn prepare_workspace(exe_name: &str) -> Result<TempDir> {
    let prefix = format!("wasi_common_{exe_name}_");
    let tempdir = tempfile::Builder::new().prefix(&prefix).tempdir()?;
    Ok(tempdir)
}

macro_rules! assert_test_exists {
    ($name:ident) => {
        #[expect(unused_imports, reason = "just here to ensure a name exists")]
        use self::$name as _;
    };
}

mod async_;
mod sync;
