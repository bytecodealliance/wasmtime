use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU32, Ordering::Relaxed},
};
use std::time::Duration;

use super::util::{config, make_component};
use anyhow::{Result, anyhow};
use component_async_tests::Ctx;
use component_async_tests::util::sleep;
use futures::{
    FutureExt,
    channel::oneshot,
    stream::{FuturesUnordered, TryStreamExt},
};
use wasmtime::component::{Accessor, AccessorTask, HasSelf, Instance, Linker, ResourceTable, Val};
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::WasiCtxBuilder;

#[tokio::test]
pub async fn async_round_trip_stackful() -> Result<()> {
    test_round_trip_uncomposed(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT).await
}

#[tokio::test]
pub async fn async_round_trip_synchronous() -> Result<()> {
    test_round_trip_uncomposed(test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT)
        .await
}

#[tokio::test]
pub async fn async_round_trip_wait() -> Result<()> {
    test_round_trip_uncomposed(test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT).await
}

#[tokio::test]
pub async fn async_round_trip_stackless_plus_stackless() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_synchronous_plus_stackless() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_stackless_plus_synchronous() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_synchronous_plus_synchronous() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_wait_plus_wait() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_synchronous_plus_wait() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_wait_plus_synchronous() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_stackless_plus_wait() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_wait_plus_stackless() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_stackful_plus_stackful() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_stackful_plus_stackless() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_stackless_plus_stackful() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_synchronous_plus_stackful() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_stackful_plus_synchronous() -> Result<()> {
    test_round_trip_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT,
    )
    .await
}

#[tokio::test]
pub async fn async_round_trip_stackless() -> Result<()> {
    test_round_trip_uncomposed(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT).await
}

#[tokio::test]
pub async fn async_round_trip_stackless_joined() -> Result<()> {
    tokio::join!(
        async {
            test_round_trip_uncomposed(
                test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT,
            )
            .await
            .unwrap()
        },
        async {
            test_round_trip_uncomposed(
                test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT,
            )
            .await
            .unwrap()
        },
    );

    Ok(())
}

#[tokio::test]
pub async fn async_round_trip_stackless_sync_import() -> Result<()> {
    test_round_trip_uncomposed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_SYNC_IMPORT_COMPONENT,
    )
    .await
}

