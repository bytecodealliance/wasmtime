#![cfg(feature = "test_programs")]
use anyhow::Result;
use tempfile::TempDir;
use wasmtime::{component::Linker, Config, Engine, Store};
use wasmtime_wasi::preview2::{
    command::sync::{add_to_linker, Command},
    pipe::MemoryOutputPipe,
    DirPerms, FilePerms, Table, WasiCtx, WasiCtxBuilder, WasiView,
};

lazy_static::lazy_static! {
    static ref ENGINE: Engine = {
        let mut config = Config::new();
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(true);
        config.async_support(false);

        let engine = Engine::new(&config).unwrap();
        engine
    };
}
// uses ENGINE, creates a fn get_component(&str) -> Component
include!(concat!(env!("OUT_DIR"), "/wasi_tests_components.rs"));

pub fn prepare_workspace(exe_name: &str) -> Result<TempDir> {
    let prefix = format!("wasi_components_{}_", exe_name);
    let tempdir = tempfile::Builder::new().prefix(&prefix).tempdir()?;
    Ok(tempdir)
}

fn run(name: &str, inherit_stdio: bool) -> Result<()> {
    let workspace = prepare_workspace(name)?;
    let stdout = MemoryOutputPipe::new();
    let stderr = MemoryOutputPipe::new();
    let r = {
        let mut linker = Linker::new(&ENGINE);
        add_to_linker(&mut linker)?;

        // Create our wasi context.
        // Additionally register any preopened directories if we have them.
        let mut builder = WasiCtxBuilder::new();

        if inherit_stdio {
            builder.inherit_stdio();
        } else {
            builder.stdout(stdout.clone()).stderr(stderr.clone());
        }
        builder.args(&[name, "."]);
        println!("preopen: {:?}", workspace);
        let preopen_dir =
            cap_std::fs::Dir::open_ambient_dir(workspace.path(), cap_std::ambient_authority())?;
        builder.preopened_dir(preopen_dir, DirPerms::all(), FilePerms::all(), ".");
        for (var, val) in test_programs::wasi_tests_environment() {
            builder.env(var, val);
        }

        let mut table = Table::new();
        let wasi = builder.build(&mut table)?;
        struct Ctx {
            wasi: WasiCtx,
            table: Table,
        }
        impl WasiView for Ctx {
            fn ctx(&self) -> &WasiCtx {
                &self.wasi
            }
            fn ctx_mut(&mut self) -> &mut WasiCtx {
                &mut self.wasi
            }
            fn table(&self) -> &Table {
                &self.table
            }
            fn table_mut(&mut self) -> &mut Table {
                &mut self.table
            }
        }

        let ctx = Ctx { wasi, table };
        let mut store = Store::new(&ENGINE, ctx);
        let (command, _instance) = Command::instantiate(&mut store, &get_component(name), &linker)?;
        command
            .call_run(&mut store)?
            .map_err(|()| anyhow::anyhow!("run returned a failure"))?;
        Ok(())
    };

    r.map_err(move |trap: anyhow::Error| {
        let stdout = stdout.try_into_inner().expect("single ref to stdout");
        if !stdout.is_empty() {
            println!("guest stdout:\n{}\n===", String::from_utf8_lossy(&stdout));
        }
        let stderr = stderr.try_into_inner().expect("single ref to stderr");
        if !stderr.is_empty() {
            println!("guest stderr:\n{}\n===", String::from_utf8_lossy(&stderr));
        }
        trap.context(format!(
            "error while testing wasi-tests {} with cap-std-sync",
            name
        ))
    })?;
    Ok(())
}

