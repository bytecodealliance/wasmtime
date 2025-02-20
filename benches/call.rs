use criterion::measurement::WallTime;
use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::Instant;
use wasmtime::*;

criterion_main!(benches);
criterion_group!(benches, measure_execution_time);

fn measure_execution_time(c: &mut Criterion) {
    host_to_wasm(c);
    wasm_to_host(c);

    #[cfg(feature = "component-model")]
    component::measure_execution_time(c);
}

#[derive(Copy, Clone)]
enum IsAsync {
    Yes,
    YesPooling,
    No,
    NoPooling,
}

impl IsAsync {
    fn desc(&self) -> &str {
        match self {
            IsAsync::Yes => "async",
            IsAsync::YesPooling => "async-pool",
            IsAsync::No => "sync",
            IsAsync::NoPooling => "sync-pool",
        }
    }
    fn use_async(&self) -> bool {
        match self {
            IsAsync::Yes | IsAsync::YesPooling => true,
            IsAsync::No | IsAsync::NoPooling => false,
        }
    }
}

fn engines() -> Vec<(Engine, IsAsync)> {
    let mut config = Config::new();

    #[cfg(feature = "component-model")]
    config.wasm_component_model(true);

    let mut pool = PoolingAllocationConfig::default();
    if std::env::var("WASMTIME_TEST_FORCE_MPK").is_ok() {
        pool.memory_protection_keys(MpkEnabled::Enable);
    }

    vec![
        (Engine::new(&config).unwrap(), IsAsync::No),
        (
            Engine::new(
                config
                    .clone()
                    .allocation_strategy(InstanceAllocationStrategy::Pooling(pool.clone())),
            )
            .unwrap(),
            IsAsync::NoPooling,
        ),
        (
            Engine::new(config.async_support(true)).unwrap(),
            IsAsync::Yes,
        ),
        (
            Engine::new(config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool)))
                .unwrap(),
            IsAsync::YesPooling,
        ),
    ]
}

/// Benchmarks the overhead of calling WebAssembly from the host in various
/// configurations.
fn host_to_wasm(c: &mut Criterion) {
    for (engine, is_async) in engines() {
        let mut store = Store::new(&engine, ());
        let module = Module::new(
            &engine,
            r#"(module
                (func (export "nop"))
                (func (export "nop-params-and-results") (param i32 i64) (result f32)
                    f32.const 0)
            )"#,
        )
        .unwrap();
        let instance = if is_async.use_async() {
            run_await(Instance::new_async(&mut store, &module, &[])).unwrap()
        } else {
            Instance::new(&mut store, &module, &[]).unwrap()
        };

        let bench_calls = |group: &mut BenchmarkGroup<'_, WallTime>, store: &mut Store<()>| {
            // Bench the overhead of a function that has no parameters or results
            bench_host_to_wasm::<(), ()>(group, store, &instance, is_async, "nop", (), ());
            // Bench the overhead of a function that has some parameters and just
            // one result (will use the raw system-v convention on applicable
            // platforms).
            bench_host_to_wasm::<(i32, i64), (f32,)>(
                group,
                store,
                &instance,
                is_async,
                "nop-params-and-results",
                (0, 0),
                (0.0,),
            );
        };

        // Bench once without any call hooks configured
        let name = format!("{}/no-hook", is_async.desc());
        bench_calls(&mut c.benchmark_group(&name), &mut store);

        // Bench again with a "call hook" enabled
        store.call_hook(|_, _| Ok(()));
        let name = format!("{}/hook-sync", is_async.desc());
        bench_calls(&mut c.benchmark_group(&name), &mut store);
    }
}

