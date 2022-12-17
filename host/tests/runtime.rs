use anyhow::Result;
use cap_rand::RngCore;
use cap_std::time::{Duration, Instant, SystemTime};
use host::{add_to_linker, Wasi, WasiCtx};
use std::{io::Cursor, sync::Mutex};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{
    clocks::{WasiMonotonicClock, WasiSystemClock},
    pipe::ReadPipe,
};
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};

test_programs_macros::tests!();

async fn instantiate(path: &str) -> Result<(Store<WasiCtx>, Wasi)> {
    println!("{}", path);

    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.async_support(true);

    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, &path)?;
    let mut linker = Linker::new(&engine);
    add_to_linker(&mut linker, |x| x)?;

    let mut store = Store::new(&engine, WasiCtxBuilder::new().build());

    let (wasi, _instance) = Wasi::instantiate_async(&mut store, &component, &linker).await?;
    Ok((store, wasi))
}

async fn run_hello_stdout(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    wasi.command(
        &mut store,
        0 as host::Descriptor,
        1 as host::Descriptor,
        &["gussie", "sparky", "willa"],
    )
    .await?;
    Ok(())
}

async fn run_panic(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    let r = wasi
        .command(
            &mut store,
            0 as host::Descriptor,
            1 as host::Descriptor,
            &[
                "diesel",
                "the",
                "cat",
                "scratched",
                "me",
                "real",
                "good",
                "yesterday",
            ],
        )
        .await;
    assert!(r.is_err());
    println!("{:?}", r);
    Ok(())
}

async fn run_args(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    wasi.command(
        &mut store,
        0 as host::Descriptor,
        1 as host::Descriptor,
        &["hello", "this", "", "is an argument", "with 🚩 emoji"],
    )
    .await?;
    Ok(())
}

async fn run_random(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
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

    wasi.command(
        &mut store,
        0 as host::Descriptor,
        1 as host::Descriptor,
        &[],
    )
    .await?;
    Ok(())
}

async fn run_time(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    struct FakeSystemClock;

    impl WasiSystemClock for FakeSystemClock {
        fn resolution(&self) -> Duration {
            Duration::from_secs(1)
        }

        fn now(&self, _precision: Duration) -> SystemTime {
            SystemTime::from_std(std::time::SystemTime::UNIX_EPOCH)
                + Duration::from_secs(1431648000)
        }
    }

    struct FakeMonotonicClock {
        now: Mutex<Instant>,
    }

    impl WasiMonotonicClock for FakeMonotonicClock {
        fn resolution(&self) -> Duration {
            Duration::from_secs(1)
        }

        fn now(&self, _precision: Duration) -> Instant {
            let mut now = self.now.lock().unwrap();
            let then = *now;
            *now += Duration::from_secs(42);
            then
        }
    }

    store.data_mut().clocks.system = Box::new(FakeSystemClock);
    store.data_mut().clocks.monotonic = Box::new(FakeMonotonicClock {
        now: Mutex::new(Instant::from_std(std::time::Instant::now())),
    });

    wasi.command(
        &mut store,
        0 as host::Descriptor,
        1 as host::Descriptor,
        &[],
    )
    .await?;
    Ok(())
}

async fn run_stdin(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    store
        .data_mut()
        .set_stdin(Box::new(ReadPipe::new(Cursor::new(
            "So rested he by the Tumtum tree",
        ))));

    wasi.command(
        &mut store,
        0 as host::Descriptor,
        1 as host::Descriptor,
        &[],
    )
    .await?;
    Ok(())
}