// Below here is mechanical: there should be one test for every binary in
// wasi-tests. The only differences should be should_panic annotations for
// tests which fail.
#[test_log::test]
fn big_random_buf() {
    run("big_random_buf", false).unwrap()
}
#[test_log::test]
fn clock_time_get() {
    run("clock_time_get", false).unwrap()
}
#[test_log::test]
fn close_preopen() {
    run("close_preopen", false).unwrap()
}
#[test_log::test]
fn dangling_fd() {
    run("dangling_fd", false).unwrap()
}
#[test_log::test]
fn dangling_symlink() {
    run("dangling_symlink", false).unwrap()
}
#[test_log::test]
fn directory_seek() {
    run("directory_seek", false).unwrap()
}
#[test_log::test]
fn dir_fd_op_failures() {
    run("dir_fd_op_failures", false).unwrap()
}
#[test_log::test]
fn fd_advise() {
    run("fd_advise", false).unwrap()
}
#[test_log::test]
fn fd_filestat_get() {
    run("fd_filestat_get", false).unwrap()
}
#[test_log::test]
fn fd_filestat_set() {
    run("fd_filestat_set", false).unwrap()
}
#[test_log::test]
fn fd_flags_set() {
    run("fd_flags_set", false).unwrap()
}
#[test_log::test]
fn fd_readdir() {
    run("fd_readdir", false).unwrap()
}
#[test_log::test]
fn file_allocate() {
    run("file_allocate", false).unwrap()
}
#[test_log::test]
fn file_pread_pwrite() {
    run("file_pread_pwrite", false).unwrap()
}
#[test_log::test]
fn file_seek_tell() {
    run("file_seek_tell", false).unwrap()
}
#[test_log::test]
fn file_truncation() {
    run("file_truncation", false).unwrap()
}
#[test_log::test]
fn file_unbuffered_write() {
    run("file_unbuffered_write", false).unwrap()
}
#[test_log::test]
#[cfg_attr(windows, should_panic)]
fn interesting_paths() {
    run("interesting_paths", false).unwrap()
}
#[test_log::test]
fn isatty() {
    run("isatty", false).unwrap()
}
#[test_log::test]
fn nofollow_errors() {
    run("nofollow_errors", false).unwrap()
}
#[test_log::test]
fn overwrite_preopen() {
    run("overwrite_preopen", false).unwrap()
}
#[test_log::test]
fn path_exists() {
    run("path_exists", false).unwrap()
}
#[test_log::test]
fn path_filestat() {
    run("path_filestat", false).unwrap()
}
#[test_log::test]
fn path_link() {
    run("path_link", false).unwrap()
}
#[test_log::test]
fn path_open_create_existing() {
    run("path_open_create_existing", false).unwrap()
}
#[test_log::test]
fn path_open_read_write() {
    run("path_open_read_write", false).unwrap()
}
#[test_log::test]
fn path_open_dirfd_not_dir() {
    run("path_open_dirfd_not_dir", false).unwrap()
}
#[test_log::test]
fn path_open_missing() {
    run("path_open_missing", false).unwrap()
}
#[test_log::test]
fn path_open_nonblock() {
    run("path_open_nonblock", false).unwrap()
}
#[test_log::test]
fn path_rename_dir_trailing_slashes() {
    run("path_rename_dir_trailing_slashes", false).unwrap()
}
#[test_log::test]
#[should_panic]
fn path_rename_file_trailing_slashes() {
    run("path_rename_file_trailing_slashes", false).unwrap()
}
#[test_log::test]
fn path_rename() {
    run("path_rename", false).unwrap()
}
#[test_log::test]
fn path_symlink_trailing_slashes() {
    run("path_symlink_trailing_slashes", false).unwrap()
}
#[test_log::test]
fn poll_oneoff_files() {
    run("poll_oneoff_files", false).unwrap()
}

#[test_log::test]
fn poll_oneoff_stdio() {
    run("poll_oneoff_stdio", true).unwrap()
}
#[test_log::test]
fn readlink() {
    run("readlink", false).unwrap()
}
#[test_log::test]
#[should_panic]
fn remove_directory_trailing_slashes() {
    run("remove_directory_trailing_slashes", false).unwrap()
}
#[test_log::test]
fn remove_nonempty_directory() {
    run("remove_nonempty_directory", false).unwrap()
}
#[test_log::test]
fn renumber() {
    run("renumber", false).unwrap()
}
#[test_log::test]
fn sched_yield() {
    run("sched_yield", false).unwrap()
}
#[test_log::test]
fn stdio() {
    run("stdio", false).unwrap()
}
#[test_log::test]
fn symlink_create() {
    run("symlink_create", false).unwrap()
}
#[test_log::test]
fn symlink_filestat() {
    run("symlink_filestat", false).unwrap()
}
#[test_log::test]
fn symlink_loop() {
    run("symlink_loop", false).unwrap()
}
#[test_log::test]
fn unlink_file_trailing_slashes() {
    run("unlink_file_trailing_slashes", false).unwrap()
}
#[test_log::test]
fn path_open_preopen() {
    run("path_open_preopen", false).unwrap()
}
