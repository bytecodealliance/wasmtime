use std::iter;
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
    stream::{FuturesUnordered, TryStreamExt},
};
use wasmtime::component::{Linker, ResourceTable, Val};
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::WasiCtxBuilder;

#[tokio::test]
pub async fn async_round_trip_many_stackless() -> Result<()> {
    test_round_trip_many_uncomposed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT,
    )
    .await
}

#[tokio::test]
pub async fn async_round_trip_many_stackful() -> Result<()> {
    test_round_trip_many_uncomposed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT,
    )
    .await
}

#[tokio::test]
pub async fn async_round_trip_many_synchronous() -> Result<()> {
    test_round_trip_many_uncomposed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT,
    )
    .await
}

#[tokio::test]
pub async fn async_round_trip_many_wait() -> Result<()> {
    test_round_trip_many_uncomposed(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT)
        .await
}

#[tokio::test]
async fn async_round_trip_many_stackless_plus_stackless() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_synchronous_plus_stackless() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_stackless_plus_synchronous() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_synchronous_plus_synchronous() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_wait_plus_wait() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_synchronous_plus_wait() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_wait_plus_synchronous() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_stackless_plus_wait() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_wait_plus_stackless() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_stackful_plus_stackful() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_stackful_plus_stackless() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_stackless_plus_stackful() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_synchronous_plus_stackful() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT,
    )
    .await
}

#[tokio::test]
async fn async_round_trip_many_stackful_plus_synchronous() -> Result<()> {
    test_round_trip_many_composed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT,
        test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT,
    )
    .await
}