fn bench_host_to_wasm<Params, Results>(
    c: &mut BenchmarkGroup<'_, WallTime>,
    store: &mut Store<()>,
    instance: &Instance,
    is_async: IsAsync,
    name: &str,
    typed_params: Params,
    typed_results: Results,
) where
    Params: WasmParams + ToVals + Copy,
    Results: WasmResults + ToVals + Copy + PartialEq + Debug,
{
    // Benchmark the "typed" version, which should be faster than the versions
    // below.
    c.bench_function(&format!("core - host-to-wasm - typed - {name}"), |b| {
        let typed = instance
            .get_typed_func::<Params, Results>(&mut *store, name)
            .unwrap();
        b.iter(|| {
            let results = if is_async.use_async() {
                run_await(typed.call_async(&mut *store, typed_params)).unwrap()
            } else {
                typed.call(&mut *store, typed_params).unwrap()
            };
            assert_eq!(results, typed_results);
        })
    });

    // Benchmark the "untyped" version which should be the slowest of the three
    // here, but not unduly slow.
    c.bench_function(&format!("core - host-to-wasm - untyped - {name}"), |b| {
        let untyped = instance.get_func(&mut *store, name).unwrap();
        let params = typed_params.to_vals();
        let expected_results = typed_results.to_vals();
        let mut results = vec![Val::I32(0); expected_results.len()];
        b.iter(|| {
            if is_async.use_async() {
                run_await(untyped.call_async(&mut *store, &params, &mut results)).unwrap();
            } else {
                untyped.call(&mut *store, &params, &mut results).unwrap();
            }
            for (expected, actual) in expected_results.iter().zip(&results) {
                assert_vals_eq(expected, actual);
            }
        })
    });

    // Currently `call_async_unchecked` isn't implemented, so can't benchmark
    // below
    if is_async.use_async() {
        return;
    }

    // Benchmark the "unchecked" version which should be between the above two,
    // but is unsafe.
    c.bench_function(&format!("core - host-to-wasm - unchecked - {name}"), |b| {
        let untyped = instance.get_func(&mut *store, name).unwrap();
        let params = typed_params.to_vals();
        let results = typed_results.to_vals();
        let mut space = vec![ValRaw::i32(0); params.len().max(results.len())];
        b.iter(|| unsafe {
            for (i, param) in params.iter().enumerate() {
                space[i] = param.to_raw(&mut *store).unwrap();
            }
            untyped.call_unchecked(&mut *store, &mut space[..]).unwrap();
            for (i, expected) in results.iter().enumerate() {
                let ty = expected.ty(&store).unwrap();
                let actual = Val::from_raw(&mut *store, space[i], ty);
                assert_vals_eq(expected, &actual);
            }
        })
    });
}

