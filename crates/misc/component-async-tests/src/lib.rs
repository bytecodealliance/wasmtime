#![deny(warnings)]

#[cfg(test)]
mod test {
    use {
        anyhow::{anyhow, Result},
        futures::future,
        round_trip_many::local::local::many::Stuff,
        std::{
            future::Future,
            iter,
            ops::DerefMut,
            sync::{Arc, Mutex, Once},
            task::{Poll, Waker},
            time::Duration,
        },
        tokio::fs,
        transmit::exports::local::local::transmit::Control,
        wasi_http_draft::{
            wasi::http::types::{Body, ErrorCode, Method, Request, Response, Scheme},
            Fields, WasiHttpView,
        },
        wasm_compose::composer::ComponentComposer,
        wasmtime::{
            component::{
                self, Component, FutureReader, Instance, Linker, Promise, PromisesUnordered,
                Resource, ResourceTable, StreamReader, StreamWriter, Val,
            },
            AsContextMut, Config, Engine, Store, StoreContextMut,
        },
        wasmtime_wasi::{IoView, WasiCtx, WasiCtxBuilder, WasiView},
    };

    macro_rules! assert_test_exists {
        ($name:ident) => {
            #[expect(unused_imports, reason = "just here to ensure a name exists")]
            use self::$name as _;
        };
    }

    test_programs_artifacts::foreach_async!(assert_test_exists);

    fn init_logger() {
        static ONCE: Once = Once::new();
        ONCE.call_once(pretty_env_logger::init);
    }

    struct Ctx {
        wasi: WasiCtx,
        table: ResourceTable,
        wakers: Arc<Mutex<Option<Vec<Waker>>>>,
        continue_: bool,
    }

    impl IoView for Ctx {
        fn table(&mut self) -> &mut ResourceTable {
            &mut self.table
        }
    }

    impl WasiView for Ctx {
        fn ctx(&mut self) -> &mut WasiCtx {
            &mut self.wasi
        }
    }

    mod round_trip {
        wasmtime::component::bindgen!({
            trappable_imports: true,
            path: "wit",
            world: "round-trip",
            concurrent_imports: true,
            concurrent_exports: true,
            async: true,
        });
    }

    impl round_trip::local::local::baz::Host for Ctx {
        type Data = Ctx;

