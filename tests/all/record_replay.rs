use wasmtime::{rr::Trace, *};

fn engine(mode: RRConfig) -> Result<Engine> {
    let mut config = Config::new();
    config.rr(mode);
    Engine::new(&config)
}

fn identity() -> Result<(Store<()>, Func, Memory)> {
    let engine = engine(RRConfig::Recording)?;
    let mut store = Store::new(&engine, ());
    store.start_recording()?;
    let module = Module::new(
        &engine,
        r#"(module
        (memory (export "memory") 1)
        (func (export "id") (param i32 i64 f32 f64 v128) (result i32 i64 f32 f64 v128)
            local.get 0 local.get 1 local.get 2 local.get 3 local.get 4))"#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_func(&mut store, "id").unwrap();
    let memory = instance.get_memory(&mut store, "memory").unwrap();
    Ok((store, func, memory))
}

#[test]
fn framing_rejects_truncation_and_trailing_data() -> Result<()> {
    let (mut store, _, _) = identity()?;
    let trace = store.finish_recording()?;
    for len in 0..trace.as_bytes().len() {
        assert!(Trace::from_bytes(trace.as_bytes()[..len].to_vec()).is_err());
    }
    let mut extra = trace.as_bytes().to_vec();
    extra.push(0);
    assert!(Trace::from_bytes(extra).is_err());
    Ok(())
}

#[test]
fn recording_requires_an_empty_store() -> Result<()> {
    // Application data and precompiled modules do not populate the store.
    let record_engine = engine(RRConfig::Recording)?;
    let mut record = Store::new(&record_engine, 123);
    record.start_recording()?;
    record.finish_recording()?;

    let constructors: &[fn(&mut Store<i32>) -> Result<()>] = &[
        |store| {
            Func::wrap(store, || ());
            Ok(())
        },
        |store| {
            Memory::new(store, MemoryType::new(0, None)?)?;
            Ok(())
        },
        |store| {
            Global::new(
                store,
                GlobalType::new(ValType::I32, Mutability::Const),
                Val::I32(0),
            )?;
            Ok(())
        },
        |store| {
            Table::new(
                store,
                TableType::new(RefType::FUNCREF, 0, None)?,
                Ref::Func(None),
            )?;
            Ok(())
        },
        |store| {
            let module = Module::new(store.engine(), "(module)")?;
            Instance::new(store, &module, &[])?;
            Ok(())
        },
    ];
    for construct in constructors {
        let mut record = Store::new(&record_engine, 123);
        construct(&mut record)?;
        assert!(
            record
                .start_recording()
                .unwrap_err()
                .to_string()
                .contains("empty store")
        );
        assert_eq!(*record.data(), 123);
    }
    Ok(())
}

#[test]
fn forbidden_mutation_poisons_even_if_error_is_caught() -> Result<()> {
    let (mut store, _, _) = identity()?;
    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::I32, Mutability::Var),
        Val::I32(0),
    )?;
    assert!(global.set(&mut store, Val::I32(1)).is_err());
    assert!(store.finish_recording().is_err());

    let (mut store, _, _) = identity()?;
    let table = Table::new(
        &mut store,
        TableType::new(RefType::FUNCREF, 1, None)?,
        Ref::Func(None),
    )?;
    assert!(table.set(&mut store, 0, Ref::Func(None)).is_err());
    assert!(store.finish_recording().is_err());
    Ok(())
}

#[test]
fn rr_config_enables_determinism_but_rejects_explicit_conflicts() -> Result<()> {
    engine(RRConfig::Recording)?;
    engine(RRConfig::Replaying)?;
    let mut config = Config::new();
    config
        .cranelift_nan_canonicalization(false)
        .rr(RRConfig::Recording);
    assert!(Engine::new(&config).is_err());
    let mut config = Config::new();
    config
        .rr(RRConfig::Replaying)
        .relaxed_simd_deterministic(false);
    assert!(Engine::new(&config).is_err());
    Ok(())
}

