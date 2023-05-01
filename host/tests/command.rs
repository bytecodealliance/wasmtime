use anyhow::Result;
use cap_rand::RngCore;
use cap_std::{ambient_authority, fs::Dir, time::Duration};
use std::{
    io::{Cursor, Write},
    sync::Mutex,
};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{
    clocks::{WasiMonotonicClock, WasiWallClock},
    dir::ReadOnlyDir,
    pipe::ReadPipe,
    wasi::command::add_to_linker,
    wasi::command::Command,
    Table, WasiCtx, WasiView,
};
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
test_programs_macros::command_components!();

// A bunch of these test cases are expected to fail. We wrap up their execution in this
// function so that we see if changes make them start passing.
// Note that we need to be careful not to check in any tests that panic for this approach
// to work.
fn expect_fail(r: Result<()>) -> Result<()> {
    match r {
        Ok(_) => Err(anyhow::anyhow!("expected failure")),
        Err(_) => Ok(()),
    }
}

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

#[test_log::test(tokio::test)]
async fn hello_stdout() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .args(&["gussie", "sparky", "willa"])
        .build(&mut table)?;
    let (mut store, command) =
        instantiate(get_component("hello_stdout"), CommandCtx { table, wasi }).await?;
    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn panic() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .args(&[
            "diesel",
            "the",
            "cat",
            "scratched",
            "me",
            "real",
            "good",
            "yesterday",
        ])
        .build(&mut table)?;
    let (mut store, command) =
        instantiate(get_component("panic"), CommandCtx { table, wasi }).await?;
    let r = command.call_main(&mut store).await;
    assert!(r.is_err());
    println!("{:?}", r);
    Ok(())
}

#[test_log::test(tokio::test)]
async fn args() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .args(&["hello", "this", "", "is an argument", "with 🚩 emoji"])
        .build(&mut table)?;
    let (mut store, command) =
        instantiate(get_component("args"), CommandCtx { table, wasi }).await?;
    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn random() -> Result<()> {
    struct FakeRng;

    impl RngCore for FakeRng {
        fn next_u32(&mut self) -> u32 {
            42
        }

        fn next_u64(&mut self) -> u64 {
            unimplemented!()
        }

        fn fill_bytes(&mut self, _dest: &mut [u8]) {
            unimplemented!()
        }

        fn try_fill_bytes(&mut self, _dest: &mut [u8]) -> Result<(), cap_rand::Error> {
            unimplemented!()
        }
    }

    let mut table = Table::new();
    let mut wasi = WasiCtxBuilder::new().build(&mut table)?;
    wasi.random = Box::new(FakeRng);
    let (mut store, command) =
        instantiate(get_component("random"), CommandCtx { table, wasi }).await?;

    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn time() -> Result<()> {
    struct FakeWallClock;

    impl WasiWallClock for FakeWallClock {
        fn resolution(&self) -> Duration {
            Duration::from_secs(1)
        }

        fn now(&self) -> Duration {
            Duration::new(1431648000, 100)
        }
    }

    struct FakeMonotonicClock {
        now: Mutex<u64>,
    }

    impl WasiMonotonicClock for FakeMonotonicClock {
        fn resolution(&self) -> u64 {
            1_000_000_000
        }

        fn now(&self) -> u64 {
            let mut now = self.now.lock().unwrap();
            let then = *now;
            *now += 42 * 1_000_000_000;
            then
        }
    }

    let mut table = Table::new();
    let mut wasi = WasiCtxBuilder::new().build(&mut table)?;
    wasi.clocks.wall = Box::new(FakeWallClock);
    wasi.clocks.monotonic = Box::new(FakeMonotonicClock { now: Mutex::new(0) });

    let (mut store, command) =
        instantiate(get_component("time"), CommandCtx { table, wasi }).await?;

    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn stdin() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .stdin(ReadPipe::new(Cursor::new(
            "So rested he by the Tumtum tree",
        )))
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("stdin"), CommandCtx { table, wasi }).await?;

    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn poll_stdin() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .stdin(ReadPipe::new(Cursor::new(
            "So rested he by the Tumtum tree",
        )))
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("poll_stdin"), CommandCtx { table, wasi }).await?;

    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn env() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .push_env("frabjous", "day")
        .push_env("callooh", "callay")
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("env"), CommandCtx { table, wasi }).await?;

    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn file_read() -> Result<()> {
    let dir = tempfile::tempdir()?;

    std::fs::File::create(dir.path().join("bar.txt"))?.write_all(b"And stood awhile in thought")?;

    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority())?;

    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .preopened_dir(open_dir, "/")
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("file_read"), CommandCtx { table, wasi }).await?;

    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn file_append() -> Result<()> {
    let dir = tempfile::tempdir()?;

    std::fs::File::create(dir.path().join("bar.txt"))?
        .write_all(b"'Twas brillig, and the slithy toves.\n")?;

    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority())?;

    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .preopened_dir(open_dir, "/")
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("file_append"), CommandCtx { table, wasi }).await?;
    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))?;

    let contents = std::fs::read(dir.path().join("bar.txt"))?;
    assert_eq!(
        std::str::from_utf8(&contents).unwrap(),
        "'Twas brillig, and the slithy toves.\n\
               Did gyre and gimble in the wabe;\n\
               All mimsy were the borogoves,\n\
               And the mome raths outgrabe.\n"
    );
    Ok(())
}

