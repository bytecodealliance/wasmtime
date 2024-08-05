use anyhow::Result;
use tempfile::TempDir;
use wasmtime::{
    component::{Component, Linker, ResourceTable},
    Engine, Store,
};
use wasmtime_wasi::preview1::WasiP1Ctx;
use wasmtime_wasi::{
    pipe::MemoryOutputPipe, DirPerms, FilePerms, WasiCtx, WasiCtxBuilder, WasiView,
};

struct Ctx {
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
    wasi: WasiP1Ctx,
}

impl WasiView for Ctx {
    fn table(&mut self) -> &mut ResourceTable {
        self.wasi.table()
    }
    fn ctx(&mut self) -> &mut WasiCtx {
        self.wasi.ctx()
    }
}

fn prepare_workspace(exe_name: &str) -> Result<TempDir> {
    let prefix = format!("wasi_components_{exe_name}_");
    let tempdir = tempfile::Builder::new().prefix(&prefix).tempdir()?;
    Ok(tempdir)
}

fn store(
    engine: &Engine,
    name: &str,
    configure: impl FnOnce(&mut WasiCtxBuilder),
) -> Result<(Store<Ctx>, TempDir)> {
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

    configure(&mut builder);
    let ctx = Ctx {
        wasi: builder.build_p1(),
        stderr,
        stdout,
    };

    Ok((Store::new(&engine, ctx), workspace))
}

impl Drop for Ctx {
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

// Assert that each of `sync` and `async` below are testing everything through
// assertion of the existence of the test function itself.
macro_rules! assert_test_exists {
    ($name:ident) => {
        #[allow(unused_imports)]
        use self::$name as _;
    };
}

mod api;
mod async_;
mod preview1;
mod sync;