/// Benchmarks the overhead of calling the host from WebAssembly itself
fn wasm_to_host(c: &mut Criterion) {
    let module = r#"(module
        ;; host imports with a variety of parameters/arguments
        (import "" "nop" (func $nop))
        (import "" "nop-params-and-results"
            (func $nop_params_and_results (param i32 i64) (result f32))
        )

        ;; "runner functions" for each of the above imports. Each runner
        ;; function takes the number of times to call the host function as
        ;; the duration of this entire loop will be measured.

        (func (export "run-nop") (param i64)
            loop
                call $nop

                local.get 0             ;; decrement & break if necessary
                i64.const -1
                i64.add
                local.tee 0
                i64.const 0
                i64.ne
                br_if 0
            end
        )

        (func (export "run-nop-params-and-results") (param i64)
            loop
                i32.const 0             ;; always zero parameters
                i64.const 0
                call $nop_params_and_results
                f32.const 0             ;; assert the correct result
                f32.eq
                i32.eqz
                if
                    unreachable
                end

                local.get 0             ;; decrement & break if necessary
                i64.const -1
                i64.add
                local.tee 0
                i64.const 0
                i64.ne
                br_if 0
            end
        )

    )"#;

    for (engine, is_async) in engines() {
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, module).unwrap();

        bench_calls(
            &mut c.benchmark_group(&format!("{}/no-hook", is_async.desc())),
            &mut store,
            &module,
            is_async,
        );
        store.call_hook(|_, _| Ok(()));
        bench_calls(
            &mut c.benchmark_group(&format!("{}/hook-sync", is_async.desc())),
            &mut store,
            &module,
            is_async,
        );
    }

    // Given a `Store` will create various instances hooked up to different ways
    // of defining host imports to benchmark their overhead.
    fn bench_calls(
        group: &mut BenchmarkGroup<'_, WallTime>,
        store: &mut Store<()>,
        module: &Module,
        is_async: IsAsync,
    ) {
        let engine = store.engine().clone();
        let mut typed = Linker::new(&engine);
        typed.func_wrap("", "nop", || {}).unwrap();
        typed
            .func_wrap("", "nop-params-and-results", |x: i32, y: i64| {
                assert_eq!(x, 0);
                assert_eq!(y, 0);
                0.0f32
            })
            .unwrap();
        let instance = if is_async.use_async() {
            run_await(typed.instantiate_async(&mut *store, &module)).unwrap()
        } else {
            typed.instantiate(&mut *store, &module).unwrap()
        };
        bench_instance(group, store, &instance, "typed", is_async);

        let mut untyped = Linker::new(&engine);
        untyped
            .func_new("", "nop", FuncType::new(&engine, [], []), |_, _, _| Ok(()))
            .unwrap();
        let ty = FuncType::new(&engine, [ValType::I32, ValType::I64], [ValType::F32]);
        untyped
            .func_new(
                "",
                "nop-params-and-results",
                ty,
                |_caller, params, results| {
                    assert_eq!(params.len(), 2);
                    match params[0] {
                        Val::I32(0) => {}
                        _ => unreachable!(),
                    }
                    match params[1] {
                        Val::I64(0) => {}
                        _ => unreachable!(),
                    }
                    assert_eq!(results.len(), 1);
                    results[0] = Val::F32(0);
                    Ok(())
                },
            )
            .unwrap();
        let instance = if is_async.use_async() {
            run_await(untyped.instantiate_async(&mut *store, &module)).unwrap()
        } else {
            untyped.instantiate(&mut *store, &module).unwrap()
        };
        bench_instance(group, store, &instance, "untyped", is_async);

        unsafe {
            let mut unchecked = Linker::new(&engine);
            unchecked
                .func_new_unchecked("", "nop", FuncType::new(&engine, [], []), |_, _| Ok(()))
                .unwrap();
            let ty = FuncType::new(&engine, [ValType::I32, ValType::I64], [ValType::F32]);
            unchecked
                .func_new_unchecked("", "nop-params-and-results", ty, |mut caller, space| {
                    match Val::from_raw(&mut caller, space[0], ValType::I32) {
                        Val::I32(0) => {}
                        _ => unreachable!(),
                    }
                    match Val::from_raw(&mut caller, space[1], ValType::I64) {
                        Val::I64(0) => {}
                        _ => unreachable!(),
                    }
                    space[0] = Val::F32(0).to_raw(&mut caller).unwrap();
                    Ok(())
                })
                .unwrap();
            let instance = if is_async.use_async() {
                run_await(unchecked.instantiate_async(&mut *store, &module)).unwrap()
            } else {
                unchecked.instantiate(&mut *store, &module).unwrap()
            };
            bench_instance(group, store, &instance, "unchecked", is_async);
        }

        // Only define async host imports if allowed
        if !is_async.use_async() {
            return;
        }

        let mut typed = Linker::new(&engine);
        typed
            .func_wrap_async("", "nop", |caller, _: ()| {
                Box::new(async {
                    drop(caller);
                })
            })
            .unwrap();
        typed
            .func_wrap_async(
                "",
                "nop-params-and-results",
                |_caller, (x, y): (i32, i64)| {
                    Box::new(async move {
                        assert_eq!(x, 0);
                        assert_eq!(y, 0);
                        0.0f32
                    })
                },
            )
            .unwrap();
        let instance = run_await(typed.instantiate_async(&mut *store, &module)).unwrap();
        bench_instance(group, store, &instance, "async-typed", is_async);
    }

    // Given a specific instance executes all of the "runner functions"
    fn bench_instance(
        group: &mut BenchmarkGroup<'_, WallTime>,
        store: &mut Store<()>,
        instance: &Instance,
        desc: &str,
        is_async: IsAsync,
    ) {
        group.bench_function(&format!("core - wasm-to-host - {desc} - nop"), |b| {
            let run = instance
                .get_typed_func::<u64, ()>(&mut *store, "run-nop")
                .unwrap();
            b.iter_custom(|iters| {
                let start = Instant::now();
                if is_async.use_async() {
                    run_await(run.call_async(&mut *store, iters)).unwrap();
                } else {
                    run.call(&mut *store, iters).unwrap();
                }
                start.elapsed()
            })
        });
        group.bench_function(
            &format!("core - wasm-to-host - {desc} - nop-params-and-results"),
            |b| {
                let run = instance
                    .get_typed_func::<u64, ()>(&mut *store, "run-nop-params-and-results")
                    .unwrap();
                b.iter_custom(|iters| {
                    let start = Instant::now();
                    if is_async.use_async() {
                        run_await(run.call_async(&mut *store, iters)).unwrap();
                    } else {
                        run.call(&mut *store, iters).unwrap();
                    }
                    start.elapsed()
                })
            },
        );
    }
}

