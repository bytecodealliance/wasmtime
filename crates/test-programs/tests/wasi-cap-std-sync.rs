#![cfg(feature = "test_programs")]
use anyhow::Result;
use tempfile::TempDir;
use wasi_common::pipe::WritePipe;
use wasmtime::{Config, Engine, Linker, Store};

lazy_static::lazy_static! {
    static ref ENGINE: Engine = {
        let mut config = Config::new();
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(false);
        config.async_support(false);

        let engine = Engine::new(&config).unwrap();
        engine
    };
}
// uses ENGINE, creates a fn get_module(&str) -> Module
include!(concat!(env!("OUT_DIR"), "/wasi_tests_modules.rs"));

pub fn prepare_workspace(exe_name: &str) -> Result<TempDir> {
    let prefix = format!("wasi_cap_std_sync_{}_", exe_name);
    let tempdir = tempfile::Builder::new().prefix(&prefix).tempdir()?;
    Ok(tempdir)
}

use wasmtime_wasi::sync::{add_to_linker, WasiCtxBuilder};
fn run(name: &str, inherit_stdio: bool) -> Result<()> {
    let workspace = prepare_workspace(name)?;
    let stdout = WritePipe::new_in_memory();
    let stderr = WritePipe::new_in_memory();
    let r = {
        let mut linker = Linker::new(&ENGINE);
        add_to_linker(&mut linker, |cx| cx)?;

        // Create our wasi context.
        // Additionally register any preopened directories if we have them.
        let mut builder = WasiCtxBuilder::new();

        if inherit_stdio {
            builder.inherit_stdio();
        } else {
            builder
                .stdout(Box::new(stdout.clone()))
                .stderr(Box::new(stderr.clone()));
        }
        builder.arg(name)?.arg(".")?;
        println!("preopen: {:?}", workspace);
        let preopen_dir =
            cap_std::fs::Dir::open_ambient_dir(workspace.path(), cap_std::ambient_authority())?;
        builder.preopened_dir(preopen_dir, ".")?;
        for (var, val) in test_programs::wasi_tests_environment() {
            builder.env(var, val)?;
        }

        let mut store = Store::new(&ENGINE, builder.build());
        let instance = linker.instantiate(&mut store, &get_module(name))?;
        let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
        start.call(&mut store, ())?;
        Ok(())
    };

    r.map_err(move |trap: anyhow::Error| {
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
    run("big_random_buf", true).unwrap()
}
#[test_log::test]
fn clock_time_get() {
    run("clock_time_get", true).unwrap()
}
#[test_log::test]
fn close_preopen() {
    run("close_preopen", true).unwrap()
}
#[test_log::test]
fn dangling_fd() {
    run("dangling_fd", true).unwrap()
}
#[test_log::test]
fn dangling_symlink() {
    run("dangling_symlink", true).unwrap()
}
#[test_log::test]
fn directory_seek() {
    run("directory_seek", true).unwrap()
}
#[test_log::test]
fn dir_fd_op_failures() {
    run("dir_fd_op_failures", true).unwrap()
}
#[test_log::test]
fn fd_advise() {
    run("fd_advise", true).unwrap()
}
#[test_log::test]
fn fd_filestat_get() {
    run("fd_filestat_get", true).unwrap()
}
#[test_log::test]
fn fd_filestat_set() {
    run("fd_filestat_set", true).unwrap()
}
#[test_log::test]
fn fd_flags_set() {
    run("fd_flags_set", true).unwrap()
}
#[test_log::test]
fn fd_readdir() {
    run("fd_readdir", true).unwrap()
}
#[test_log::test]
fn file_allocate() {
    run("file_allocate", true).unwrap()
}
#[test_log::test]
fn file_pread_pwrite() {
    run("file_pread_pwrite", true).unwrap()
}
#[test_log::test]
fn file_seek_tell() {
    run("file_seek_tell", true).unwrap()
}
#[test_log::test]
fn file_truncation() {
    run("file_truncation", true).unwrap()
}
#[test_log::test]
fn file_unbuffered_write() {
    run("file_unbuffered_write", true).unwrap()
}
#[test_log::test]
#[cfg_attr(windows, should_panic)]
fn interesting_paths() {
    run("interesting_paths", true).unwrap()
}
#[test_log::test]
fn isatty() {
    run("isatty", true).unwrap()
}
#[test_log::test]
fn nofollow_errors() {
    run("nofollow_errors", true).unwrap()
}
#[test_log::test]
fn overwrite_preopen() {
    run("overwrite_preopen", true).unwrap()
}
#[test_log::test]
fn path_exists() {
    run("path_exists", true).unwrap()
}
#[test_log::test]
fn path_filestat() {
    run("path_filestat", true).unwrap()
}
#[test_log::test]
fn path_link() {
    run("path_link", true).unwrap()
}
#[test_log::test]
fn path_open_create_existing() {
    run("path_open_create_existing", true).unwrap()
}
#[test_log::test]
fn path_open_read_write() {
    run("path_open_read_write", true).unwrap()
}
#[test_log::test]
fn path_open_dirfd_not_dir() {
    run("path_open_dirfd_not_dir", true).unwrap()
}
#[test_log::test]
fn path_open_missing() {
    run("path_open_missing", true).unwrap()
}
#[test_log::test]
fn path_open_nonblock() {
    run("path_open_nonblock", true).unwrap()
}
#[test_log::test]
fn path_rename_dir_trailing_slashes() {
    run("path_rename_dir_trailing_slashes", true).unwrap()
}
#[test_log::test]
#[should_panic]
fn path_rename_file_trailing_slashes() {
    run("path_rename_file_trailing_slashes", false).unwrap()
}
#[test_log::test]
fn path_rename() {
    run("path_rename", true).unwrap()
}
#[test_log::test]
fn path_symlink_trailing_slashes() {
    run("path_symlink_trailing_slashes", true).unwrap()
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
    run("readlink", true).unwrap()
}
#[test_log::test]
#[should_panic]
fn remove_directory_trailing_slashes() {
    run("remove_directory_trailing_slashes", false).unwrap()
}
#[test_log::test]
fn remove_nonempty_directory() {
    run("remove_nonempty_directory", true).unwrap()
}
#[test_log::test]
fn renumber() {
    run("renumber", true).unwrap()
}
#[test_log::test]
fn sched_yield() {
    run("sched_yield", true).unwrap()
}
#[test_log::test]
fn stdio() {
    run("stdio", true).unwrap()
}
#[test_log::test]
fn symlink_create() {
    run("symlink_create", true).unwrap()
}
#[test_log::test]
fn symlink_filestat() {
    run("symlink_filestat", true).unwrap()
}
#[test_log::test]
fn symlink_loop() {
    run("symlink_loop", true).unwrap()
}
#[test_log::test]
fn unlink_file_trailing_slashes() {
    run("unlink_file_trailing_slashes", true).unwrap()
}
#[test_log::test]
fn path_open_preopen() {
    run("path_open_preopen", true).unwrap()
}
