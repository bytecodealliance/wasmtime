use anyhow::Result;
use tempfile::TempDir;
use wasmtime::{
    Engine, Store,
    component::{Component, Linker},
};
use wasmtime_wasi::preview1::WasiP1Ctx;
use wasmtime_wasi::{
    DirPerms, FilePerms, WasiCtxBuilder, WasiCtxView, WasiView, p2::pipe::MemoryOutputPipe,
};

struct Ctx {
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
    wasi: WasiP1Ctx,
}

impl WasiView for Ctx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiView::ctx(&mut self.wasi)
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

/// A `sandbox` directory to preopen next to an `outside` directory which isn't
/// preopened, for checking that the guest can't move `outside` into `sandbox`.
pub struct SandboxFixture {
    tempdir: TempDir,
}

impl SandboxFixture {
    pub fn new() -> Result<Self> {
        let tempdir = prepare_workspace("sandbox")?;
        std::fs::create_dir(tempdir.path().join("sandbox"))?;
        std::fs::create_dir(tempdir.path().join("outside"))?;
        std::fs::write(tempdir.path().join("outside/secret"), b"secret\n")?;
        Ok(Self { tempdir })
    }

    pub fn configure_wasi(&self, builder: &mut WasiCtxBuilder) {
        builder
            .preopened_dir(
                self.tempdir.path().join("sandbox"),
                "sandbox",
                DirPerms::all(),
                FilePerms::all(),
            )
            .unwrap();
    }

    pub fn assert_secure(&self) {
        assert!(
            self.tempdir.path().join("outside/secret").exists(),
            "`outside` was moved out of place"
        );
        assert!(
            !self.tempdir.path().join("sandbox/stolen").exists(),
            "`outside` was moved into the sandbox"
        );
    }
}

// Assert that each of `sync` and `async` below are testing everything through
// assertion of the existence of the test function itself.
macro_rules! assert_test_exists {
    ($name:ident) => {
        #[expect(unused_imports, reason = "just here to ensure a name exists")]
        use self::$name as _;
    };
}

mod api;
mod async_;
mod preview1;
mod sync;
