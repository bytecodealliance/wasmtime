#![cfg(feature = "test_programs")]
use anyhow::Result;
use tempfile::TempDir;
use wasi_common::pipe::WritePipe;
use wasmtime::{Config, Engine, Linker, Store};
use wasmtime_wasi::tokio::{add_to_linker, WasiCtxBuilder};

lazy_static::lazy_static! {
    static ref ENGINE: Engine = {
        let mut config = Config::new();
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(false);
        config.async_support(true);

        let engine = Engine::new(&config).unwrap();
        engine
    };
}
// uses ENGINE, creates a fn get_module(&str) -> Module
include!(concat!(env!("OUT_DIR"), "/wasi_tests_modules.rs"));

pub fn prepare_workspace(exe_name: &str) -> Result<TempDir> {
    let prefix = format!("wasi_tokio_{}_", exe_name);
    let tempdir = tempfile::Builder::new().prefix(&prefix).tempdir()?;
    Ok(tempdir)
}

async fn run(name: &str, inherit_stdio: bool) -> Result<()> {
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
        let instance = linker
            .instantiate_async(&mut store, &get_module(name))
            .await?;
        let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
        start.call_async(&mut store, ()).await?;
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
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn big_random_buf() {
    run("big_random_buf", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn clock_time_get() {
    run("clock_time_get", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn close_preopen() {
    run("close_preopen", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn dangling_fd() {
    run("dangling_fd", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn dangling_symlink() {
    run("dangling_symlink", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn directory_seek() {
    run("directory_seek", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn dir_fd_op_failures() {
    run("dir_fd_op_failures", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn fd_advise() {
    run("fd_advise", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn fd_filestat_get() {
    run("fd_filestat_get", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn fd_filestat_set() {
    run("fd_filestat_set", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn fd_flags_set() {
    run("fd_flags_set", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn fd_readdir() {
    run("fd_readdir", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn file_allocate() {
    run("file_allocate", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn file_pread_pwrite() {
    run("file_pread_pwrite", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn file_seek_tell() {
    run("file_seek_tell", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn file_truncation() {
    run("file_truncation", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn file_unbuffered_write() {
    run("file_unbuffered_write", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
#[cfg_attr(windows, should_panic)]
async fn interesting_paths() {
    run("interesting_paths", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn isatty() {
    run("isatty", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn nofollow_errors() {
    run("nofollow_errors", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn overwrite_preopen() {
    run("overwrite_preopen", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn path_exists() {
    run("path_exists", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn path_filestat() {
    run("path_filestat", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn path_link() {
    run("path_link", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn path_open_create_existing() {
    run("path_open_create_existing", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn path_open_read_write() {
    run("path_open_read_write", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn path_open_dirfd_not_dir() {
    run("path_open_dirfd_not_dir", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn path_open_missing() {
    run("path_open_missing", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn path_open_nonblock() {
    run("path_open_nonblock", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn path_rename_dir_trailing_slashes() {
    run("path_rename_dir_trailing_slashes", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
#[should_panic]
async fn path_rename_file_trailing_slashes() {
    run("path_rename_file_trailing_slashes", false)
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn path_rename() {
    run("path_rename", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn path_symlink_trailing_slashes() {
    run("path_symlink_trailing_slashes", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn poll_oneoff_files() {
    run("poll_oneoff_files", false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn poll_oneoff_stdio() {
    run("poll_oneoff_stdio", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn readlink() {
    run("readlink", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
#[should_panic]
async fn remove_directory_trailing_slashes() {
    run("remove_directory_trailing_slashes", false)
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn remove_nonempty_directory() {
    run("remove_nonempty_directory", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn renumber() {
    run("renumber", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn sched_yield() {
    run("sched_yield", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn stdio() {
    run("stdio", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn symlink_create() {
    run("symlink_create", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn symlink_filestat() {
    run("symlink_filestat", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn symlink_loop() {
    run("symlink_loop", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn unlink_file_trailing_slashes() {
    run("unlink_file_trailing_slashes", true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn path_open_preopen() {
    run("path_open_preopen", true).await.unwrap()
}