#[test_log::test(tokio::test)]
async fn file_dir_sync() -> Result<()> {
    let dir = tempfile::tempdir()?;

    std::fs::File::create(dir.path().join("bar.txt"))?
        .write_all(b"'Twas brillig, and the slithy toves.\n")?;

    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority())?;

    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .preopened_dir(open_dir, "/")
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("file_dir_sync"), CommandCtx { table, wasi }).await?;

    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn exit_success() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new().build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("exit_success"), CommandCtx { table, wasi }).await?;

    let r = command.call_main(&mut store).await;
    let err = r.unwrap_err();
    let status = err.downcast_ref::<wasi_common::I32Exit>().unwrap();
    assert_eq!(status.0, 0);
    Ok(())
}

#[test_log::test(tokio::test)]
async fn exit_default() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new().build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("exit_default"), CommandCtx { table, wasi }).await?;

    let r = command.call_main(&mut store).await?;
    assert!(r.is_ok());
    Ok(())
}

#[test_log::test(tokio::test)]
async fn exit_failure() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new().build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("exit_failure"), CommandCtx { table, wasi }).await?;

    let r = command.call_main(&mut store).await;
    let err = r.unwrap_err();
    let status = err.downcast_ref::<wasi_common::I32Exit>().unwrap();
    assert_eq!(status.0, 1);
    Ok(())
}

#[test_log::test(tokio::test)]
async fn exit_panic() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new().build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("exit_panic"), CommandCtx { table, wasi }).await?;

    let r = command.call_main(&mut store).await;
    let err = r.unwrap_err();
    // The panic should trap.
    assert!(err.downcast_ref::<wasi_common::I32Exit>().is_none());
    Ok(())
}

#[test_log::test(tokio::test)]
async fn directory_list() -> Result<()> {
    let dir = tempfile::tempdir()?;

    std::fs::File::create(dir.path().join("foo.txt"))?;
    std::fs::File::create(dir.path().join("bar.txt"))?;
    std::fs::File::create(dir.path().join("baz.txt"))?;
    std::fs::create_dir(dir.path().join("sub"))?;
    std::fs::File::create(dir.path().join("sub").join("wow.txt"))?;
    std::fs::File::create(dir.path().join("sub").join("yay.txt"))?;

    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority())?;

    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .preopened_dir(open_dir, "/")
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("directory_list"), CommandCtx { table, wasi }).await?;

    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn default_clocks() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new().build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("default_clocks"), CommandCtx { table, wasi }).await?;

    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_with_temp_dir(component: &str) -> Result<()> {
    let mut builder = WasiCtxBuilder::new();

    if cfg!(windows) {
        builder = builder
            .push_env("ERRNO_MODE_WINDOWS", "1")
            .push_env("NO_FDFLAGS_SYNC_SUPPORT", "1")
            .push_env("NO_DANGLING_FILESYSTEM", "1")
            .push_env("NO_RENAME_DIR_TO_EMPTY_DIR", "1");
    }
    if cfg!(all(unix, not(target_os = "macos"))) {
        builder = builder.push_env("ERRNO_MODE_UNIX", "1");
    }
    if cfg!(target_os = "macos") {
        builder = builder.push_env("ERRNO_MODE_MACOS", "1");
    }

    let dir = tempfile::tempdir()?;
    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority())?;

    let mut table = Table::new();
    let wasi = builder
        .preopened_dir(open_dir, "/foo")
        .set_args(&["program", "/foo"])
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component(component), CommandCtx { table, wasi }).await?;

    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn big_random_buf() -> Result<()> {
    run_with_temp_dir("big_random_buf").await
}