/// The bits of `f32.add`'s and relaxed `madd`'s results for a non-canonical
/// NaN, `0x7fa00000`, under `config`.
fn nan_results(config: &Config) -> Result<[u32; 2]> {
    let engine = Engine::new(config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
                (func (export "add") (param i32) (result i32)
                    (i32.reinterpret_f32
                        (f32.add (f32.reinterpret_i32 (local.get 0)) (f32.const 0))))
                (func (export "madd") (param i32) (result i32)
                    (local v128)
                    (local.set 1 (f32x4.splat (f32.reinterpret_i32 (local.get 0))))
                    (i32.reinterpret_f32
                        (f32x4.extract_lane 0
                            (f32x4.relaxed_madd (local.get 1) (local.get 1) (local.get 1))))))
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let mut results = [0; 2];
    for (result, name) in results.iter_mut().zip(["add", "madd"]) {
        let func = instance.get_typed_func::<u32, u32>(&mut store, name)?;
        *result = func.call(&mut store, 0x7fa0_0000)?;
    }
    Ok(results)
}

#[test]
fn rr_config_canonicalizes_nans_regardless_of_option_order() -> Result<()> {
    let mut config = Config::new();
    config.wasm_relaxed_simd(true).rr(RRConfig::Recording);
    // Resetting the compiler configuration does not lose the setting.
    config.enable_compiler(false).enable_compiler(true);
    assert_eq!(nan_results(&config)?, [0x7fc0_0000; 2]);
    Ok(())
}

#[test]
#[cfg(target_arch = "x86_64")]
fn fma_builtin_results_match_native_fma() -> Result<()> {
    let mut config = Config::new();
    config.wasm_relaxed_simd(true).rr(RRConfig::Recording);
    // SAFETY: disabling a CPU feature only makes code use fallbacks.
    unsafe {
        config.cranelift_flag_set("has_fma", "false");
    }
    assert_eq!(nan_results(&config)?, [0x7fc0_0000; 2]);
    Ok(())
}

#[cfg(feature = "gc")]
#[test]
fn gc_reference_boundaries_are_rejected() -> Result<()> {
    let engine = engine(RRConfig::Recording)?;
    let mut store = Store::new(&engine, ());
    store.start_recording()?;
    let host = Func::new(
        &mut store,
        FuncType::new(&engine, [ValType::EXTERNREF], []),
        |_, _, _| Ok(()),
    );
    let module = Module::new(
        &engine,
        r#"(module (import "" "host" (func (param externref))))"#,
    )?;
    assert!(Instance::new(&mut store, &module, &[host.into()]).is_err());
    assert!(store.finish_recording().is_err());
    Ok(())
}

/// A store recording with `config`, after `setup`, and a module whose export
/// `spin` loops as many times as its argument says.
///
/// Instantiation runs Wasm (record/replay engines give every module a startup
/// function), so `setup` provides fuel or an epoch deadline beforehand.
fn spinner(
    config: &mut Config,
    setup: impl FnOnce(&mut Store<()>) -> Result<()>,
) -> Result<(Store<()>, Module)> {
    config.rr(RRConfig::Recording);
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());
    setup(&mut store)?;
    store.start_recording()?;
    let module = Module::new(
        &engine,
        r#"(module
        (func (export "spin") (param i32)
            (loop
                (local.set 0 (i32.sub (local.get 0) (i32.const 1)))
                (br_if 0 (local.get 0)))))"#,
    )?;
    Ok((store, module))
}

fn spin(store: &mut Store<()>, module: &Module) -> Result<TypedFunc<i32, ()>> {
    let instance = Instance::new(&mut *store, module, &[])?;
    instance.get_typed_func(store, "spin")
}

async fn spin_async(store: &mut Store<()>, module: &Module) -> Result<TypedFunc<i32, ()>> {
    let instance = Instance::new_async(&mut *store, module, &[]).await?;
    instance.get_typed_func(store, "spin")
}

/// Runs `future` to completion, returning its output and how many times it
/// yielded.
async fn count_yields<T>(future: impl Future<Output = T>) -> (T, usize) {
    let mut future = std::pin::pin!(future);
    let mut yields = 0;
    let output = std::future::poll_fn(|cx| {
        let poll = future.as_mut().poll(cx);
        if poll.is_pending() {
            yields += 1;
        }
        poll
    })
    .await;
    (output, yields)
}

