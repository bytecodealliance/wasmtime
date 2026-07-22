use tempfile::TempDir;
use wasmtime::Result;
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
        Self::new_with_workspace_setup(engine, name, |_| Ok(()), configure)
    }

    /// Like [`Self::new`], but allows seeding the preopened scratch directory
    /// before the guest runs (for host-prepared filesystem fixtures).
    pub fn new_with_workspace_setup(
        engine: &Engine,
        name: &str,
        setup: impl FnOnce(&std::path::Path) -> Result<()>,
        configure: impl FnOnce(&mut WasiCtxBuilder) -> T,
    ) -> Result<(Store<Ctx<T>>, TempDir)> {
        const MAX_OUTPUT_SIZE: usize = 10 << 20;
        let stdout = MemoryOutputPipe::new(MAX_OUTPUT_SIZE);
        let stderr = MemoryOutputPipe::new(MAX_OUTPUT_SIZE);
        let workspace = prepare_workspace(name)?;
        setup(workspace.path())?;

        // Create our wasi context.
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone()).stderr(stderr.clone());

        builder
            .args(&[name, "."])
            .inherit_network()
            .allow_tcp(true)
            .allow_udp(true)
            .allow_ip_name_lookup(true);
        println!("preopen: {workspace:?}");
        builder.preopened_dir(workspace.path(), ".", DirPerms::all(), FilePerms::all())?;
        for (var, val) in test_programs_artifacts::wasi_tests_environment() {
            builder.env(var, val);
        }

        let supports_ipv6 = std::net::TcpListener::bind((std::net::Ipv6Addr::LOCALHOST, 0)).is_ok();
        if !supports_ipv6 {
            builder.env("DISABLE_IPV6", "1");
        }

        let ctx = Ctx {
            wasi: configure(&mut builder),
            stderr,
            stdout,
        };

        Ok((Store::new(engine, ctx), workspace))
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
    wasi: WasiCtx,
    table: ResourceTable,
}

impl MyWasiCtx {
    pub fn new(wasi: WasiCtx) -> Self {
        let mut table = ResourceTable::new();
        table.set_max_capacity(1000);
        Self { wasi, table }
    }
}

impl WasiView for Ctx<MyWasiCtx> {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi.wasi,
            table: &mut self.wasi.table,
        }
    }
}

/// Best-effort far-past host `SystemTime` for stress-testing filestat conversion.
///
/// Prefer a value outside `Datetime`'s `i64` second range when the platform can
/// represent it (typical on Unix). Windows `SystemTime` is FILETIME-based and
/// cannot represent that far past, so fall back to the earliest practical
/// constructible time (around the Windows epoch, ~1601-01-01). The guest test
/// only requires that `path_filestat_get` does not panic the host.
fn extreme_host_mtime() -> std::time::SystemTime {
    use std::time::{Duration, SystemTime};

    // Outside i64 second range when representable (Unix).
    if let Some(t) =
        SystemTime::UNIX_EPOCH.checked_sub(Duration::from_secs((i64::MAX as u64).saturating_add(1)))
    {
        return t;
    }
    // Windows FILETIME lower bound ≈ 1601-01-01 UTC.
    if let Some(t) = SystemTime::UNIX_EPOCH.checked_sub(Duration::from_secs(11_644_473_600)) {
        return t;
    }
    SystemTime::UNIX_EPOCH
        .checked_sub(Duration::from_secs(1))
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Seed a preopened workspace with `extreme.dat` using a host mtime that may
/// fall outside WASI datetime ranges (or be clamped by the OS).
pub fn prepare_extreme_mtime_fixture(dir: &std::path::Path) -> Result<()> {
    use std::fs::{File, FileTimes};
    use std::io::Write;

    let path = dir.join("extreme.dat");
    File::create(&path)?.write_all(b"hello")?;
    let extreme = extreme_host_mtime();
    let f = File::options().write(true).open(&path)?;
    let times = FileTimes::new().set_modified(extreme).set_accessed(extreme);
    // Platforms may reject or clamp extreme times; the file still exists so the
    // guest can open and stat without panicking the host.
    let _ = f.set_times(times);
    Ok(())
}