#[test_log::test(tokio::test)]
async fn clock_time_get() -> Result<()> {
    run_with_temp_dir("clock_time_get").await
}

#[test_log::test(tokio::test)]
async fn close_preopen() -> Result<()> {
    run_with_temp_dir("close_preopen").await
}

#[test_log::test(tokio::test)]
async fn overwrite_preopen() -> Result<()> {
    run_with_temp_dir("overwrite_preopen").await
}

#[test_log::test(tokio::test)]
async fn dangling_fd() -> Result<()> {
    run_with_temp_dir("dangling_fd").await
}

#[test_log::test(tokio::test)]
async fn dangling_symlink() -> Result<()> {
    run_with_temp_dir("dangling_symlink").await
}

#[test_log::test(tokio::test)]
async fn directory_seek() -> Result<()> {
    run_with_temp_dir("directory_seek").await
}

#[test_log::test(tokio::test)]
async fn fd_advise() -> Result<()> {
    run_with_temp_dir("fd_advise").await
}

#[test_log::test(tokio::test)]
async fn fd_filestat_get() -> Result<()> {
    run_with_temp_dir("fd_filestat_get").await
}

#[test_log::test(tokio::test)]
async fn fd_filestat_set() -> Result<()> {
    run_with_temp_dir("fd_filestat_set").await
}

#[test_log::test(tokio::test)]
async fn fd_flags_set() -> Result<()> {
    run_with_temp_dir("fd_flags_set").await
}

#[test_log::test(tokio::test)]
async fn fd_readdir() -> Result<()> {
    run_with_temp_dir("fd_readdir").await
}

#[test_log::test(tokio::test)]
async fn file_allocate() -> Result<()> {
    run_with_temp_dir("file_allocate").await
}

#[test_log::test(tokio::test)]
async fn file_pread_pwrite() -> Result<()> {
    run_with_temp_dir("file_pread_pwrite").await
}

#[test_log::test(tokio::test)]
async fn file_seek_tell() -> Result<()> {
    run_with_temp_dir("file_seek_tell").await
}

#[test_log::test(tokio::test)]
async fn file_truncation() -> Result<()> {
    run_with_temp_dir("file_truncation").await
}

#[test_log::test(tokio::test)]
async fn file_unbuffered_write() -> Result<()> {
    run_with_temp_dir("file_unbuffered_write").await
}

#[test_log::test(tokio::test)]
async fn interesting_paths() -> Result<()> {
    if cfg!(windows) {
        expect_fail(run_with_temp_dir("interesting_paths").await)
    } else {
        run_with_temp_dir("interesting_paths").await
    }
}

#[test_log::test(tokio::test)]
async fn isatty() -> Result<()> {
    run_with_temp_dir("isatty").await
}

#[test_log::test(tokio::test)]
async fn nofollow_errors() -> Result<()> {
    run_with_temp_dir("nofollow_errors").await
}

#[test_log::test(tokio::test)]
async fn path_exists() -> Result<()> {
    run_with_temp_dir("path_exists").await
}

#[test_log::test(tokio::test)]
async fn path_filestat() -> Result<()> {
    run_with_temp_dir("path_filestat").await
}

#[test_log::test(tokio::test)]
async fn path_link() -> Result<()> {
    run_with_temp_dir("path_link").await
}

#[test_log::test(tokio::test)]
async fn path_open_create_existing() -> Result<()> {
    run_with_temp_dir("path_open_create_existing").await
}

