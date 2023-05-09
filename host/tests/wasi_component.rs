use anyhow::Result;
use cap_std::{ambient_authority, fs::Dir};
use wasi_common::wasi::command::add_to_linker;
use wasi_common::{wasi::command::Command, Table, WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};

lazy_static::lazy_static! {
    static ref ENGINE: Engine = {
        let mut config = Config::new();
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(true);
        config.async_support(true);

        let engine = Engine::new(&config).unwrap();
        engine
    };
}
// uses ENGINE, creates a fn get_component(&str) -> Component
test_programs::wasi_tests_components!();

struct CommandCtx {
    table: Table,
    wasi: WasiCtx,
}

impl WasiView for CommandCtx {
    fn table(&self) -> &Table {
        &self.table
    }
    fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }
    fn ctx(&self) -> &WasiCtx {
        &self.wasi
    }
    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

async fn instantiate(
    component: Component,
    ctx: CommandCtx,
) -> Result<(Store<CommandCtx>, Command)> {
    let mut linker = Linker::new(&ENGINE);
    add_to_linker(&mut linker)?;

    let mut store = Store::new(&ENGINE, ctx);

    let (command, _instance) = Command::instantiate_async(&mut store, &component, &linker).await?;
    Ok((store, command))
}
async fn run_with_temp_dir(component: &str) {
    let mut builder = WasiCtxBuilder::new().push_env("NO_FDFLAGS_SYNC_SUPPORT", "1");

    if cfg!(windows) {
        builder = builder
            .inherit_stdio()
            .push_env("ERRNO_MODE_WINDOWS", "1")
            .push_env("NO_DANGLING_FILESYSTEM", "1")
            .push_env("NO_RENAME_DIR_TO_EMPTY_DIR", "1");
    }
    if cfg!(all(unix, not(target_os = "macos"))) {
        builder = builder.push_env("ERRNO_MODE_UNIX", "1");
    }
    if cfg!(target_os = "macos") {
        builder = builder.push_env("ERRNO_MODE_MACOS", "1");
    }

    let dir = tempfile::tempdir().expect("create tempdir");
    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority()).expect("open dir");

    let mut table = Table::new();
    let wasi = builder
        .push_preopened_dir(
            open_dir,
            wasi_common::DirPerms::all(),
            wasi_common::FilePerms::all(),
            "/foo",
        )
        .set_args(&["program", "/foo"])
        .build(&mut table)
        .expect("build wasi ctx");

    let (mut store, command) = instantiate(get_component(component), CommandCtx { table, wasi })
        .await
        .expect("instantiate");

    command
        .call_run(&mut store)
        .await
        .expect("command did not trap")
        .expect("command returned success");
}

#[test_log::test(tokio::test)]
async fn big_random_buf() {
    run_with_temp_dir("big_random_buf").await
}

#[test_log::test(tokio::test)]
async fn clock_time_get() {
    run_with_temp_dir("clock_time_get").await
}

#[test_log::test(tokio::test)]
async fn close_preopen() {
    run_with_temp_dir("close_preopen").await
}

#[test_log::test(tokio::test)]
async fn overwrite_preopen() {
    run_with_temp_dir("overwrite_preopen").await
}

#[test_log::test(tokio::test)]
async fn dangling_fd() {
    run_with_temp_dir("dangling_fd").await
}

#[test_log::test(tokio::test)]
async fn dangling_symlink() {
    run_with_temp_dir("dangling_symlink").await
}

#[test_log::test(tokio::test)]
async fn directory_seek() {
    run_with_temp_dir("directory_seek").await
}

#[test_log::test(tokio::test)]
async fn fd_advise() {
    run_with_temp_dir("fd_advise").await
}

#[test_log::test(tokio::test)]
async fn fd_filestat_get() {
    run_with_temp_dir("fd_filestat_get").await
}

#[test_log::test(tokio::test)]
async fn fd_filestat_set() {
    run_with_temp_dir("fd_filestat_set").await
}

#[test_log::test(tokio::test)]
async fn fd_flags_set() {
    run_with_temp_dir("fd_flags_set").await
}

#[test_log::test(tokio::test)]
async fn fd_readdir() {
    run_with_temp_dir("fd_readdir").await
}

#[test_log::test(tokio::test)]
async fn file_allocate() {
    run_with_temp_dir("file_allocate").await
}