fn assert_vals_eq(a: &Val, b: &Val) {
    match (a, b) {
        (Val::I32(a), Val::I32(b)) => assert_eq!(a, b),
        (Val::I64(a), Val::I64(b)) => assert_eq!(a, b),
        (Val::F32(a), Val::F32(b)) => assert_eq!(a, b),
        (Val::F64(a), Val::F64(b)) => assert_eq!(a, b),
        _ => unimplemented!(),
    }
}

trait ToVals {
    fn to_vals(&self) -> Vec<Val>;
}

macro_rules! tuples {
    ($($t:ident)*) => (
        #[allow(non_snake_case, reason = "macro-generated code")]
        impl<$($t:Copy + Into<Val>,)*> ToVals for ($($t,)*) {
            fn to_vals(&self) -> Vec<Val> {
                let mut _dst = Vec::new();
                let ($($t,)*) = *self;
                $(_dst.push($t.into());)*
                _dst
            }
        }
    )
}

tuples!();
tuples!(A);
tuples!(A B);
tuples!(A B C);

fn run_await<F: Future>(future: F) -> F::Output {
    let mut f = Pin::from(Box::new(future));
    let waker = dummy_waker();
    let mut cx = Context::from_waker(&waker);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(val) => break val,
            Poll::Pending => {}
        }
    }
}

fn dummy_waker() -> Waker {
    return unsafe { Waker::from_raw(clone(5 as *const _)) };

    unsafe fn clone(ptr: *const ()) -> RawWaker {
        assert_eq!(ptr as usize, 5);
        const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
        RawWaker::new(ptr, &VTABLE)
    }

    unsafe fn wake(ptr: *const ()) {
        assert_eq!(ptr as usize, 5);
    }

    unsafe fn wake_by_ref(ptr: *const ()) {
        assert_eq!(ptr as usize, 5);
    }

    unsafe fn drop(ptr: *const ()) {
        assert_eq!(ptr as usize, 5);
    }
}

#[cfg(feature = "component-model")]
mod component {
    use super::*;
    use wasmtime::component::{self, Component};

    pub fn measure_execution_time(c: &mut Criterion) {
        host_to_wasm(c);
        wasm_to_host(c);
    }

    trait ToComponentVal {
        fn to_component_val(&self) -> component::Val;
    }

    impl ToComponentVal for u32 {
        fn to_component_val(&self) -> component::Val {
            component::Val::U32(*self)
        }
    }

    impl ToComponentVal for u64 {
        fn to_component_val(&self) -> component::Val {
            component::Val::U64(*self)
        }
    }

