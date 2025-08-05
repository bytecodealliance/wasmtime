use anyhow::Result;
use tempfile::TempDir;
use wasmtime::component::ResourceTable;
use wasmtime::{Engine, Store};
use wasmtime_wasi::{
    DirPerms, FilePerms, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, p2::pipe::MemoryOutputPipe,
};

pub struct Ctx<T> {
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
    pub wasi: T,
}

fn prepare_workspace(exe_name: &str) -> Result<TempDir> {
    let prefix = format!("wasi_components_{exe_name}_");
    let tempdir = tempfile::Builder::new().prefix(&prefix).tempdir()?;
    Ok(tempdir)
}

impl<T> Ctx<T> {
    pub fn new(
        engine: &Engine,
        name: &str,
        configure: impl FnOnce(&mut WasiCtxBuilder) -> T,
    ) -> Result<(Store<Ctx<T>>, TempDir)> {
        let stdout = MemoryOutputPipe::new(4096);
        let stderr = MemoryOutputPipe::new(4096);
        let workspace = prepare_workspace(name)?;

        // Create our wasi context.
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone()).stderr(stderr.clone());

        builder
            .args(&[name, "."])
            .inherit_network()
            .allow_ip_name_lookup(true);
        println!("preopen: {workspace:?}");
        builder.preopened_dir(workspace.path(), ".", DirPerms::all(), FilePerms::all())?;
        for (var, val) in test_programs_artifacts::wasi_tests_environment() {
            builder.env(var, val);
        }

        let ctx = Ctx {
            wasi: configure(&mut builder),
            stderr,
            stdout,
        };

        Ok((Store::new(&engine, ctx), workspace))
    }
}

impl<T> Drop for Ctx<T> {
    fn drop(&mut self) {
        let stdout = self.stdout.contents();
        if !stdout.is_empty() {
            println!("[guest] stdout:\n{}\n===", String::from_utf8_lossy(&stdout));
        }
        let stderr = self.stderr.contents();
        if !stderr.is_empty() {
            println!("[guest] stderr:\n{}\n===", String::from_utf8_lossy(&stderr));
        }
    }
}

pub struct MyWasiCtx {
    pub wasi: WasiCtx,
    pub table: ResourceTable,
}

impl WasiView for Ctx<MyWasiCtx> {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi.wasi,
            table: &mut self.wasi.table,
        }
    }
}