#[test_log::test(tokio::test)]
async fn file_pread_pwrite() {
    run_with_temp_dir("file_pread_pwrite").await
}

#[test_log::test(tokio::test)]
async fn file_seek_tell() {
    run_with_temp_dir("file_seek_tell").await
}

#[test_log::test(tokio::test)]
async fn file_truncation() {
    run_with_temp_dir("file_truncation").await
}

#[test_log::test(tokio::test)]
async fn file_unbuffered_write() {
    run_with_temp_dir("file_unbuffered_write").await
}

#[test_log::test(tokio::test)]
#[cfg_attr(windows, should_panic)]
async fn interesting_paths() {
    run_with_temp_dir("interesting_paths").await
}

#[test_log::test(tokio::test)]
async fn isatty() {
    run_with_temp_dir("isatty").await
}

#[test_log::test(tokio::test)]
async fn nofollow_errors() {
    run_with_temp_dir("nofollow_errors").await
}

#[test_log::test(tokio::test)]
async fn path_exists() {
    run_with_temp_dir("path_exists").await
}

#[test_log::test(tokio::test)]
async fn path_filestat() {
    run_with_temp_dir("path_filestat").await
}

#[test_log::test(tokio::test)]
async fn path_link() {
    run_with_temp_dir("path_link").await
}

#[test_log::test(tokio::test)]
async fn path_open_create_existing() {
    run_with_temp_dir("path_open_create_existing").await
}

#[test_log::test(tokio::test)]
async fn path_open_dirfd_not_dir() {
    run_with_temp_dir("path_open_dirfd_not_dir").await
}

#[test_log::test(tokio::test)]
async fn path_open_missing() {
    run_with_temp_dir("path_open_missing").await
}

#[test_log::test(tokio::test)]
async fn path_rename() {
    run_with_temp_dir("path_rename").await
}

#[test_log::test(tokio::test)]
async fn path_rename_dir_trailing_slashes() {
    run_with_temp_dir("path_rename_dir_trailing_slashes").await
}

#[test_log::test(tokio::test)]
#[should_panic]
async fn path_rename_file_trailing_slashes() {
    // renaming a file with trailing slash in destination name expected to fail, but succeeds: line 18
    run_with_temp_dir("path_rename_file_trailing_slashes").await
}

#[test_log::test(tokio::test)]
async fn path_symlink_trailing_slashes() {
    run_with_temp_dir("path_symlink_trailing_slashes").await
}

#[test_log::test(tokio::test)]
#[cfg_attr(windows, should_panic)]
async fn poll_oneoff_files() {
    run_with_temp_dir("poll_oneoff_files").await
}

#[test_log::test(tokio::test)]
#[should_panic]
async fn poll_oneoff_stdio() {
    run_with_temp_dir("poll_oneoff_stdio").await
}

#[test_log::test(tokio::test)]
async fn readlink() {
    run_with_temp_dir("readlink").await
}

#[test_log::test(tokio::test)]
#[should_panic]
async fn remove_directory_trailing_slashes() {
    // removing a directory with a trailing slash in the path succeeded under preview 1,
    // fails now returning INVAL
    run_with_temp_dir("remove_directory_trailing_slashes").await
}

#[test_log::test(tokio::test)]
async fn remove_nonempty_directory() {
    run_with_temp_dir("remove_nonempty_directory").await
}

#[test_log::test(tokio::test)]
async fn renumber() {
    run_with_temp_dir("renumber").await
}

#[test_log::test(tokio::test)]
async fn sched_yield() {
    run_with_temp_dir("sched_yield").await
}

#[test_log::test(tokio::test)]
async fn stdio() {
    run_with_temp_dir("stdio").await
}

#[test_log::test(tokio::test)]
async fn symlink_create() {
    run_with_temp_dir("symlink_create").await
}

#[test_log::test(tokio::test)]
async fn symlink_filestat() {
    run_with_temp_dir("symlink_filestat").await
}

#[test_log::test(tokio::test)]
async fn symlink_loop() {
    run_with_temp_dir("symlink_loop").await
}

#[test_log::test(tokio::test)]
async fn unlink_file_trailing_slashes() {
    run_with_temp_dir("unlink_file_trailing_slashes").await
}

#[test_log::test(tokio::test)]
async fn dir_fd_op_failures() {
    run_with_temp_dir("dir_fd_op_failures").await
}
