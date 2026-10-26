#![cfg(arc_try_new)]

use wasmtime::{
    Config, Engine, Func, FuncType, Instance, Linker, Module, Ref, Result, Store, Table, Val,
};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn caller_get_export() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"(module
            (import "" "f" (func))
            (memory (export "m") 1)
            (func (export "run") (call 0))
        )"#,
    )?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let host_func = Func::try_new(
            &mut store,
            FuncType::try_new(&engine, [], [])?,
            |mut caller, _params, _results| {
                let mem = caller.get_export("m");
                assert!(mem.is_some());
                Ok(())
            },
        )?;
        let instance = wasmtime::Instance::new(&mut store, &module, &[host_func.into()])?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        run.call(&mut store, ())?;
        Ok(())
    })
}

#[test]
fn caller_data() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"(module
            (import "" "f" (func))
            (func (export "run") (call 0))
        )"#,
    )?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, 42u32)?;
        let host_func = Func::try_new(
            &mut store,
            FuncType::try_new(&engine, [], [])?,
            |caller, _params, _results| {
                assert_eq!(*caller.data(), 42u32);
                Ok(())
            },
        )?;
        let instance = wasmtime::Instance::new(&mut store, &module, &[host_func.into()])?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        run.call(&mut store, ())?;
        Ok(())
    })
}

#[test]
fn caller_engine() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"(module
            (import "" "f" (func))
            (func (export "run") (call 0))
        )"#,
    )?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let host_func = Func::try_new(
            &mut store,
            FuncType::try_new(&engine, [], [])?,
            |caller, _params, _results| {
                let _engine = caller.engine();
                Ok(())
            },
        )?;
        let instance = wasmtime::Instance::new(&mut store, &module, &[host_func.into()])?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        run.call(&mut store, ())?;
        Ok(())
    })
}

/// Check that an OOM during a call, or while mutating an instance's VM
/// state, leaves the store usable.
///
/// The scratch buffers (`wasm_val_raw_storage`, `hostcall_val_storage`) are
/// `mem::take`n at the start of a call and saved back at the end, so a call
/// that errors in between leaves them for the next one.
#[tokio::test]
async fn call_scratch_and_vm_state_reuse_store() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    // No room to grow in place, so `memory.grow` must relocate the base that
    // compiled code reads.
    config.memory_reservation(1 << 16);
    config.memory_reservation_for_growth(0);
    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "host" (func $host (param i32) (result i32)))

                (memory (export "mem") 1)
                (table (export "tab") 4 funcref)
                (global (export "g") (mut i32) (i32.const 0))

                (func $forty_two (result i32) (i32.const 42))
                (elem (i32.const 0) func $forty_two)

                ;; Calls back out to the host and back in, so a call exercises
                ;; both scratch buffers.
                (func (export "round_trip") (param i32) (result i32)
                    (call $host (local.get 0)))

                ;; Grows memory, relocating its base.
                (func (export "grow") (param i32) (result i32)
                    (memory.grow (local.get 0)))

                (func (export "indirect") (param i32) (result i32)
                    (call_indirect (type 1) (local.get 0)))
            )
        "#,
    )?;

    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("", "host", |x: i32| x * 2)?;
    let instance_pre = linker.instantiate_pre(&module)?;

    /// Exercise the call paths and in-place VM state mutations.
    async fn exercise(store: &mut Store<()>, instance: Instance) -> Result<()> {
        // Each of these takes and restores the scratch buffers.
        let round_trip = instance.get_func(&mut *store, "round_trip").unwrap();
        let mut results = [Val::I32(0)];
        round_trip.call(&mut *store, &[Val::I32(21)], &mut results)?;
        assert_eq!(results[0].unwrap_i32(), 42);
        round_trip
            .call_async(&mut *store, &[Val::I32(21)], &mut results)
            .await?;
        assert_eq!(results[0].unwrap_i32(), 42);

        let typed = round_trip.typed::<i32, i32>(&*store)?;
        assert_eq!(typed.call(&mut *store, 21)?, 42);
        assert_eq!(typed.call_async(&mut *store, 21).await?, 42);

        // `Table::get` lazily initializes elements in place.
        let table = instance.get_table(&mut *store, "tab").unwrap();
        assert!(table.get(&mut *store, 0).is_some());
        let indirect = instance.get_typed_func::<i32, i32>(&mut *store, "indirect")?;
        assert_eq!(indirect.call(&mut *store, 0)?, 42);

        let old_size = table.grow(&mut *store, 2, Ref::Func(None))?;
        assert_eq!(old_size, 4);
        table.fill(&mut *store, 4, Ref::Func(None), 2)?;
        table.set(&mut *store, 5, Ref::Func(None))?;
        Table::copy(&mut *store, &table, 2, &table, 0, 2)?;

        let memory = instance.get_memory(&mut *store, "mem").unwrap();
        memory.write(&mut *store, 0, &[1, 2, 3, 4])?;
        memory.data_mut(&mut *store)[4] = 5;
        let old_pages = memory.grow(&mut *store, 1)?;
        assert_eq!(old_pages, 1);

        let mut buf = [0u8; 5];
        memory.read(&*store, 0, &mut buf)?;
        assert_eq!(buf, [1, 2, 3, 4, 5]);

        // `memory.grow` reports failure in-band as -1, and whether there is
        // room depends on the host, so only require that it agrees with the
        // embedder API.
        let grow = instance.get_typed_func::<i32, i32>(&mut *store, "grow")?;
        let grown = grow.call(&mut *store, 1)?;
        if grown != -1 {
            assert_eq!(grown, 2);
            assert_eq!(memory.size(&*store), 3);
        } else {
            assert_eq!(memory.size(&*store), 2);
        }

        let global = instance.get_global(&mut *store, "g").unwrap();
        global.set(&mut *store, Val::I32(7))?;
        assert_eq!(global.get(&mut *store).unwrap_i32(), 7);

        Ok(())
    }

    OomTest::new()
        .allow_alloc_after_oom(true)
        .alloc_succeeds_after_oom(true)
        // `memory.grow` reports failure in-band, so a run can finish without
        // surfacing the injected OOM.
        .allow_missed_oom_errors(true)
        .test_with_reuse_async(
            async || Store::try_new(&engine, ()),
            async |store: &mut Store<()>| {
                let instance = instance_pre.instantiate_async(&mut *store).await?;
                exercise(store, instance).await
            },
            async |mut store: Store<()>| {
                // A fresh instance must run every path again on the
                // inherited buffers and VM state.
                let instance = instance_pre.instantiate_async(&mut store).await?;
                exercise(&mut store, instance).await
            },
        )
        .await
}
