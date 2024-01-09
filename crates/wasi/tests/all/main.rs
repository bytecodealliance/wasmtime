use anyhow::Result;
use tempfile::TempDir;
use wasmtime::{
    component::{Component, Linker, ResourceTable},
    Config, Engine, Store,
};
use wasmtime_wasi::preview2::{
    pipe::MemoryOutputPipe,
    preview1::{WasiPreview1Adapter, WasiPreview1View},
    DirPerms, FilePerms, StdinStream, StdoutStream, WasiCtx, WasiCtxBuilder, WasiView,
};

struct Ctx {
    table: ResourceTable,
    wasi: WasiCtx,
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
    adapter: WasiPreview1Adapter,
}

impl WasiView for Ctx {
    fn table(&self) -> &ResourceTable {
        &self.table
    }
    fn table_mut(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
    fn ctx(&self) -> &WasiCtx {
        &self.wasi
    }
    fn ctx_mut(&mut self) -> &mut WasiCtx {
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

struct StoreBuilder {
    builder: WasiCtxBuilder,
    inherit_stdio: bool,
    workspace: TempDir,
    stdin: Option<Box<dyn StdinStream>>,
    stdout: Option<Box<dyn StdoutStream>>,
    stderr: Option<Box<dyn StdoutStream>>,
}

impl StoreBuilder {
    fn new(name: &str) -> Result<Self> {
        let mut builder = WasiCtxBuilder::new();

        builder
            .args(&[name, "."])
            .inherit_network()
            .allow_ip_name_lookup(true);

        let workspace = prepare_workspace(name)?;

        Ok(Self {
            builder,
            inherit_stdio: false,
            workspace,
            stdin: None,
            stdout: None,
            stderr: None,
        })
    }

    fn stdout(&mut self, stdout: Box<dyn StdoutStream>) -> &mut Self {
        self.stdout.replace(stdout);
        self
    }

    fn stdin(&mut self, stdin: Box<dyn StdinStream>) -> &mut Self {
        self.stdin.replace(stdin);
        self
    }

    fn build(mut self, engine: &Engine) -> Result<(Store<Ctx>, TempDir)> {
        let stdout = MemoryOutputPipe::new(4096);
        let stderr = MemoryOutputPipe::new(4096);

        // Create our wasi context.
        if self.inherit_stdio {
            self.builder.inherit_stdio();
        } else {
            if let Some(stdin) = self.stdin {
                self.builder.stdin(stdin);
            }

            if let Some(stdout) = self.stdout {
                self.builder.stdout(stdout);
            } else {
                self.builder.stdout(stdout.clone());
            }

            if let Some(stderr) = self.stderr {
                self.builder.stderr(stderr);
            } else {
                self.builder.stderr(stderr.clone());
            }
        }

        println!("preopen: {:?}", self.workspace);
        let preopen_dir = cap_std::fs::Dir::open_ambient_dir(
            self.workspace.path(),
            cap_std::ambient_authority(),
        )?;
        self.builder
            .preopened_dir(preopen_dir, DirPerms::all(), FilePerms::all(), ".");
        for (var, val) in test_programs_artifacts::wasi_tests_environment() {
            self.builder.env(var, val);
        }

        let ctx = Ctx {
            table: ResourceTable::new(),
            wasi: self.builder.build(),
            stderr,
            stdout,
            adapter: WasiPreview1Adapter::new(),
        };

        Ok((Store::new(&engine, ctx), self.workspace))
    }
}

fn store(engine: &Engine, name: &str, inherit_stdio: bool) -> Result<(Store<Ctx>, TempDir)> {
    let mut builder = StoreBuilder::new(name)?;
    builder.inherit_stdio = inherit_stdio;
    builder.build(engine)
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
mod piped;
mod preview1;
mod sync;
