use anyhow::Result;
use cap_rand::RngCore;
use cap_std::{ambient_authority, fs::Dir, time::Duration};
use host::{command::add_to_linker, command::wasi::Command, WasiCtx};
use std::{
    io::{Cursor, Write},
    sync::Mutex,
};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{
    clocks::{WasiMonotonicClock, WasiWallClock},
    dir::ReadOnlyDir,
    pipe::ReadPipe,
};
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};

test_programs_macros::command_tests!();

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

async fn instantiate(path: &str) -> Result<(Store<WasiCtx>, Command)> {
    println!("{}", path);

    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.async_support(true);

    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, &path)?;
    let mut linker = Linker::new(&engine);
    add_to_linker(&mut linker, |x| x)?;

    let mut store = Store::new(
        &engine,
        WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build(),
    );

    let (wasi, _instance) = Command::instantiate_async(&mut store, &component, &linker).await?;
    Ok((store, wasi))
}

async fn run_hello_stdout(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    store.data_mut().set_args(&["gussie", "sparky", "willa"]);
    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_panic(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    store.data_mut().set_args(&[
        "diesel",
        "the",
        "cat",
        "scratched",
        "me",
        "real",
        "good",
        "yesterday",
    ]);
    let r = wasi.call_main(&mut store).await;
    assert!(r.is_err());
    println!("{:?}", r);
    Ok(())
}

async fn run_args(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    store
        .data_mut()
        .set_args(&["hello", "this", "", "is an argument", "with 🚩 emoji"]);
    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_random(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
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

    store.data_mut().random = Box::new(FakeRng);

    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_time(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
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

    store.data_mut().clocks.wall = Box::new(FakeWallClock);
    store.data_mut().clocks.monotonic = Box::new(FakeMonotonicClock { now: Mutex::new(0) });

    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_stdin(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    store
        .data_mut()
        .set_stdin(Box::new(ReadPipe::new(Cursor::new(
            "So rested he by the Tumtum tree",
        ))));

    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_poll_stdin(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    store
        .data_mut()
        .set_stdin(Box::new(ReadPipe::new(Cursor::new(
            "So rested he by the Tumtum tree",
        ))));

    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_env(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    store.data_mut().push_env("frabjous", "day");
    store.data_mut().push_env("callooh", "callay");
    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_file_read(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    let dir = tempfile::tempdir()?;

    std::fs::File::create(dir.path().join("bar.txt"))?.write_all(b"And stood awhile in thought")?;

    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority())?;
    store.data_mut().push_preopened_dir(
        Box::new(wasi_cap_std_sync::dir::Dir::from_cap_std(open_dir)),
        "/",
    )?;

    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_file_append(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    let dir = tempfile::tempdir()?;

    std::fs::File::create(dir.path().join("bar.txt"))?
        .write_all(b"'Twas brillig, and the slithy toves.\n")?;

    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority())?;
    store.data_mut().push_preopened_dir(
        Box::new(wasi_cap_std_sync::dir::Dir::from_cap_std(open_dir)),
        "/",
    )?;

    wasi.call_main(&mut store)
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

async fn run_file_dir_sync(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    let dir = tempfile::tempdir()?;

    std::fs::File::create(dir.path().join("bar.txt"))?
        .write_all(b"'Twas brillig, and the slithy toves.\n")?;

    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority())?;
    store.data_mut().push_preopened_dir(
        Box::new(wasi_cap_std_sync::dir::Dir::from_cap_std(open_dir)),
        "/",
    )?;

    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_exit_success(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    let r = wasi.call_main(&mut store).await;
    let err = r.unwrap_err();
    let status = err.downcast_ref::<wasi_common::I32Exit>().unwrap();
    assert_eq!(status.0, 0);
    Ok(())
}

async fn run_exit_default(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    let r = wasi.call_main(&mut store).await?;
    assert!(r.is_ok());
    Ok(())
}

async fn run_exit_failure(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    let r = wasi.call_main(&mut store).await;
    let err = r.unwrap_err();
    let status = err.downcast_ref::<wasi_common::I32Exit>().unwrap();
    assert_eq!(status.0, 1);
    Ok(())
}

async fn run_exit_panic(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    let r = wasi.call_main(&mut store).await;
    let err = r.unwrap_err();
    // The panic should trap.
    assert!(err.downcast_ref::<wasi_common::I32Exit>().is_none());
    Ok(())
}

async fn run_directory_list(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    let dir = tempfile::tempdir()?;

    std::fs::File::create(dir.path().join("foo.txt"))?;
    std::fs::File::create(dir.path().join("bar.txt"))?;
    std::fs::File::create(dir.path().join("baz.txt"))?;
    std::fs::create_dir(dir.path().join("sub"))?;
    std::fs::File::create(dir.path().join("sub").join("wow.txt"))?;
    std::fs::File::create(dir.path().join("sub").join("yay.txt"))?;

    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority())?;
    store.data_mut().push_preopened_dir(
        Box::new(wasi_cap_std_sync::dir::Dir::from_cap_std(open_dir)),
        "/",
    )?;

    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_default_clocks(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_with_temp_dir(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    let dir = tempfile::tempdir()?;

    store.data_mut().push_env("NO_RIGHTS_READBACK_SUPPORT", "1");

    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority())?;
    store.data_mut().push_preopened_dir(
        Box::new(wasi_cap_std_sync::dir::Dir::from_cap_std(open_dir)),
        "/foo",
    )?;
    store.data_mut().set_args(&["program", "/foo"]);

    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_big_random_buf(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_clock_time_get(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_close_preopen(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_overwrite_preopen(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_dangling_fd(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    if cfg!(windows) {
        expect_fail(run_with_temp_dir(store, wasi).await)
    } else {
        run_with_temp_dir(store, wasi).await
    }
}

async fn run_dangling_symlink(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    if cfg!(windows) {
        expect_fail(run_with_temp_dir(store, wasi).await)
    } else {
        run_with_temp_dir(store, wasi).await
    }
}

async fn run_directory_seek(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_fd_advise(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_fd_filestat_get(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_fd_filestat_set(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_fd_flags_set(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_fd_readdir(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_file_allocate(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_file_pread_pwrite(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_file_seek_tell(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_file_truncation(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_file_unbuffered_write(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_interesting_paths(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    if cfg!(windows) {
        expect_fail(run_with_temp_dir(store, wasi).await)
    } else {
        run_with_temp_dir(store, wasi).await
    }
}

async fn run_isatty(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_nofollow_errors(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_path_exists(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_path_filestat(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_path_link(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    if cfg!(windows) {
        expect_fail(run_with_temp_dir(store, wasi).await)
    } else {
        run_with_temp_dir(store, wasi).await
    }
}

async fn run_path_open_create_existing(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_path_open_dirfd_not_dir(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_path_open_missing(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_path_open_read_without_rights(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_path_rename(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    if cfg!(windows) {
        expect_fail(run_with_temp_dir(store, wasi).await)
    } else {
        run_with_temp_dir(store, wasi).await
    }
}

async fn run_path_rename_dir_trailing_slashes(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_path_rename_file_trailing_slashes(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_path_symlink_trailing_slashes(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    if cfg!(windows) {
        expect_fail(run_with_temp_dir(store, wasi).await)
    } else {
        run_with_temp_dir(store, wasi).await
    }
}

async fn run_poll_oneoff_files(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_poll_oneoff_stdio(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    if cfg!(windows) {
        expect_fail(run_with_temp_dir(store, wasi).await)
    } else {
        run_with_temp_dir(store, wasi).await
    }
}

async fn run_readlink(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_remove_directory_trailing_slashes(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_remove_nonempty_directory(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_renumber(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_sched_yield(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_stdio(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_symlink_create(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_symlink_filestat(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_symlink_loop(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    if cfg!(windows) {
        expect_fail(run_with_temp_dir(store, wasi).await)
    } else {
        run_with_temp_dir(store, wasi).await
    }
}

async fn run_truncation_rights(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    expect_fail(run_with_temp_dir(store, wasi).await)
}

async fn run_unlink_file_trailing_slashes(store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    run_with_temp_dir(store, wasi).await
}

async fn run_export_cabi_realloc(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

async fn run_read_only(mut store: Store<WasiCtx>, wasi: Command) -> Result<()> {
    let dir = tempfile::tempdir()?;

    std::fs::File::create(dir.path().join("bar.txt"))?.write_all(b"And stood awhile in thought")?;
    std::fs::create_dir(dir.path().join("sub"))?;

    let open_dir = Dir::open_ambient_dir(dir.path(), ambient_authority())?;
    store.data_mut().push_preopened_dir(
        Box::new(ReadOnlyDir(Box::new(
            wasi_cap_std_sync::dir::Dir::from_cap_std(open_dir),
        ))),
        "/",
    )?;

    wasi.call_main(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))?;

    Ok(())
}