    impl ToComponentVal for f32 {
        fn to_component_val(&self) -> component::Val {
            component::Val::Float32(*self)
        }
    }

    trait ToComponentVals {
        fn to_component_vals(&self) -> Vec<component::Val>;
    }

    macro_rules! tuples {
        ($($t:ident)*) => (
            #[allow(non_snake_case, reason = "macro-generated code")]
            impl<$($t:Copy + ToComponentVal,)*> ToComponentVals for ($($t,)*) {
                fn to_component_vals(&self) -> Vec<component::Val> {
                    let mut _dst = Vec::new();
                    let ($($t,)*) = *self;
                    $(_dst.push($t.to_component_val());)*
                    _dst
                }
            }
        )
    }

    tuples!();
    tuples!(A);
    tuples!(A B);
    tuples!(A B C);

    fn host_to_wasm(c: &mut Criterion) {
        for (engine, is_async) in engines() {
            let mut store = Store::new(&engine, ());

            let component = Component::new(
                &engine,
                r#"
                    (component
                        (core module $m
                            (func (export "nop"))
                            (func (export "nop-params-and-results") (param i32 i64) (result f32)
                                f32.const 0
                            )
                        )
                        (core instance $i (instantiate $m))
                        (func (export "nop")
                            (canon lift (core func $i "nop"))
                        )
                        (func (export "nop-params-and-results") (param "x" u32) (param "y" u64) (result float32)
                            (canon lift (core func $i "nop-params-and-results"))
                        )
                    )
                "#,
            )
            .unwrap();

            let linker = component::Linker::<()>::new(&engine);
            let instance = if is_async.use_async() {
                run_await(linker.instantiate_async(&mut store, &component)).unwrap()
            } else {
                linker.instantiate(&mut store, &component).unwrap()
            };

            let bench_calls = |group: &mut BenchmarkGroup<'_, WallTime>, store: &mut Store<()>| {
                // Bench the overhead of a function that has no parameters or results
                bench_host_to_wasm::<(), ()>(group, store, &instance, is_async, "nop", (), ());
                // Bench the overhead of a function that has some parameters and just
                // one result (will use the raw system-v convention on applicable
                // platforms).
                bench_host_to_wasm::<(u32, u64), (f32,)>(
                    group,
                    store,
                    &instance,
                    is_async,
                    "nop-params-and-results",
                    (0, 0),
                    (0.0,),
                );
            };

            // Bench once without any call hooks configured
            let name = format!("{}/no-hook", is_async.desc());
            bench_calls(&mut c.benchmark_group(&name), &mut store);

            // Bench again with a "call hook" enabled
            store.call_hook(|_, _| Ok(()));
            let name = format!("{}/hook-sync", is_async.desc());
            bench_calls(&mut c.benchmark_group(&name), &mut store);
        }
    }

    fn bench_host_to_wasm<Params, Results>(
        c: &mut BenchmarkGroup<'_, WallTime>,
        store: &mut Store<()>,
        instance: &component::Instance,
        is_async: IsAsync,
        name: &str,
        typed_params: Params,
        typed_results: Results,
    ) where
        Params:
            component::ComponentNamedList + ToComponentVals + component::Lower + Copy + Send + Sync,
        Results: component::ComponentNamedList
            + ToComponentVals
            + component::Lift
            + Copy
            + PartialEq
            + Debug
            + Send
            + Sync,
    {
        // Benchmark the "typed" version.
        c.bench_function(&format!("component - host-to-wasm - typed - {name}"), |b| {
            let typed = instance
                .get_typed_func::<Params, Results>(&mut *store, name)
                .unwrap();
            b.iter(|| {
                let results = if is_async.use_async() {
                    run_await(typed.call_async(&mut *store, typed_params)).unwrap()
                } else {
                    typed.call(&mut *store, typed_params).unwrap()
                };
                assert_eq!(results, typed_results);
                if is_async.use_async() {
                    run_await(typed.post_return_async(&mut *store)).unwrap()
                } else {
                    typed.post_return(&mut *store).unwrap()
                }
            })
        });

        // Benchmark the "untyped" version.
        c.bench_function(
            &format!("component - host-to-wasm - untyped - {name}"),
            |b| {
                let untyped = instance.get_func(&mut *store, name).unwrap();
                let params = typed_params.to_component_vals();
                let expected_results = typed_results.to_component_vals();
                let mut results = vec![component::Val::U32(0); expected_results.len()];
                b.iter(|| {
                    if is_async.use_async() {
                        run_await(untyped.call_async(&mut *store, &params, &mut results)).unwrap();
                    } else {
                        untyped.call(&mut *store, &params, &mut results).unwrap();
                    }
                    for (expected, actual) in expected_results.iter().zip(&results) {
                        assert_eq!(expected, actual);
                    }
                    if is_async.use_async() {
                        run_await(untyped.post_return_async(&mut *store)).unwrap()
                    } else {
                        untyped.post_return(&mut *store).unwrap()
                    }
                })
            },
        );
    }

    fn wasm_to_host(c: &mut Criterion) {
        let module = r#"
            (component
                (import "nop" (func $comp_nop))
                (import "nop-params-and-results" (func $comp_nop_params_and_results (param "x" u32) (param "y" u64) (result float32)))

                (core func $core_nop (canon lower (func $comp_nop)))
                (core func $core_nop_params_and_results (canon lower (func $comp_nop_params_and_results)))

                (core module $m
                    ;; host imports with a variety of parameters/arguments
                    (import "" "nop" (func $nop))
                    (import "" "nop-params-and-results"
                        (func $nop_params_and_results (param i32 i64) (result f32))
                    )

                    ;; "runner functions" for each of the above imports. Each runner
                    ;; function takes the number of times to call the host function as
                    ;; the duration of this entire loop will be measured.

                    (func (export "run-nop") (param i64)
                        loop
                            call $nop

                            local.get 0             ;; decrement & break if necessary
                            i64.const -1
                            i64.add
                            local.tee 0
                            i64.const 0
                            i64.ne
                            br_if 0
                        end
                    )

                    (func (export "run-nop-params-and-results") (param i64)
                        loop
                            i32.const 0             ;; always zero parameters
                            i64.const 0
                            call $nop_params_and_results
                            f32.const 0             ;; assert the correct result
                            f32.eq
                            i32.eqz
                            if
                                unreachable
                            end

                            local.get 0             ;; decrement & break if necessary
                            i64.const -1
                            i64.add
                            local.tee 0
                            i64.const 0
                            i64.ne
                            br_if 0
                        end
                    )
                )

                (core instance $i (instantiate $m (with "" (instance
                  (export "nop" (func $core_nop))
                  (export "nop-params-and-results" (func $core_nop_params_and_results))
                ))))

                (func (export "run-nop") (param "i" u64)
                    (canon lift (core func $i "run-nop"))
                )
                (func (export "run-nop-params-and-results") (param "i" u64)
                    (canon lift (core func $i "run-nop-params-and-results"))
                )
            )
        "#;

        for (engine, is_async) in engines() {
            let mut store = Store::new(&engine, ());
            let component = component::Component::new(&engine, module).unwrap();

            bench_calls(
                &mut c.benchmark_group(&format!("{}/no-hook", is_async.desc())),
                &mut store,
                &component,
                is_async,
            );
            store.call_hook(|_, _| Ok(()));
            bench_calls(
                &mut c.benchmark_group(&format!("{}/hook-sync", is_async.desc())),
                &mut store,
                &component,
                is_async,
            );
        }

        // Given a `Store` will create various instances hooked up to different ways
        // of defining host imports to benchmark their overhead.
        fn bench_calls(
            group: &mut BenchmarkGroup<'_, WallTime>,
            store: &mut Store<()>,
            component: &component::Component,
            is_async: IsAsync,
        ) {
            let engine = store.engine().clone();
            let mut typed = component::Linker::new(&engine);
            typed.root().func_wrap("nop", |_, ()| Ok(())).unwrap();
            typed
                .root()
                .func_wrap("nop-params-and-results", |_, (x, y): (u32, u64)| {
                    assert_eq!(x, 0);
                    assert_eq!(y, 0);
                    Ok((0.0f32,))
                })
                .unwrap();
            let instance = if is_async.use_async() {
                run_await(typed.instantiate_async(&mut *store, &component)).unwrap()
            } else {
                typed.instantiate(&mut *store, &component).unwrap()
            };
            bench_instance(group, store, &instance, "typed", is_async);

            let mut untyped = component::Linker::new(&engine);
            untyped.root().func_new("nop", |_, _, _| Ok(())).unwrap();
            untyped
                .root()
                .func_new("nop-params-and-results", |_caller, params, results| {
                    assert_eq!(params.len(), 2);
                    match params[0] {
                        component::Val::U32(0) => {}
                        _ => unreachable!(),
                    }
                    match params[1] {
                        component::Val::U64(0) => {}
                        _ => unreachable!(),
                    }
                    assert_eq!(results.len(), 1);
                    results[0] = component::Val::Float32(0.0);
                    Ok(())
                })
                .unwrap();
            let instance = if is_async.use_async() {
                run_await(untyped.instantiate_async(&mut *store, &component)).unwrap()
            } else {
                untyped.instantiate(&mut *store, &component).unwrap()
            };
            bench_instance(group, store, &instance, "untyped", is_async);

            // Only define async host imports if allowed
            if !is_async.use_async() {
                return;
            }

            let mut typed = component::Linker::new(&engine);
            typed
                .root()
                .func_wrap_async("nop", |caller, ()| {
                    Box::new(async {
                        drop(caller);
                        Ok(())
                    })
                })
                .unwrap();
            typed
                .root()
                .func_wrap_async("nop-params-and-results", |_caller, (x, y): (u32, u64)| {
                    Box::new(async move {
                        assert_eq!(x, 0);
                        assert_eq!(y, 0);
                        Ok((0.0f32,))
                    })
                })
                .unwrap();
            let instance = run_await(typed.instantiate_async(&mut *store, &component)).unwrap();
            bench_instance(group, store, &instance, "async-typed", is_async);
        }

        // Given a specific instance executes all of the "runner functions"
        fn bench_instance(
            group: &mut BenchmarkGroup<'_, WallTime>,
            store: &mut Store<()>,
            instance: &component::Instance,
            desc: &str,
            is_async: IsAsync,
        ) {
            group.bench_function(&format!("component - wasm-to-host - {desc} - nop"), |b| {
                let run = instance
                    .get_typed_func::<(u64,), ()>(&mut *store, "run-nop")
                    .unwrap();
                b.iter_custom(|iters| {
                    let start = Instant::now();
                    if is_async.use_async() {
                        run_await(run.call_async(&mut *store, (iters,))).unwrap();
                        run_await(run.post_return_async(&mut *store)).unwrap();
                    } else {
                        run.call(&mut *store, (iters,)).unwrap();
                        run.post_return(&mut *store).unwrap();
                    }
                    start.elapsed()
                })
            });
            group.bench_function(
                &format!("component - wasm-to-host - {desc} - nop-params-and-results"),
                |b| {
                    let run = instance
                        .get_typed_func::<(u64,), ()>(&mut *store, "run-nop-params-and-results")
                        .unwrap();
                    b.iter_custom(|iters| {
                        let start = Instant::now();
                        if is_async.use_async() {
                            run_await(run.call_async(&mut *store, (iters,))).unwrap();
                            run_await(run.post_return_async(&mut *store)).unwrap();
                        } else {
                            run.call(&mut *store, (iters,)).unwrap();
                            run.post_return(&mut *store).unwrap();
                        }
                        start.elapsed()
                    })
                },
            );
        }
    }
}