#[test_log::test(tokio::test)]
async fn path_open_dirfd_not_dir() -> Result<()> {
    run_with_temp_dir("path_open_dirfd_not_dir").await
}

#[test_log::test(tokio::test)]
async fn path_open_missing() -> Result<()> {
    run_with_temp_dir("path_open_missing").await
}

#[test_log::test(tokio::test)]
async fn path_rename() -> Result<()> {
    run_with_temp_dir("path_rename").await
}

#[test_log::test(tokio::test)]
async fn path_rename_dir_trailing_slashes() -> Result<()> {
    run_with_temp_dir("path_rename_dir_trailing_slashes").await
}

#[test_log::test(tokio::test)]
async fn path_rename_file_trailing_slashes() -> Result<()> {
    // renaming a file with trailing slash in destination name expected to fail, but succeeds: line 18
    expect_fail(run_with_temp_dir("path_rename_file_trailing_slashes").await)
}

#[test_log::test(tokio::test)]
async fn path_symlink_trailing_slashes() -> Result<()> {
    run_with_temp_dir("path_symlink_trailing_slashes").await
}

#[test_log::test(tokio::test)]
async fn poll_oneoff_files() -> Result<()> {
    if cfg!(windows) {
        expect_fail(run_with_temp_dir("poll_oneoff_files").await)
    } else {
        run_with_temp_dir("poll_oneoff_files").await
    }
}

#[test_log::test(tokio::test)]
async fn poll_oneoff_stdio() -> Result<()> {
    expect_fail(run_with_temp_dir("poll_oneoff_stdio").await)
}

#[test_log::test(tokio::test)]
async fn readlink() -> Result<()> {
    run_with_temp_dir("readlink").await
}

#[test_log::test(tokio::test)]
async fn remove_directory_trailing_slashes() -> Result<()> {
    // removing a directory with a trailing slash in the path succeeded under preview 1,
    // fails now returning INVAL
    expect_fail(run_with_temp_dir("remove_directory_trailing_slashes").await)
}

#[test_log::test(tokio::test)]
async fn remove_nonempty_directory() -> Result<()> {
    run_with_temp_dir("remove_nonempty_directory").await
}

#[test_log::test(tokio::test)]
async fn renumber() -> Result<()> {
    run_with_temp_dir("renumber").await
}

#[test_log::test(tokio::test)]
async fn sched_yield() -> Result<()> {
    run_with_temp_dir("sched_yield").await
}

#[test_log::test(tokio::test)]
async fn stdio() -> Result<()> {
    run_with_temp_dir("stdio").await
}

#[test_log::test(tokio::test)]
async fn symlink_create() -> Result<()> {
    run_with_temp_dir("symlink_create").await
}

#[test_log::test(tokio::test)]
async fn symlink_filestat() -> Result<()> {
    run_with_temp_dir("symlink_filestat").await
}

#[test_log::test(tokio::test)]
async fn symlink_loop() -> Result<()> {
    run_with_temp_dir("symlink_loop").await
}

#[test_log::test(tokio::test)]
async fn unlink_file_trailing_slashes() -> Result<()> {
    run_with_temp_dir("unlink_file_trailing_slashes").await
}

#[test_log::test(tokio::test)]
async fn export_cabi_realloc() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new().build(&mut table)?;
    let (mut store, command) = instantiate(
        get_component("export_cabi_realloc"),
        CommandCtx { table, wasi },
    )
    .await?;

    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn read_only() -> Result<()> {
    let dir = tempfile::tempdir()?;

    std::fs::File::create(dir.path().join("bar.txt"))?.write_all(b"And stood awhile in thought")?;
    std::fs::create_dir(dir.path().join("sub"))?;

    let mut table = Table::new();
    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority())?;
    let wasi = WasiCtxBuilder::new()
        .preopened_dir_impl(
            ReadOnlyDir(Box::new(wasi_cap_std_sync::dir::Dir::from_cap_std(
                open_dir,
            ))),
            "/",
        )
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("read_only"), CommandCtx { table, wasi }).await?;

    command
        .call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn dir_fd_op_failures() -> Result<()> {
    run_with_temp_dir("dir_fd_op_failures").await
}