#[test]
fn fuel_is_recorded_unless_it_runs_out() -> Result<()> {
    let fuel = |fuel| move |store: &mut Store<()>| store.set_fuel(fuel);
    let (mut store, module) = spinner(Config::new().consume_fuel(true), fuel(10_000))?;
    spin(&mut store, &module)?.call(&mut store, 10)?;
    store.finish_recording()?;

    let (mut store, module) = spinner(Config::new().consume_fuel(true), fuel(100))?;
    let err = spin(&mut store, &module)?
        .call(&mut store, 1000)
        .unwrap_err();
    assert_eq!(err.downcast::<Trap>()?, Trap::OutOfFuel);
    let err = store.finish_recording().err().unwrap().to_string();
    assert!(err.contains("fuel check"), "{err}");
    Ok(())
}

#[tokio::test]
async fn fuel_yields_are_recorded() -> Result<()> {
    let (mut store, module) = spinner(Config::new().consume_fuel(true), |store| {
        store.set_fuel(10_000)?;
        store.fuel_async_yield_interval(Some(10))
    })?;
    let spin = spin_async(&mut store, &module).await?;
    let (result, yields) = count_yields(spin.call_async(&mut store, 100)).await;
    result?;
    assert!(yields > 0);
    store.finish_recording()?;
    Ok(())
}

#[test]
fn epoch_traps_poison_the_recording() -> Result<()> {
    let (mut store, module) = spinner(Config::new().epoch_interruption(true), |store| {
        store.set_epoch_deadline(1);
        Ok(())
    })?;
    let spin = spin(&mut store, &module)?;
    spin.call(&mut store, 10)?;
    store.engine().increment_epoch();
    let err = spin.call(&mut store, 10).unwrap_err();
    assert_eq!(err.downcast::<Trap>()?, Trap::Interrupt);
    let err = store.finish_recording().err().unwrap().to_string();
    assert!(err.contains("epoch check"), "{err}");
    Ok(())
}

#[tokio::test]
async fn epoch_yields_are_recorded() -> Result<()> {
    let (mut store, module) = spinner(Config::new().epoch_interruption(true), |store| {
        store.set_epoch_deadline(1);
        store.epoch_deadline_async_yield_and_update(1);
        Ok(())
    })?;
    let spin = spin_async(&mut store, &module).await?;
    store.engine().increment_epoch();
    let (result, yields) = count_yields(spin.call_async(&mut store, 10)).await;
    result?;
    assert_eq!(yields, 1);
    store.finish_recording()?;
    Ok(())
}

#[test]
fn epoch_deadline_callbacks_are_rejected() -> Result<()> {
    let mut config = Config::new();
    config.epoch_interruption(true).rr(RRConfig::Recording);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    store.epoch_deadline_callback(|_| Ok(UpdateDeadline::Continue(1)));
    let err = store.start_recording().unwrap_err().to_string();
    assert!(err.contains("epoch deadline callbacks"), "{err}");

    // Going back to trapping makes the store recordable again.
    store.epoch_deadline_trap();
    store.start_recording()?;
    store.epoch_deadline_callback(|_| Ok(UpdateDeadline::Continue(1)));
    let err = store.finish_recording().err().unwrap().to_string();
    assert!(err.contains("epoch deadline callbacks"), "{err}");
    Ok(())
}

#[tokio::test]
async fn cancelling_at_a_yield_poisons_the_recording() -> Result<()> {
    let (mut store, module) = spinner(Config::new().epoch_interruption(true), |store| {
        store.set_epoch_deadline(1);
        store.epoch_deadline_async_yield_and_update(1);
        Ok(())
    })?;
    let spin = spin_async(&mut store, &module).await?;
    store.engine().increment_epoch();
    {
        // Poll the call until it yields, then drop it.
        let mut call = std::pin::pin!(spin.call_async(&mut store, 10));
        std::future::poll_fn(|cx| {
            assert!(call.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
    }
    let err = store.finish_recording().err().unwrap().to_string();
    assert!(err.contains("epoch check"), "{err}");
    Ok(())
}