async fn test_round_trip_many_uncomposed(component: &str) -> Result<()> {
    test_round_trip_many(
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

async fn test_round_trip_many(
    components: &[&str],
    inputs_and_outputs: &[(&str, &str)],
) -> Result<()> {
    use component_async_tests::round_trip_many::bindings::exports::local::local::many;

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

    let b = 42;
    let c = vec![42u8; 42];
    let d = (4242, 424242424242);
    let e = many::Stuff {
        a: vec![42i32; 42],
        b: true,
        c: 424242,
    };
    let f = Some(e.clone());
    let g = Err(());

    // On miri, we only use one call style per test since they take so long to
    // run.  On non-miri, we use every call style for each test.
    static CALL_STYLE_COUNTER: AtomicU32 = AtomicU32::new(0);
    let call_style = CALL_STYLE_COUNTER.fetch_add(1, Relaxed) % 4;

    // First, test the `wasmtime-wit-bindgen` static API:
    {
        let mut linker = Linker::new(&engine);

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        component_async_tests::round_trip_many::bindings::local::local::many::add_to_linker::<
            _,
            Ctx,
        >(&mut linker, |ctx| ctx)?;

        let mut store = make_store();

        let instance = linker.instantiate_async(&mut store, &component).await?;
        instance.enable_concurrent_state_debug(&mut store, true);
        let round_trip_many = component_async_tests::round_trip_many::bindings::RoundTripMany::new(
            &mut store, &instance,
        )?;

        if call_style == 0 {
            // Start concurrent calls and then join them all:
            let mut futures = FuturesUnordered::new();
            for (input, output) in inputs_and_outputs {
                let output = (*output).to_owned();
                futures.push(
                    round_trip_many
                        .local_local_many()
                        .call_foo(
                            &mut store,
                            (*input).to_owned(),
                            b,
                            c.clone(),
                            d,
                            e.clone(),
                            f.clone(),
                            g.clone(),
                        )
                        .map(move |v| v.map(move |v| (v, output))),
                );
            }

            while let Some((actual, expected)) =
                instance.run(&mut store, futures.try_next()).await??
            {
                assert_eq!(
                    (expected, b, c.clone(), d, e.clone(), f.clone(), g.clone()),
                    actual
                );
            }

            instance.assert_concurrent_state_empty(&mut store);
        }

        if call_style == 1 {
            // Now do it again using `TypedFunc::call_async`-based bindings:
            let e = component_async_tests::round_trip_many::non_concurrent_export_bindings::exports::local::local::many::Stuff {
                a: vec![42i32; 42],
                b: true,
                c: 424242,
            };
            let f = Some(e.clone());
            let g = Err(());

            let round_trip_many = component_async_tests::round_trip_many::non_concurrent_export_bindings::RoundTripMany::instantiate_async(
                &mut store, &component, &linker,
            )
                .await?;

            for (input, expected) in inputs_and_outputs {
                assert_eq!(
                    (
                        (*expected).to_owned(),
                        b,
                        c.clone(),
                        d,
                        e.clone(),
                        f.clone(),
                        g.clone()
                    ),
                    round_trip_many
                        .local_local_many()
                        .call_foo(
                            &mut store,
                            (*input).to_owned(),
                            b,
                            c.clone(),
                            d,
                            e.clone(),
                            f.clone(),
                            g.clone()
                        )
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
            .instance("local:local/many")?
            .func_new_concurrent("[async]foo", |_, params, results| {
                Box::pin(async move {
                    sleep(Duration::from_millis(10)).await;
                    let mut params = params.into_iter();
                    let Some(Val::String(s)) = params.next() else {
                        unreachable!()
                    };
                    results[0] = Val::Tuple(
                        iter::once(Val::String(format!("{s} - entered host - exited host")))
                            .chain(params.cloned())
                            .collect(),
                    );
                    Ok(())
                })
            })?;

        let mut store = make_store();

        let instance = linker.instantiate_async(&mut store, &component).await?;
        let baz_instance = instance
            .get_export_index(&mut store, None, "local:local/many")
            .ok_or_else(|| anyhow!("can't find `local:local/many` in instance"))?;
        let foo_function = instance
            .get_export_index(&mut store, Some(&baz_instance), "[async]foo")
            .ok_or_else(|| anyhow!("can't find `foo` in instance"))?;
        let foo_function = instance
            .get_func(&mut store, foo_function)
            .ok_or_else(|| anyhow!("can't find `foo` in instance"))?;

        let make = |input: &str| {
            let stuff = Val::Record(vec![
                (
                    "a".into(),
                    Val::List(e.a.iter().map(|v| Val::S32(*v)).collect()),
                ),
                ("b".into(), Val::Bool(e.b)),
                ("c".into(), Val::U64(e.c)),
            ]);
            vec![
                Val::String(input.to_owned()),
                Val::U32(b),
                Val::List(c.iter().map(|v| Val::U8(*v)).collect()),
                Val::Tuple(vec![Val::U64(d.0), Val::U64(d.1)]),
                stuff.clone(),
                Val::Option(Some(Box::new(stuff))),
                Val::Result(Err(None)),
            ]
        };

        if call_style == 2 {
            // Start three concurrent calls and then join them all:
            let mut futures = FuturesUnordered::new();
            for (input, output) in inputs_and_outputs {
                let output = (*output).to_owned();
                futures.push(
                    foo_function
                        .call_concurrent(&mut store, make(input))
                        .map(move |v| v.map(move |v| (v, output))),
                );
            }

            while let Some((actual, expected)) =
                instance.run(&mut store, futures.try_next()).await??
            {
                let Some(Val::Tuple(actual)) = actual.into_iter().next() else {
                    unreachable!()
                };
                assert_eq!(make(&expected), actual);
            }

            instance.assert_concurrent_state_empty(&mut store);
        }

        if call_style == 3 {
            // Now do it again using `Func::call_async`:
            for (input, expected) in inputs_and_outputs {
                let mut results = [Val::Bool(false)];
                foo_function
                    .call_async(&mut store, &make(input), &mut results)
                    .await?;
                let Val::Tuple(actual) = &results[0] else {
                    unreachable!()
                };
                assert_eq!(&make(expected), actual);
                foo_function.post_return_async(&mut store).await?;
            }

            instance.assert_concurrent_state_empty(&mut store);
        }
    }

    Ok(())
}

pub async fn test_round_trip_many_composed(a: &str, b: &str) -> Result<()> {
    test_round_trip_many(
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