pub async fn test_round_trip(
    components: &[&str],
    inputs_and_outputs: &[(&str, &str)],
) -> Result<()> {
    let engine = Engine::new(&config())?;

    let make_store = || {
        Store::new(
            &engine,
            Ctx {
                wasi: WasiCtxBuilder::new().inherit_stdio().build(),
                table: ResourceTable::default(),
                continue_: false,
                wakers: Arc::new(Mutex::new(None)),
            },
        )
    };

    let component = make_component(&engine, components).await?;

    // On miri, we only use one call style per test since they take so long to
    // run.  On non-miri, we use every call style for each test.
    static CALL_STYLE_COUNTER: AtomicU32 = AtomicU32::new(0);
    let call_style = CALL_STYLE_COUNTER.fetch_add(1, Relaxed) % 5;

    // First, test the `wasmtime-wit-bindgen` static API:
    {
        let mut linker = Linker::new(&engine);

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        component_async_tests::round_trip::bindings::local::local::baz::add_to_linker::<_, Ctx>(
            &mut linker,
            |ctx| ctx,
        )?;

        let mut store = make_store();

        let instance = linker.instantiate_async(&mut store, &component).await?;
        instance.enable_concurrent_state_debug(&mut store, true);
        let round_trip =
            component_async_tests::round_trip::bindings::RoundTrip::new(&mut store, &instance)?;

        if call_style == 0 || !cfg!(miri) {
            // Now do it again using `Instance::run_concurrent`:
            instance
                .run_concurrent(&mut store, {
                    let inputs_and_outputs = inputs_and_outputs
                        .iter()
                        .map(|(a, b)| (String::from(*a), String::from(*b)))
                        .collect::<Vec<_>>();

                    async move |accessor| {
                        let mut futures = FuturesUnordered::new();
                        for (input, output) in &inputs_and_outputs {
                            let output = output.clone();
                            futures.push(
                                round_trip
                                    .local_local_baz()
                                    .call_foo(accessor, input.clone())
                                    .map(move |v| v.map(move |v| (v, output)))
                                    .boxed(),
                            );
                        }

                        while let Some((actual, expected)) = futures.try_next().await? {
                            assert_eq!(expected, actual);
                        }

                        Ok::<_, wasmtime::Error>(())
                    }
                })
                .await??;

            instance.assert_concurrent_state_empty(&mut store);
        }

        if call_style == 1 || !cfg!(miri) {
            // And again using `Instance::spawn`:
            struct Task {
                instance: Instance,
                inputs_and_outputs: Vec<(String, String)>,
                tx: oneshot::Sender<()>,
            }

            impl AccessorTask<Ctx, HasSelf<Ctx>, Result<()>> for Task {
                async fn run(self, accessor: &Accessor<Ctx>) -> Result<()> {
                    let round_trip = accessor.with(|mut store| {
                        component_async_tests::round_trip::bindings::RoundTrip::new(
                            &mut store,
                            &self.instance,
                        )
                    })?;

                    let mut futures = FuturesUnordered::new();
                    for (input, output) in &self.inputs_and_outputs {
                        let output = output.clone();
                        futures.push(
                            round_trip
                                .local_local_baz()
                                .call_foo(accessor, input.clone())
                                .map(move |v| v.map(move |v| (v, output)))
                                .boxed(),
                        );
                    }

                    while let Some((actual, expected)) = futures.try_next().await? {
                        assert_eq!(expected, actual);
                    }

                    _ = self.tx.send(());

                    Ok(())
                }
            }

            let (tx, rx) = oneshot::channel();
            instance.spawn(
                &mut store,
                Task {
                    instance,
                    inputs_and_outputs: inputs_and_outputs
                        .iter()
                        .map(|(a, b)| (String::from(*a), String::from(*b)))
                        .collect::<Vec<_>>(),
                    tx,
                },
            );

            instance
                .run_concurrent(&mut store, async |_| rx.await)
                .await??;

            instance.assert_concurrent_state_empty(&mut store);
        }

        if call_style == 2 || !cfg!(miri) {
            // And again using `TypedFunc::call_async`-based bindings:
            let round_trip =
                component_async_tests::round_trip::non_concurrent_export_bindings::RoundTrip::new(
                    &mut store, &instance,
                )?;

            for (input, expected) in inputs_and_outputs {
                assert_eq!(
                    *expected,
                    &round_trip
                        .local_local_baz()
                        .call_foo(&mut store, (*input).to_owned())
                        .await?
                );
            }

            instance.assert_concurrent_state_empty(&mut store);
        }
    }

    // Now do it again using the dynamic API (except for WASI, where we stick with the static API):
    {
        let mut linker = Linker::new(&engine);

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        linker
            .root()
            .instance("local:local/baz")?
            .func_new_concurrent("[async]foo", |_, params, results| {
                Box::pin(async move {
                    sleep(Duration::from_millis(10)).await;
                    let Some(Val::String(s)) = params.into_iter().next() else {
                        unreachable!()
                    };
                    results[0] = Val::String(format!("{s} - entered host - exited host"));
                    Ok(())
                })
            })?;

        let mut store = make_store();

        let instance = linker.instantiate_async(&mut store, &component).await?;
        let baz_instance = instance
            .get_export_index(&mut store, None, "local:local/baz")
            .ok_or_else(|| anyhow!("can't find `local:local/baz` in instance"))?;
        let foo_function = instance
            .get_export_index(&mut store, Some(&baz_instance), "[async]foo")
            .ok_or_else(|| anyhow!("can't find `foo` in instance"))?;
        let foo_function = instance
            .get_func(&mut store, foo_function)
            .ok_or_else(|| anyhow!("can't find `foo` in instance"))?;

        if call_style == 3 || !cfg!(miri) {
            instance
                .run_concurrent(&mut store, async |store| {
                    // Start three concurrent calls and then join them all:
                    let mut futures = FuturesUnordered::new();
                    for (input, output) in inputs_and_outputs {
                        let output = (*output).to_owned();
                        futures.push(
                            foo_function
                                .call_concurrent(store, vec![Val::String((*input).to_owned())])
                                .map(move |v| v.map(move |v| (v, output))),
                        );
                    }

                    while let Some((actual, expected)) = futures.try_next().await? {
                        let Some(Val::String(actual)) = actual.into_iter().next() else {
                            unreachable!()
                        };
                        assert_eq!(expected, actual);
                    }
                    anyhow::Ok(())
                })
                .await??;

            instance.assert_concurrent_state_empty(&mut store);
        }

        if call_style == 4 || !cfg!(miri) {
            // Now do it again using `Func::call_async`:
            for (input, expected) in inputs_and_outputs {
                let mut results = [Val::Bool(false)];
                foo_function
                    .call_async(
                        &mut store,
                        &[Val::String((*input).to_owned())],
                        &mut results,
                    )
                    .await?;
                let Val::String(actual) = &results[0] else {
                    unreachable!()
                };
                assert_eq!(*expected, actual);
                foo_function.post_return_async(&mut store).await?;
            }

            instance.assert_concurrent_state_empty(&mut store);
        }
    }

    Ok(())
}

pub async fn test_round_trip_uncomposed(component: &str) -> Result<()> {
    test_round_trip(
        &[component],
        &[
            (
                "hello, world!",
                "hello, world! - entered guest - entered host - exited host - exited guest",
            ),
            (
                "¡hola, mundo!",
                "¡hola, mundo! - entered guest - entered host - exited host - exited guest",
            ),
            (
                "hi y'all!",
                "hi y'all! - entered guest - entered host - exited host - exited guest",
            ),
        ],
    )
    .await
}

pub async fn test_round_trip_composed(a: &str, b: &str) -> Result<()> {
    test_round_trip(
        &[a, b],
        &[
            (
                "hello, world!",
                "hello, world! - entered guest - entered guest - entered host \
                 - exited host - exited guest - exited guest",
            ),
            (
                "¡hola, mundo!",
                "¡hola, mundo! - entered guest - entered guest - entered host \
                 - exited host - exited guest - exited guest",
            ),
            (
                "hi y'all!",
                "hi y'all! - entered guest - entered guest - entered host \
                 - exited host - exited guest - exited guest",
            ),
        ],
    )
    .await
}
