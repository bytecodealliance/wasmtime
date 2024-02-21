use anyhow::Result;
use tempfile::TempDir;
use wasmtime::{
    component::{Component, Linker, ResourceTable},
    Config, Engine, Store,
};
use wasmtime_wasi::{
    pipe::MemoryOutputPipe,
    preview1::{WasiPreview1Adapter, WasiPreview1View},
    DirPerms, FilePerms, WasiCtx, WasiCtxBuilder, WasiView,
};

struct Ctx {
    table: ResourceTable,
    wasi: WasiCtx,
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
    adapter: WasiPreview1Adapter,
}

impl WasiView for Ctx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl WasiPreview1View for Ctx {
    fn adapter(&self) -> &WasiPreview1Adapter {
        &self.adapter
    }
    fn adapter_mut(&mut self) -> &mut WasiPreview1Adapter {
        &mut self.adapter
    }
}

fn prepare_workspace(exe_name: &str) -> Result<TempDir> {
    let prefix = format!("wasi_components_{}_", exe_name);
    let tempdir = tempfile::Builder::new().prefix(&prefix).tempdir()?;
    Ok(tempdir)
}

fn store(engine: &Engine, name: &str, inherit_stdio: bool) -> Result<(Store<Ctx>, TempDir)> {
    let stdout = MemoryOutputPipe::new(4096);
    let stderr = MemoryOutputPipe::new(4096);
    let workspace = prepare_workspace(name)?;

    // Create our wasi context.
    let mut builder = WasiCtxBuilder::new();
    if inherit_stdio {
        builder.inherit_stdio();
    } else {
        builder.stdout(stdout.clone()).stderr(stderr.clone());
    }

    builder
        .args(&[name, "."])
        .inherit_network()
        .allow_ip_name_lookup(true);
    println!("preopen: {:?}", workspace);
    let preopen_dir =
        cap_std::fs::Dir::open_ambient_dir(workspace.path(), cap_std::ambient_authority())?;
    builder.preopened_dir(preopen_dir, DirPerms::all(), FilePerms::all(), ".");
    for (var, val) in test_programs_artifacts::wasi_tests_environment() {
        builder.env(var, val);
    }

    let ctx = Ctx {
        table: ResourceTable::new(),
        wasi: builder.build(),
        stderr,
        stdout,
        adapter: WasiPreview1Adapter::new(),
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
