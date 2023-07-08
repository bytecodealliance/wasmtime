use anyhow::Result;
use cap_std::{ambient_authority, fs::Dir, time::Duration};
use std::{
    io::{Cursor, Write},
    sync::Mutex,
};
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
use wasmtime_wasi::preview2::{
    clocks::{HostMonotonicClock, HostWallClock},
    pipe::ReadPipe,
    wasi::command::add_to_linker,
    wasi::command::Command,
    DirPerms, FilePerms, Table, WasiCtx, WasiCtxBuilder, WasiView,
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
include!(concat!(env!("OUT_DIR"), "/command_tests_components.rs"));

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
        .set_args(&["gussie", "sparky", "willa"])
        .build(&mut table)?;
    let (mut store, command) =
        instantiate(get_component("hello_stdout"), CommandCtx { table, wasi }).await?;
    command
        .call_run(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn panic() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .set_args(&[
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
    let r = command.call_run(&mut store).await;
    assert!(r.is_err());
    println!("{:?}", r);
    Ok(())
}

#[test_log::test(tokio::test)]
async fn args() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .set_args(&["hello", "this", "", "is an argument", "with 🚩 emoji"])
        .build(&mut table)?;
    let (mut store, command) =
        instantiate(get_component("args"), CommandCtx { table, wasi }).await?;
    command
        .call_run(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn random() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new().build(&mut table)?;
    let (mut store, command) =
        instantiate(get_component("random"), CommandCtx { table, wasi }).await?;

    command
        .call_run(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn time() -> Result<()> {
    struct FakeWallClock;

    impl HostWallClock for FakeWallClock {
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

    impl HostMonotonicClock for FakeMonotonicClock {
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
    let wasi = WasiCtxBuilder::new()
        .set_monotonic_clock(FakeMonotonicClock { now: Mutex::new(0) })
        .set_wall_clock(FakeWallClock)
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("time"), CommandCtx { table, wasi }).await?;

    command
        .call_run(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn stdin() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .set_stdin(ReadPipe::new(Cursor::new(
            "So rested he by the Tumtum tree",
        )))
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("stdin"), CommandCtx { table, wasi }).await?;

    command
        .call_run(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn poll_stdin() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .set_stdin(ReadPipe::new(Cursor::new(
            "So rested he by the Tumtum tree",
        )))
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("poll_stdin"), CommandCtx { table, wasi }).await?;

    command
        .call_run(&mut store)
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
        .call_run(&mut store)
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
        .push_preopened_dir(open_dir, DirPerms::all(), FilePerms::all(), "/")
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("file_read"), CommandCtx { table, wasi }).await?;

    command
        .call_run(&mut store)
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
        .push_preopened_dir(open_dir, DirPerms::all(), FilePerms::all(), "/")
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("file_append"), CommandCtx { table, wasi }).await?;
    command
        .call_run(&mut store)
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
        .push_preopened_dir(open_dir, DirPerms::all(), FilePerms::all(), "/")
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("file_dir_sync"), CommandCtx { table, wasi }).await?;

    command
        .call_run(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test)]
async fn exit_success() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new().build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("exit_success"), CommandCtx { table, wasi }).await?;

    let r = command.call_run(&mut store).await;
    let err = r.unwrap_err();
    let status = err
        .downcast_ref::<wasmtime_wasi::preview2::I32Exit>()
        .unwrap();
    assert_eq!(status.0, 0);
    Ok(())
}

#[test_log::test(tokio::test)]
async fn exit_default() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new().build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("exit_default"), CommandCtx { table, wasi }).await?;

    let r = command.call_run(&mut store).await?;
    assert!(r.is_ok());
    Ok(())
}

#[test_log::test(tokio::test)]
async fn exit_failure() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new().build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("exit_failure"), CommandCtx { table, wasi }).await?;

    let r = command.call_run(&mut store).await;
    let err = r.unwrap_err();
    let status = err
        .downcast_ref::<wasmtime_wasi::preview2::I32Exit>()
        .unwrap();
    assert_eq!(status.0, 1);
    Ok(())
}

#[test_log::test(tokio::test)]
async fn exit_panic() -> Result<()> {
    let mut table = Table::new();
    let wasi = WasiCtxBuilder::new().build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("exit_panic"), CommandCtx { table, wasi }).await?;

    let r = command.call_run(&mut store).await;
    let err = r.unwrap_err();
    // The panic should trap.
    assert!(err
        .downcast_ref::<wasmtime_wasi::preview2::I32Exit>()
        .is_none());
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
        .inherit_stdio()
        .push_preopened_dir(open_dir, DirPerms::all(), FilePerms::all(), "/")
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("directory_list"), CommandCtx { table, wasi }).await?;

    command
        .call_run(&mut store)
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
        .call_run(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
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
        .call_run(&mut store)
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
        .push_preopened_dir(open_dir, DirPerms::READ, FilePerms::READ, "/")
        .build(&mut table)?;

    let (mut store, command) =
        instantiate(get_component("read_only"), CommandCtx { table, wasi }).await?;

    command
        .call_run(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}