        #[allow(clippy::manual_async_fn)]
        fn foo(
            _: StoreContextMut<'_, Self>,
            s: String,
        ) -> impl Future<
            Output = impl FnOnce(StoreContextMut<'_, Self>) -> wasmtime::Result<String> + 'static,
        > + Send
               + 'static {
            async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                component::for_any(move |_: StoreContextMut<'_, Self>| {
                    Ok(format!("{s} - entered host - exited host"))
                })
            }
        }
    }

    mod round_trip_many {
        wasmtime::component::bindgen!({
            trappable_imports: true,
            path: "wit",
            world: "round-trip-many",
            concurrent_imports: true,
            concurrent_exports: true,
            async: true,
            additional_derives: [ Eq, PartialEq ],
        });
    }

    impl round_trip_many::local::local::many::Host for Ctx {
        type Data = Ctx;

        #[allow(clippy::manual_async_fn)]
        fn foo(
            _: StoreContextMut<'_, Self>,
            a: String,
            b: u32,
            c: Vec<u8>,
            d: (u64, u64),
            e: Stuff,
            f: Option<Stuff>,
            g: Result<Stuff, ()>,
        ) -> impl Future<
            Output = impl FnOnce(
                StoreContextMut<'_, Self>,
            ) -> wasmtime::Result<(
                String,
                u32,
                Vec<u8>,
                (u64, u64),
                Stuff,
                Option<Stuff>,
                Result<Stuff, ()>,
            )> + 'static,
        > + Send
               + 'static {
            async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                component::for_any(move |_: StoreContextMut<'_, Self>| {
                    Ok((
                        format!("{a} - entered host - exited host"),
                        b,
                        c,
                        d,
                        e,
                        f,
                        g,
                    ))
                })
            }
        }
    }

    mod round_trip_direct {
        wasmtime::component::bindgen!({
            trappable_imports: true,
            path: "wit",
            world: "round-trip-direct",
            concurrent_imports: true,
            concurrent_exports: true,
            async: true,
        });
    }

    impl round_trip_direct::RoundTripDirectImports for Ctx {
        type Data = Ctx;

        #[allow(clippy::manual_async_fn)]
        fn foo(
            _: StoreContextMut<'_, Self>,
            s: String,
        ) -> impl Future<
            Output = impl FnOnce(StoreContextMut<'_, Self>) -> wasmtime::Result<String> + 'static,
        > + Send
               + 'static {
            async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                component::for_any(move |_: StoreContextMut<'_, Self>| {
                    Ok(format!("{s} - entered host - exited host"))
                })
            }
        }
    }

    mod borrowing_host {
        wasmtime::component::bindgen!({
            path: "wit",
            world: "borrowing-host",
            trappable_imports: true,
            concurrent_imports: true,
            concurrent_exports: true,
            async: {
                only_imports: []
            },
            with: {
                "local:local/borrowing-types/x": super::MyX,
            }
        });
    }

    pub struct MyX;

    impl borrowing_host::local::local::borrowing_types::HostX for Ctx {
        fn new(&mut self) -> Result<Resource<MyX>> {
            Ok(IoView::table(self).push(MyX)?)
        }

        fn foo(&mut self, x: Resource<MyX>) -> Result<()> {
            _ = IoView::table(self).get(&x)?;
            Ok(())
        }

        fn drop(&mut self, x: Resource<MyX>) -> Result<()> {
            IoView::table(self).delete(x)?;
            Ok(())
        }
    }

    impl borrowing_host::local::local::borrowing_types::Host for Ctx {}

    /// Compose two components
    ///
    /// a is the "root" component, and b is composed into it
    async fn compose(a: &[u8], b: &[u8]) -> Result<Vec<u8>> {
        let dir = tempfile::tempdir()?;

        let a_file = dir.path().join("a.wasm");
        fs::write(&a_file, a).await?;

        let b_file = dir.path().join("b.wasm");
        fs::write(&b_file, b).await?;

        ComponentComposer::new(
            &a_file,
            &wasm_compose::config::Config {
                dir: dir.path().to_owned(),
                definitions: vec![b_file.to_owned()],
                ..Default::default()
            },
        )
        .compose()
    }

    async fn test_round_trip(component: &[u8], inputs_and_outputs: &[(&str, &str)]) -> Result<()> {
        init_logger();

        let mut config = Config::new();
        config.debug_info(true);
        config.cranelift_debug_verifier(true);
        config.wasm_component_model(true);
        config.wasm_component_model_async(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;

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

        let component = Component::new(&engine, component)?;

        // First, test the `wasmtime-wit-bindgen` static API:
        {
            let mut linker = Linker::new(&engine);

            wasmtime_wasi::add_to_linker_async(&mut linker)?;
            round_trip::RoundTrip::add_to_linker(&mut linker, |ctx| ctx)?;

            let mut store = make_store();

            let round_trip =
                round_trip::RoundTrip::instantiate_async(&mut store, &component, &linker).await?;

            // Start concurrent calls and then join them all:
            let mut promises = PromisesUnordered::new();
            for (input, output) in inputs_and_outputs {
                let output = (*output).to_owned();
                promises.push(
                    round_trip
                        .local_local_baz()
                        .call_foo(&mut store, (*input).to_owned())
                        .await?
                        .map(move |v| (v, output)),
                );
            }

            while let Some((actual, expected)) = promises.next(&mut store).await? {
                assert_eq!(expected, actual);
            }
        }

        // Now do it again using the dynamic API (except for WASI, where we stick with the static API):
        {
            let mut linker = Linker::new(&engine);

            wasmtime_wasi::add_to_linker_async(&mut linker)?;
            linker
                .root()
                .instance("local:local/baz")?
                .func_new_concurrent("foo", |_, params| async move {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    component::for_any(move |_: StoreContextMut<'_, Ctx>| {
                        let Some(Val::String(s)) = params.into_iter().next() else {
                            unreachable!()
                        };
                        Ok(vec![Val::String(format!(
                            "{s} - entered host - exited host"
                        ))])
                    })
                })?;

            let mut store = make_store();

            let instance = linker.instantiate_async(&mut store, &component).await?;
            let baz_instance = instance
                .get_export(&mut store, None, "local:local/baz")
                .ok_or_else(|| anyhow!("can't find `local:local/baz` in instance"))?;
            let foo_function = instance
                .get_export(&mut store, Some(&baz_instance), "foo")
                .ok_or_else(|| anyhow!("can't find `foo` in instance"))?;
            let foo_function = instance
                .get_func(&mut store, foo_function)
                .ok_or_else(|| anyhow!("can't find `foo` in instance"))?;

            // Start three concurrent calls and then join them all:
            let mut promises = PromisesUnordered::new();
            for (input, output) in inputs_and_outputs {
                let output = (*output).to_owned();
                promises.push(
                    foo_function
                        .call_concurrent(&mut store, vec![Val::String((*input).to_owned())])
                        .await?
                        .map(move |v| (v, output)),
                );
            }

            while let Some((actual, expected)) = promises.next(&mut store).await? {
                let Some(Val::String(actual)) = actual.into_iter().next() else {
                    unreachable!()
                };
                assert_eq!(expected, actual);
            }
        }

        Ok(())
    }

    async fn test_round_trip_uncomposed(component: &[u8]) -> Result<()> {
        test_round_trip(
            component,
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

    async fn test_round_trip_composed(a: &[u8], b: &[u8]) -> Result<()> {
        test_round_trip(
            &compose(a, b).await?,
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

    #[tokio::test]
    async fn async_round_trip_stackless() -> Result<()> {
        test_round_trip_uncomposed(
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT).await?,
        )
        .await
    }

    #[tokio::test]
    async fn async_round_trip_stackful() -> Result<()> {
        test_round_trip_uncomposed(
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT).await?,
        )
        .await
    }

    #[tokio::test]
    async fn async_round_trip_synchronous() -> Result<()> {
        test_round_trip_uncomposed(
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT).await?,
        )
        .await
    }

    #[tokio::test]
    async fn async_round_trip_wait() -> Result<()> {
        test_round_trip_uncomposed(
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT).await?,
        )
        .await
    }

    #[tokio::test]
    async fn async_round_trip_stackless_plus_stackless() -> Result<()> {
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT).await?;
        test_round_trip_composed(stackless, stackless).await
    }

    #[tokio::test]
    async fn async_round_trip_synchronous_plus_stackless() -> Result<()> {
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT).await?;
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT).await?;
        test_round_trip_composed(synchronous, stackless).await
    }

    #[tokio::test]
    async fn async_round_trip_stackless_plus_synchronous() -> Result<()> {
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT).await?;
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT).await?;
        test_round_trip_composed(stackless, synchronous).await
    }

    #[tokio::test]
    async fn async_round_trip_synchronous_plus_synchronous() -> Result<()> {
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT).await?;
        test_round_trip_composed(synchronous, synchronous).await
    }

    #[tokio::test]
    async fn async_round_trip_wait_plus_wait() -> Result<()> {
        let wait = &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT).await?;
        test_round_trip_composed(wait, wait).await
    }

    #[tokio::test]
    async fn async_round_trip_synchronous_plus_wait() -> Result<()> {
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT).await?;
        let wait = &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT).await?;
        test_round_trip_composed(synchronous, wait).await
    }

    #[tokio::test]
    async fn async_round_trip_wait_plus_synchronous() -> Result<()> {
        let wait = &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT).await?;
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT).await?;
        test_round_trip_composed(wait, synchronous).await
    }

    #[tokio::test]
    async fn async_round_trip_stackless_plus_wait() -> Result<()> {
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT).await?;
        let wait = &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT).await?;
        test_round_trip_composed(stackless, wait).await
    }

    #[tokio::test]
    async fn async_round_trip_wait_plus_stackless() -> Result<()> {
        let wait = &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_WAIT_COMPONENT).await?;
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT).await?;
        test_round_trip_composed(wait, stackless).await
    }

    #[tokio::test]
    async fn async_round_trip_stackful_plus_stackful() -> Result<()> {
        let stackful =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT).await?;
        test_round_trip_composed(stackful, stackful).await
    }

    #[tokio::test]
    async fn async_round_trip_stackful_plus_stackless() -> Result<()> {
        let stackful =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT).await?;
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT).await?;
        test_round_trip_composed(stackful, stackless).await
    }

    #[tokio::test]
    async fn async_round_trip_stackless_plus_stackful() -> Result<()> {
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKLESS_COMPONENT).await?;
        let stackful =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT).await?;
        test_round_trip_composed(stackless, stackful).await
    }

    #[tokio::test]
    async fn async_round_trip_synchronous_plus_stackful() -> Result<()> {
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT).await?;
        let stackful =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT).await?;
        test_round_trip_composed(synchronous, stackful).await
    }

    #[tokio::test]
    async fn async_round_trip_stackful_plus_synchronous() -> Result<()> {
        let stackful =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_STACKFUL_COMPONENT).await?;
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_SYNCHRONOUS_COMPONENT).await?;
        test_round_trip_composed(stackful, synchronous).await
    }

    async fn test_round_trip_many(
        component: &[u8],
        inputs_and_outputs: &[(&str, &str)],
    ) -> Result<()> {
        use round_trip_many::exports::local::local::many;

        init_logger();

        let mut config = Config::new();
        config.debug_info(true);
        config.cranelift_debug_verifier(true);
        config.wasm_component_model(true);
        config.wasm_component_model_async(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;

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

        let component = Component::new(&engine, component)?;

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

        // First, test the `wasmtime-wit-bindgen` static API:
        {
            let mut linker = Linker::new(&engine);

            wasmtime_wasi::add_to_linker_async(&mut linker)?;
            round_trip_many::RoundTripMany::add_to_linker(&mut linker, |ctx| ctx)?;

            let mut store = make_store();

            let round_trip_many =
                round_trip_many::RoundTripMany::instantiate_async(&mut store, &component, &linker)
                    .await?;

            // Start concurrent calls and then join them all:
            let mut promises = PromisesUnordered::new();
            for (input, output) in inputs_and_outputs {
                let output = (*output).to_owned();
                promises.push(
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
                        .await?
                        .map(move |v| (v, output)),
                );
            }

            while let Some((actual, expected)) = promises.next(&mut store).await? {
                assert_eq!(
                    (expected, b, c.clone(), d, e.clone(), f.clone(), g.clone()),
                    actual
                );
            }
        }

        // Now do it again using the dynamic API (except for WASI, where we stick with the static API):
        {
            let mut linker = Linker::new(&engine);

            wasmtime_wasi::add_to_linker_async(&mut linker)?;
            linker
                .root()
                .instance("local:local/many")?
                .func_new_concurrent("foo", |_, params| async move {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    component::for_any(move |_: StoreContextMut<'_, Ctx>| {
                        let mut params = params.into_iter();
                        let Some(Val::String(s)) = params.next() else {
                            unreachable!()
                        };
                        Ok(vec![Val::Tuple(
                            iter::once(Val::String(format!("{s} - entered host - exited host")))
                                .chain(params)
                                .collect(),
                        )])
                    })
                })?;

            let mut store = make_store();

            let instance = linker.instantiate_async(&mut store, &component).await?;
            let baz_instance = instance
                .get_export(&mut store, None, "local:local/many")
                .ok_or_else(|| anyhow!("can't find `local:local/many` in instance"))?;
            let foo_function = instance
                .get_export(&mut store, Some(&baz_instance), "foo")
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

            // Start three concurrent calls and then join them all:
            let mut promises = PromisesUnordered::new();
            for (input, output) in inputs_and_outputs {
                let output = (*output).to_owned();
                promises.push(
                    foo_function
                        .call_concurrent(&mut store, make(input))
                        .await?
                        .map(move |v| (v, output)),
                );
            }

            while let Some((actual, expected)) = promises.next(&mut store).await? {
                let Some(Val::Tuple(actual)) = actual.into_iter().next() else {
                    unreachable!()
                };
                assert_eq!(make(&expected), actual);
            }
        }

        Ok(())
    }

    async fn test_round_trip_many_uncomposed(component: &[u8]) -> Result<()> {
        test_round_trip_many(
            component,
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

    async fn test_round_trip_many_composed(a: &[u8], b: &[u8]) -> Result<()> {
        test_round_trip_many(
            &compose(a, b).await?,
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

    #[tokio::test]
    async fn async_round_trip_many_stackless() -> Result<()> {
        test_round_trip_many_uncomposed(
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT).await?,
        )
        .await
    }

    #[tokio::test]
    async fn async_round_trip_many_stackful() -> Result<()> {
        test_round_trip_many_uncomposed(
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT).await?,
        )
        .await
    }

    #[tokio::test]
    async fn async_round_trip_many_synchronous() -> Result<()> {
        test_round_trip_many_uncomposed(
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT).await?,
        )
        .await
    }

    #[tokio::test]
    async fn async_round_trip_many_wait() -> Result<()> {
        test_round_trip_many_uncomposed(
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT).await?,
        )
        .await
    }

    #[tokio::test]
    async fn async_round_trip_many_stackless_plus_stackless() -> Result<()> {
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT).await?;
        test_round_trip_many_composed(stackless, stackless).await
    }

    #[tokio::test]
    async fn async_round_trip_many_synchronous_plus_stackless() -> Result<()> {
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT).await?;
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT).await?;
        test_round_trip_many_composed(synchronous, stackless).await
    }

    #[tokio::test]
    async fn async_round_trip_many_stackless_plus_synchronous() -> Result<()> {
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT).await?;
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT).await?;
        test_round_trip_many_composed(stackless, synchronous).await
    }

    #[tokio::test]
    async fn async_round_trip_many_synchronous_plus_synchronous() -> Result<()> {
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT).await?;
        test_round_trip_many_composed(synchronous, synchronous).await
    }

    #[tokio::test]
    async fn async_round_trip_many_wait_plus_wait() -> Result<()> {
        let wait = &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT).await?;
        test_round_trip_many_composed(wait, wait).await
    }

    #[tokio::test]
    async fn async_round_trip_many_synchronous_plus_wait() -> Result<()> {
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT).await?;
        let wait = &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT).await?;
        test_round_trip_many_composed(synchronous, wait).await
    }

    #[tokio::test]
    async fn async_round_trip_many_wait_plus_synchronous() -> Result<()> {
        let wait = &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT).await?;
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT).await?;
        test_round_trip_many_composed(wait, synchronous).await
    }

    #[tokio::test]
    async fn async_round_trip_many_stackless_plus_wait() -> Result<()> {
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT).await?;
        let wait = &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT).await?;
        test_round_trip_many_composed(stackless, wait).await
    }

    #[tokio::test]
    async fn async_round_trip_many_wait_plus_stackless() -> Result<()> {
        let wait = &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_WAIT_COMPONENT).await?;
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT).await?;
        test_round_trip_many_composed(wait, stackless).await
    }

    #[tokio::test]
    async fn async_round_trip_many_stackful_plus_stackful() -> Result<()> {
        let stackful =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT).await?;
        test_round_trip_many_composed(stackful, stackful).await
    }

    #[tokio::test]
    async fn async_round_trip_many_stackful_plus_stackless() -> Result<()> {
        let stackful =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT).await?;
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT).await?;
        test_round_trip_many_composed(stackful, stackless).await
    }

    #[tokio::test]
    async fn async_round_trip_many_stackless_plus_stackful() -> Result<()> {
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKLESS_COMPONENT).await?;
        let stackful =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT).await?;
        test_round_trip_many_composed(stackless, stackful).await
    }

    #[tokio::test]
    async fn async_round_trip_many_synchronous_plus_stackful() -> Result<()> {
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT).await?;
        let stackful =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT).await?;
        test_round_trip_many_composed(synchronous, stackful).await
    }

    #[tokio::test]
    async fn async_round_trip_many_stackful_plus_synchronous() -> Result<()> {
        let stackful =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_STACKFUL_COMPONENT).await?;
        let synchronous =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_MANY_SYNCHRONOUS_COMPONENT).await?;
        test_round_trip_many_composed(stackful, synchronous).await
    }

    async fn test_round_trip_direct(
        component: &[u8],
        input: &str,
        expected_output: &str,
    ) -> Result<()> {
        init_logger();

        let mut config = Config::new();
        config.debug_info(true);
        config.cranelift_debug_verifier(true);
        config.wasm_component_model(true);
        config.wasm_component_model_async(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;

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

        let component = Component::new(&engine, component)?;

        // First, test the `wasmtime-wit-bindgen` static API:
        {
            let mut linker = Linker::new(&engine);

            wasmtime_wasi::add_to_linker_async(&mut linker)?;
            round_trip_direct::RoundTripDirect::add_to_linker(&mut linker, |ctx| ctx)?;

            let mut store = make_store();

            let round_trip = round_trip_direct::RoundTripDirect::instantiate_async(
                &mut store, &component, &linker,
            )
            .await?;

            // Start three concurrent calls and then join them all:
            let mut promises = PromisesUnordered::new();
            for _ in 0..3 {
                promises.push(round_trip.call_foo(&mut store, input.to_owned()).await?);
            }

            while let Some(value) = promises.next(&mut store).await? {
                assert_eq!(expected_output, &value);
            }
        }

        // Now do it again using the dynamic API (except for WASI, where we stick with the static API):
        {
            let mut linker = Linker::new(&engine);

            wasmtime_wasi::add_to_linker_async(&mut linker)?;
            linker
                .root()
                .func_new_concurrent("foo", |_, params| async move {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    component::for_any(move |_: StoreContextMut<'_, Ctx>| {
                        let Some(Val::String(s)) = params.into_iter().next() else {
                            unreachable!()
                        };
                        Ok(vec![Val::String(format!(
                            "{s} - entered host - exited host"
                        ))])
                    })
                })?;

            let mut store = make_store();

            let instance = linker.instantiate_async(&mut store, &component).await?;
            let foo_function = instance
                .get_export(&mut store, None, "foo")
                .ok_or_else(|| anyhow!("can't find `foo` in instance"))?;
            let foo_function = instance
                .get_func(&mut store, foo_function)
                .ok_or_else(|| anyhow!("can't find `foo` in instance"))?;

            // Start three concurrent calls and then join them all:
            let mut promises = PromisesUnordered::new();
            for _ in 0..3 {
                promises.push(
                    foo_function
                        .call_concurrent(&mut store, vec![Val::String(input.to_owned())])
                        .await?,
                );
            }

            while let Some(value) = promises.next(&mut store).await? {
                let Some(Val::String(value)) = value.into_iter().next() else {
                    unreachable!()
                };
                assert_eq!(expected_output, &value);
            }
        }

        Ok(())
    }

    async fn test_round_trip_direct_uncomposed(component: &[u8]) -> Result<()> {
        test_round_trip_direct(
            component,
            "hello, world!",
            "hello, world! - entered guest - entered host - exited host - exited guest",
        )
        .await
    }

    #[tokio::test]
    async fn async_round_trip_direct_stackless() -> Result<()> {
        let stackless =
            &fs::read(test_programs_artifacts::ASYNC_ROUND_TRIP_DIRECT_STACKLESS_COMPONENT).await?;
        test_round_trip_direct_uncomposed(stackless).await
    }

    mod yield_host {
        wasmtime::component::bindgen!({
            path: "wit",
            world: "yield-host",
            concurrent_imports: true,
            concurrent_exports: true,
            async: {
                only_imports: [
                    "local:local/ready#when-ready",
                ]
            },
        });
    }

    impl yield_host::local::local::continue_::Host for Ctx {
        fn set_continue(&mut self, v: bool) {
            self.continue_ = v;
        }

        fn get_continue(&mut self) -> bool {
            self.continue_
        }
    }

    impl yield_host::local::local::ready::Host for Ctx {
        type Data = Ctx;

        fn set_ready(&mut self, ready: bool) {
            let mut wakers = self.wakers.lock().unwrap();
            if ready {
                if let Some(wakers) = wakers.take() {
                    for waker in wakers {
                        waker.wake();
                    }
                }
            } else if wakers.is_none() {
                *wakers = Some(Vec::new());
            }
        }

        fn when_ready(
            store: StoreContextMut<Self::Data>,
        ) -> impl Future<Output = impl FnOnce(StoreContextMut<Self::Data>) + 'static>
               + Send
               + Sync
               + 'static {
            let wakers = store.data().wakers.clone();
            future::poll_fn(move |cx| {
                let mut wakers = wakers.lock().unwrap();
                if let Some(wakers) = wakers.deref_mut() {
                    wakers.push(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(component::for_any(|_| ()))
                }
            })
        }
    }

    async fn test_run(component: &[u8]) -> Result<()> {
        init_logger();

        let mut config = Config::new();
        config.debug_info(true);
        config.cranelift_debug_verifier(true);
        config.wasm_component_model(true);
        config.wasm_component_model_async(true);
        config.async_support(true);
        config.epoch_interruption(true);

        let engine = Engine::new(&config)?;

        let component = Component::new(&engine, component)?;

        let mut linker = Linker::new(&engine);

        wasmtime_wasi::add_to_linker_async(&mut linker)?;
        yield_host::YieldHost::add_to_linker(&mut linker, |ctx| ctx)?;

        let mut store = Store::new(
            &engine,
            Ctx {
                wasi: WasiCtxBuilder::new().inherit_stdio().build(),
                table: ResourceTable::default(),
                continue_: false,
                wakers: Arc::new(Mutex::new(None)),
            },
        );
        store.set_epoch_deadline(1);

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(10));
            engine.increment_epoch();
        });

        let yield_host =
            yield_host::YieldHost::instantiate_async(&mut store, &component, &linker).await?;

        // Start three concurrent calls and then join them all:
        let mut promises = PromisesUnordered::new();
        for _ in 0..3 {
            promises.push(yield_host.local_local_run().call_run(&mut store).await?);
        }

        while let Some(()) = promises.next(&mut store).await? {
            // continue
        }

        Ok(())
    }

    // No-op function; we only test this by composing it in `async_yield_caller`
    #[allow(
        dead_code,
        reason = "here only to make the `assert_test_exists` macro happy"
    )]
    fn async_yield_callee() {}

    #[tokio::test]
    async fn async_yield_caller() -> Result<()> {
        let caller = &fs::read(test_programs_artifacts::ASYNC_YIELD_CALLER_COMPONENT).await?;
        let callee = &fs::read(test_programs_artifacts::ASYNC_YIELD_CALLEE_COMPONENT).await?;
        test_run(&compose(caller, callee).await?).await
    }

    #[tokio::test]
    async fn async_poll() -> Result<()> {
        test_run(&fs::read(test_programs_artifacts::ASYNC_POLL_COMPONENT).await?).await
    }

    // No-op function; we only test this by composing it in `async_backpressure_caller`
    #[allow(
        dead_code,
        reason = "here only to make the `assert_test_exists` macro happy"
    )]
    fn async_backpressure_callee() {}

    #[tokio::test]
    async fn async_backpressure_caller() -> Result<()> {
        let caller =
            &fs::read(test_programs_artifacts::ASYNC_BACKPRESSURE_CALLER_COMPONENT).await?;
        let callee =
            &fs::read(test_programs_artifacts::ASYNC_BACKPRESSURE_CALLEE_COMPONENT).await?;
        test_run(&compose(caller, callee).await?).await
    }

    #[tokio::test]
    async fn async_transmit_caller() -> Result<()> {
        let caller = &fs::read(test_programs_artifacts::ASYNC_TRANSMIT_CALLER_COMPONENT).await?;
        let callee = &fs::read(test_programs_artifacts::ASYNC_TRANSMIT_CALLEE_COMPONENT).await?;
        test_run(&compose(caller, callee).await?).await
    }

    // No-op function; we only test this by composing it in `async_post_return_caller`
    #[allow(
        dead_code,
        reason = "here only to make the `assert_test_exists` macro happy"
    )]
    fn async_post_return_callee() {}

    #[tokio::test]
    async fn async_post_return_caller() -> Result<()> {
        let caller = &fs::read(test_programs_artifacts::ASYNC_POST_RETURN_CALLER_COMPONENT).await?;
        let callee = &fs::read(test_programs_artifacts::ASYNC_POST_RETURN_CALLEE_COMPONENT).await?;
        test_run(&compose(caller, callee).await?).await
    }

    // No-op function; we only test this by composing it in `async_unit_stream_caller`
    #[allow(
        dead_code,
        reason = "here only to make the `assert_test_exists` macro happy"
    )]
    fn async_unit_stream_callee() {}

    #[tokio::test]
    async fn async_unit_stream_caller() -> Result<()> {
        let caller = &fs::read(test_programs_artifacts::ASYNC_UNIT_STREAM_CALLER_COMPONENT).await?;
        let callee = &fs::read(test_programs_artifacts::ASYNC_UNIT_STREAM_CALLEE_COMPONENT).await?;
        test_run(&compose(caller, callee).await?).await
    }

    async fn test_run_bool(component: &[u8], v: bool) -> Result<()> {
        init_logger();

        let mut config = Config::new();
        config.debug_info(true);
        config.cranelift_debug_verifier(true);
        config.wasm_component_model(true);
        config.wasm_component_model_async(true);
        config.async_support(true);
        config.epoch_interruption(true);

        let engine = Engine::new(&config)?;

        let component = Component::new(&engine, component)?;

        let mut linker = Linker::new(&engine);

        wasmtime_wasi::add_to_linker_async(&mut linker)?;
        borrowing_host::BorrowingHost::add_to_linker(&mut linker, |ctx| ctx)?;

        let mut store = Store::new(
            &engine,
            Ctx {
                wasi: WasiCtxBuilder::new().inherit_stdio().build(),
                table: ResourceTable::default(),
                continue_: false,
                wakers: Arc::new(Mutex::new(None)),
            },
        );
        store.set_epoch_deadline(1);

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(10));
            engine.increment_epoch();
        });

        let borrowing_host =
            borrowing_host::BorrowingHost::instantiate_async(&mut store, &component, &linker)
                .await?;

        // Start three concurrent calls and then join them all:
        let mut promises = PromisesUnordered::new();
        for _ in 0..3 {
            promises.push(
                borrowing_host
                    .local_local_run_bool()
                    .call_run(&mut store, v)
                    .await?,
            );
        }

        while let Some(()) = promises.next(&mut store).await? {
            // continue
        }

        Ok(())
    }

    #[tokio::test]
    async fn async_borrowing_caller() -> Result<()> {
        let caller = &fs::read(test_programs_artifacts::ASYNC_BORROWING_CALLER_COMPONENT).await?;
        let callee = &fs::read(test_programs_artifacts::ASYNC_BORROWING_CALLEE_COMPONENT).await?;
        test_run_bool(&compose(caller, callee).await?, false).await
    }

    #[tokio::test]
    async fn async_borrowing_caller_misbehave() -> Result<()> {
        let caller = &fs::read(test_programs_artifacts::ASYNC_BORROWING_CALLER_COMPONENT).await?;
        let callee = &fs::read(test_programs_artifacts::ASYNC_BORROWING_CALLEE_COMPONENT).await?;
        let error = format!(
            "{:?}",
            test_run_bool(&compose(caller, callee).await?, true)
                .await
                .unwrap_err()
        );
        assert!(error.contains("unknown handle index"), "{error}");
        Ok(())
    }

    #[tokio::test]
    async fn async_borrowing_callee() -> Result<()> {
        let callee = &fs::read(test_programs_artifacts::ASYNC_BORROWING_CALLEE_COMPONENT).await?;
        test_run_bool(callee, false).await
    }

    #[tokio::test]
    async fn async_borrowing_callee_misbehave() -> Result<()> {
        let callee = &fs::read(test_programs_artifacts::ASYNC_BORROWING_CALLEE_COMPONENT).await?;
        let error = format!("{:?}", test_run_bool(callee, true).await.unwrap_err());
        assert!(error.contains("unknown handle index"), "{error}");
        Ok(())
    }

    mod transmit {
        wasmtime::component::bindgen!({
            path: "wit",
            world: "transmit-callee",
            concurrent_exports: true,
            async: true,
        });
    }

    trait TransmitTest {
        type Instance;
        type Params;
        type Result;

        async fn instantiate(
            store: impl AsContextMut<Data = Ctx>,
            component: &Component,
            linker: &Linker<Ctx>,
        ) -> Result<Self::Instance>;

        async fn call(
            store: impl AsContextMut<Data = Ctx>,
            instance: &Self::Instance,
            params: Self::Params,
        ) -> Result<Promise<Self::Result>>;

        fn into_params(
            control: StreamReader<Control>,
            caller_stream: StreamReader<String>,
            caller_future1: FutureReader<String>,
            caller_future2: FutureReader<String>,
        ) -> Self::Params;

        fn from_result(
            store: impl AsContextMut<Data = Ctx>,
            result: Self::Result,
        ) -> Result<(
            StreamReader<String>,
            FutureReader<String>,
            FutureReader<String>,
        )>;
    }

    struct StaticTransmitTest;

    impl TransmitTest for StaticTransmitTest {
        type Instance = transmit::TransmitCallee;
        type Params = (
            StreamReader<Control>,
            StreamReader<String>,
            FutureReader<String>,
            FutureReader<String>,
        );
        type Result = (
            StreamReader<String>,
            FutureReader<String>,
            FutureReader<String>,
        );

        async fn instantiate(
            store: impl AsContextMut<Data = Ctx>,
            component: &Component,
            linker: &Linker<Ctx>,
        ) -> Result<Self::Instance> {
            transmit::TransmitCallee::instantiate_async(store, component, linker).await
        }

        async fn call(
            store: impl AsContextMut<Data = Ctx>,
            instance: &Self::Instance,
            params: Self::Params,
        ) -> Result<Promise<Self::Result>> {
            instance
                .local_local_transmit()
                .call_exchange(store, params.0, params.1, params.2, params.3)
                .await
        }

        fn into_params(
            control: StreamReader<Control>,
            caller_stream: StreamReader<String>,
            caller_future1: FutureReader<String>,
            caller_future2: FutureReader<String>,
        ) -> Self::Params {
            (control, caller_stream, caller_future1, caller_future2)
        }

        fn from_result(
            _: impl AsContextMut<Data = Ctx>,
            result: Self::Result,
        ) -> Result<(
            StreamReader<String>,
            FutureReader<String>,
            FutureReader<String>,
        )> {
            Ok(result)
        }
    }

    struct DynamicTransmitTest;

    impl TransmitTest for DynamicTransmitTest {
        type Instance = Instance;
        type Params = Vec<Val>;
        type Result = Val;

        async fn instantiate(
            store: impl AsContextMut<Data = Ctx>,
            component: &Component,
            linker: &Linker<Ctx>,
        ) -> Result<Self::Instance> {
            linker.instantiate_async(store, component).await
        }

        async fn call(
            mut store: impl AsContextMut<Data = Ctx>,
            instance: &Self::Instance,
            params: Self::Params,
        ) -> Result<Promise<Self::Result>> {
            let transmit_instance = instance
                .get_export(store.as_context_mut(), None, "local:local/transmit")
                .ok_or_else(|| anyhow!("can't find `local:local/transmit` in instance"))?;
            let exchange_function = instance
                .get_export(store.as_context_mut(), Some(&transmit_instance), "exchange")
                .ok_or_else(|| anyhow!("can't find `exchange` in instance"))?;
            let exchange_function = instance
                .get_func(store.as_context_mut(), exchange_function)
                .ok_or_else(|| anyhow!("can't find `exchange` in instance"))?;

            Ok(exchange_function
                .call_concurrent(store, params)
                .await?
                .map(|results| results.into_iter().next().unwrap()))
        }

        fn into_params(
            control: StreamReader<Control>,
            caller_stream: StreamReader<String>,
            caller_future1: FutureReader<String>,
            caller_future2: FutureReader<String>,
        ) -> Self::Params {
            vec![
                control.into_val(),
                caller_stream.into_val(),
                caller_future1.into_val(),
                caller_future2.into_val(),
            ]
        }

        fn from_result(
            mut store: impl AsContextMut<Data = Ctx>,
            result: Self::Result,
        ) -> Result<(
            StreamReader<String>,
            FutureReader<String>,
            FutureReader<String>,
        )> {
            let Val::Tuple(fields) = result else {
                unreachable!()
            };
            let stream = StreamReader::from_val(store.as_context_mut(), &fields[0])?;
            let future1 = FutureReader::from_val(store.as_context_mut(), &fields[1])?;
            let future2 = FutureReader::from_val(store.as_context_mut(), &fields[2])?;
            Ok((stream, future1, future2))
        }
    }

    async fn test_transmit(component: &[u8]) -> Result<()> {
        init_logger();

        test_transmit_with::<StaticTransmitTest>(component).await?;
        test_transmit_with::<DynamicTransmitTest>(component).await
    }

    async fn test_transmit_with<Test: TransmitTest + 'static>(component: &[u8]) -> Result<()> {
        let mut config = Config::new();
        config.debug_info(true);
        config.cranelift_debug_verifier(true);
        config.wasm_component_model(true);
        config.wasm_component_model_async(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;

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

        let component = Component::new(&engine, component)?;

        let mut linker = Linker::new(&engine);

        wasmtime_wasi::add_to_linker_async(&mut linker)?;

        let mut store = make_store();

        let instance = Test::instantiate(&mut store, &component, &linker).await?;

        enum Event<Test: TransmitTest> {
            Result(Test::Result),
            ControlWriteA(StreamWriter<Control>),
            ControlWriteB(StreamWriter<Control>),
            ControlWriteC(StreamWriter<Control>),
            ControlWriteD(StreamWriter<Control>),
            WriteA(StreamWriter<String>),
            WriteB,
            ReadC(Option<(StreamReader<String>, Vec<String>)>),
            ReadD(Option<String>),
            ReadNone(Option<(StreamReader<String>, Vec<String>)>),
        }

        let (control_tx, control_rx) = component::stream(&mut store)?;
        let (caller_stream_tx, caller_stream_rx) = component::stream(&mut store)?;
        let (caller_future1_tx, caller_future1_rx) = component::future(&mut store)?;
        let (_caller_future2_tx, caller_future2_rx) = component::future(&mut store)?;

        let mut promises = PromisesUnordered::<Event<Test>>::new();
        let mut caller_future1_tx = Some(caller_future1_tx);
        let mut callee_stream_rx = None;
        let mut callee_future1_rx = None;
        let mut complete = false;

        promises.push(
            control_tx
                .write(&mut store, vec![Control::ReadStream("a".into())])?
                .map(Event::ControlWriteA),
        );

        promises.push(
            caller_stream_tx
                .write(&mut store, vec!["a".into()])?
                .map(Event::WriteA),
        );

        promises.push(
            Test::call(
                &mut store,
                &instance,
                Test::into_params(
                    control_rx,
                    caller_stream_rx,
                    caller_future1_rx,
                    caller_future2_rx,
                ),
            )
            .await?
            .map(Event::Result),
        );

        while let Some(event) = promises.next(&mut store).await? {
            match event {
                Event::Result(result) => {
                    let results = Test::from_result(&mut store, result)?;
                    callee_stream_rx = Some(results.0);
                    callee_future1_rx = Some(results.1);
                    results.2.close(&mut store)?;
                }
                Event::ControlWriteA(tx) => {
                    promises.push(
                        tx.write(&mut store, vec![Control::ReadFuture("b".into())])?
                            .map(Event::ControlWriteB),
                    );
                }
                Event::WriteA(tx) => {
                    tx.close(&mut store)?;
                    promises.push(
                        caller_future1_tx
                            .take()
                            .unwrap()
                            .write(&mut store, "b".into())?
                            .map(|()| Event::WriteB),
                    );
                }
                Event::ControlWriteB(tx) => {
                    promises.push(
                        tx.write(&mut store, vec![Control::WriteStream("c".into())])?
                            .map(Event::ControlWriteC),
                    );
                }
                Event::WriteB => {
                    promises.push(
                        callee_stream_rx
                            .take()
                            .unwrap()
                            .read(&mut store)?
                            .map(Event::ReadC),
                    );
                }
                Event::ControlWriteC(tx) => {
                    promises.push(
                        tx.write(&mut store, vec![Control::WriteFuture("d".into())])?
                            .map(Event::ControlWriteD),
                    );
                }
                Event::ReadC(None) => unreachable!(),
                Event::ReadC(Some((rx, values))) => {
                    assert_eq!("c", &values[0]);
                    promises.push(
                        callee_future1_rx
                            .take()
                            .unwrap()
                            .read(&mut store)?
                            .map(Event::ReadD),
                    );
                    callee_stream_rx = Some(rx);
                }
                Event::ControlWriteD(tx) => {
                    tx.close(&mut store)?;
                }
                Event::ReadD(None) => unreachable!(),
                Event::ReadD(Some(value)) => {
                    assert_eq!("d", &value);
                    promises.push(
                        callee_stream_rx
                            .take()
                            .unwrap()
                            .read(&mut store)?
                            .map(Event::ReadNone),
                    );
                }
                Event::ReadNone(Some(_)) => unreachable!(),
                Event::ReadNone(None) => {
                    complete = true;
                }
            }
        }

        assert!(complete);

        Ok(())
    }

    #[tokio::test]
    async fn async_transmit_callee() -> Result<()> {
        test_transmit(&fs::read(test_programs_artifacts::ASYNC_TRANSMIT_CALLEE_COMPONENT).await?)
            .await
    }

    mod proxy {
        wasmtime::component::bindgen!({
            path: "wit",
            world: "wasi:http/proxy",
            concurrent_imports: true,
            concurrent_exports: true,
            async: {
                only_imports: [
                    "wasi:http/types@0.3.0-draft#[static]body.finish",
                    "wasi:http/handler@0.3.0-draft#handle",
                ]
            },
            with: {
                "wasi:http/types": wasi_http_draft::wasi::http::types,
            }
        });
    }

    impl WasiHttpView for Ctx {
        type Data = Ctx;

        fn table(&mut self) -> &mut ResourceTable {
            &mut self.table
        }

        #[allow(clippy::manual_async_fn)]
        fn send_request(
            _store: StoreContextMut<'_, Self::Data>,
            _request: Resource<Request>,
        ) -> impl Future<
            Output = impl FnOnce(
                StoreContextMut<'_, Self::Data>,
            )
                -> wasmtime::Result<Result<Resource<Response>, ErrorCode>>
                         + 'static,
        > + Send
               + 'static {
            async move {
                move |_: StoreContextMut<'_, Self>| {
                    Err(anyhow!("no outbound request handler available"))
                }
            }
        }
    }

    async fn test_http_echo(component: &[u8], use_compression: bool) -> Result<()> {
        use {
            flate2::{
                write::{DeflateDecoder, DeflateEncoder},
                Compression,
            },
            std::io::Write,
        };

        init_logger();

        let mut config = Config::new();
        config.cranelift_debug_verifier(true);
        config.wasm_component_model(true);
        config.wasm_component_model_async(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;

        let component = Component::new(&engine, component)?;

        let mut linker = Linker::new(&engine);

        wasmtime_wasi::add_to_linker_async(&mut linker)?;
        wasi_http_draft::add_to_linker(&mut linker)?;

        let mut store = Store::new(
            &engine,
            Ctx {
                wasi: WasiCtxBuilder::new().inherit_stdio().build(),
                table: ResourceTable::default(),
                continue_: false,
                wakers: Arc::new(Mutex::new(None)),
            },
        );

        let proxy = proxy::Proxy::instantiate_async(&mut store, &component, &linker).await?;

        let headers = [("foo".into(), b"bar".into())];

        let body = b"And the mome raths outgrabe";

        enum Event {
            RequestBodyWrite(StreamWriter<u8>),
            RequestTrailersWrite,
            Response(Result<Resource<Response>, ErrorCode>),
            ResponseBodyRead(Option<(StreamReader<u8>, Vec<u8>)>),
            ResponseTrailersRead(Option<Resource<Fields>>),
        }

        let mut promises = PromisesUnordered::new();

        let (request_body_tx, request_body_rx) = component::stream(&mut store)?;

        promises.push(
            request_body_tx
                .write(
                    &mut store,
                    if use_compression {
                        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
                        encoder.write_all(body)?;
                        encoder.finish()?
                    } else {
                        body.to_vec()
                    },
                )?
                .map(Event::RequestBodyWrite),
        );

        let trailers = vec![("fizz".into(), b"buzz".into())];

        let (request_trailers_tx, request_trailers_rx) = component::future(&mut store)?;

        let request_trailers = IoView::table(store.data_mut()).push(Fields(trailers.clone()))?;

        promises.push(
            request_trailers_tx
                .write(&mut store, request_trailers)?
                .map(|()| Event::RequestTrailersWrite),
        );

        let request = IoView::table(store.data_mut()).push(Request {
            method: Method::Post,
            scheme: Some(Scheme::Http),
            path_with_query: Some("/".into()),
            authority: Some("localhost".into()),
            headers: Fields(
                headers
                    .iter()
                    .cloned()
                    .chain(if use_compression {
                        vec![
                            ("content-encoding".into(), b"deflate".into()),
                            ("accept-encoding".into(), b"deflate".into()),
                        ]
                    } else {
                        Vec::new()
                    })
                    .collect(),
            ),
            body: Body {
                stream: Some(request_body_rx),
                trailers: Some(request_trailers_rx),
            },
            options: None,
        })?;

        promises.push(
            proxy
                .wasi_http_handler()
                .call_handle(&mut store, request)
                .await?
                .map(Event::Response),
        );

        let mut response_body = Vec::new();
        let mut response_trailers = None;
        let mut received_trailers = false;
        while let Some(event) = promises.next(&mut store).await? {
            match event {
                Event::RequestBodyWrite(tx) => tx.close(&mut store)?,
                Event::RequestTrailersWrite => {}
                Event::Response(response) => {
                    let mut response = IoView::table(store.data_mut()).delete(response?)?;

                    assert!(response.status_code == 200);

                    assert!(headers.iter().all(|(k0, v0)| response
                        .headers
                        .0
                        .iter()
                        .any(|(k1, v1)| k0 == k1 && v0 == v1)));

                    if use_compression {
                        assert!(response.headers.0.iter().any(|(k, v)| matches!(
                            (k.as_str(), v.as_slice()),
                            ("content-encoding", b"deflate")
                        )));
                    }

                    response_trailers = response.body.trailers.take();

                    promises.push(
                        response
                            .body
                            .stream
                            .take()
                            .unwrap()
                            .read(&mut store)?
                            .map(Event::ResponseBodyRead),
                    );
                }
                Event::ResponseBodyRead(Some((rx, chunk))) => {
                    response_body.extend(chunk);
                    promises.push(rx.read(&mut store)?.map(Event::ResponseBodyRead));
                }
                Event::ResponseBodyRead(None) => {
                    let response_body = if use_compression {
                        let mut decoder = DeflateDecoder::new(Vec::new());
                        decoder.write_all(&response_body)?;
                        decoder.finish()?
                    } else {
                        response_body.clone()
                    };

                    assert_eq!(body as &[_], &response_body);

                    promises.push(
                        response_trailers
                            .take()
                            .unwrap()
                            .read(&mut store)?
                            .map(Event::ResponseTrailersRead),
                    );
                }
                Event::ResponseTrailersRead(Some(response_trailers)) => {
                    let response_trailers =
                        IoView::table(store.data_mut()).delete(response_trailers)?;

                    assert!(trailers.iter().all(|(k0, v0)| response_trailers
                        .0
                        .iter()
                        .any(|(k1, v1)| k0 == k1 && v0 == v1)));

                    received_trailers = true;
                }
                Event::ResponseTrailersRead(None) => panic!("expected response trailers; got none"),
            }
        }

        assert!(received_trailers);

        Ok(())
    }

    #[tokio::test]
    async fn async_http_echo() -> Result<()> {
        test_http_echo(
            &fs::read(test_programs_artifacts::ASYNC_HTTP_ECHO_COMPONENT).await?,
            false,
        )
        .await
    }

    #[tokio::test]
    async fn async_http_middleware() -> Result<()> {
        let echo = &fs::read(test_programs_artifacts::ASYNC_HTTP_ECHO_COMPONENT).await?;
        let middleware =
            &fs::read(test_programs_artifacts::ASYNC_HTTP_MIDDLEWARE_COMPONENT).await?;
        test_http_echo(&compose(middleware, echo).await?, true).await
    }

    #[tokio::test]
    async fn async_error_context() -> Result<()> {
        test_run(&fs::read(test_programs_artifacts::ASYNC_ERROR_CONTEXT_COMPONENT).await?).await
    }

    #[tokio::test]
    async fn async_error_context_callee() -> Result<()> {
        test_run(&fs::read(test_programs_artifacts::ASYNC_ERROR_CONTEXT_COMPONENT).await?).await
    }

    #[tokio::test]
    async fn async_error_context_caller() -> Result<()> {
        let caller =
            &fs::read(test_programs_artifacts::ASYNC_ERROR_CONTEXT_CALLER_COMPONENT).await?;
        let callee =
            &fs::read(test_programs_artifacts::ASYNC_ERROR_CONTEXT_CALLEE_COMPONENT).await?;
        test_run(&compose(caller, callee).await?).await
    }

    #[tokio::test]
    async fn async_error_context_roundtrip() -> Result<()> {
        let caller =
            &fs::read(test_programs_artifacts::ASYNC_ERROR_CONTEXT_CALLER_COMPONENT).await?;
        let callee =
            &fs::read(test_programs_artifacts::ASYNC_ERROR_CONTEXT_CALLEE_COMPONENT).await?;
        test_run(&compose(caller, callee).await?).await
    }

    // No-op function; we only test this by composing it in `async_error_context_stream_callee`
    #[allow(
        dead_code,
        reason = "here only to make the `assert_test_exists` macro happy"
    )]
    fn async_error_context_stream_callee() {}

    // No-op function; we only test this by composing it in `async_error_context_stream_caller`
    #[allow(
        dead_code,
        reason = "here only to make the `assert_test_exists` macro happy"
    )]
    fn async_error_context_stream_caller() {}

    #[tokio::test]
    async fn async_stream_end_err() -> Result<()> {
        let caller =
            &fs::read(test_programs_artifacts::ASYNC_ERROR_CONTEXT_STREAM_CALLER_COMPONENT).await?;
        let callee =
            &fs::read(test_programs_artifacts::ASYNC_ERROR_CONTEXT_STREAM_CALLEE_COMPONENT).await?;
        test_run(&compose(caller, callee).await?).await
    }
}
