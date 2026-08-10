#![cfg(not(miri))]

use crate::async_functions::{CountPending, PollOnce};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use wasmtime::format_err;
use wasmtime::*;
use wasmtime_test_macros::wasmtime_test;

fn build_engine(config: &mut Config) -> Result<Arc<Engine>> {
    config.epoch_interruption(true);
    Ok(Arc::new(Engine::new(&config)?))
}

fn make_env<T: 'static>(engine: &Engine) -> Linker<T> {
    let mut linker = Linker::new(engine);
    let engine = engine.clone();

    linker
        .func_new(
            "",
            "bump_epoch",
            FuncType::new(&engine, None, None),
            move |_caller, _params, _results| {
                engine.increment_epoch();
                Ok(())
            },
        )
        .unwrap();

    linker
}

enum InterruptMode {
    Trap,
    Callback(fn(StoreContextMut<usize>) -> Result<UpdateDeadline>),
    Yield(u64),
}

/// Run a test with the given wasm, giving an initial deadline of
/// `initial` ticks in the future, and either configuring the wasm to
/// yield and set a deadline `delta` ticks in the future if `delta` is
/// `Some(..)` or trapping if `delta` is `None`.
///
/// Returns `Some((yields, store))` if function completed normally, giving
/// the number of yields that occurred, or `None` if a trap occurred.
async fn run_and_count_yields_or_trap<F: Fn(Arc<Engine>)>(
    config: &mut Config,
    wasm: &str,
    initial: u64,
    delta: InterruptMode,
    setup_func: F,
) -> Result<Option<(usize, usize)>> {
    let engine = build_engine(config)?;
    let linker = make_env::<usize>(&engine);
    let module = Module::new(&engine, wasm)?;
    let mut store = Store::new(&engine, 0);
    store.set_epoch_deadline(initial);
    match delta {
        InterruptMode::Yield(delta) => {
            store.epoch_deadline_async_yield_and_update(delta);
        }
        InterruptMode::Callback(func) => {
            store.epoch_deadline_callback(func);
        }
        InterruptMode::Trap => {
            store.epoch_deadline_trap();
        }
    }

    let engine_clone = engine.clone();
    setup_func(engine_clone);

    let instance = linker.instantiate_async(&mut store, &module).await?;
    let f = instance.get_func(&mut store, "run").unwrap();
    let (result, yields) =
        CountPending::new(Box::pin(f.call_async(&mut store, &[], &mut []))).await;
    let store = store.data();
    Ok(result.ok().map(|_| (yields, *store)))
}

#[wasmtime_test]
async fn epoch_yield_at_func_entry(config: &mut Config) -> Result<()> {
    // Should yield at start of call to func $subfunc.
    assert_eq!(
        Some((1, 0)),
        run_and_count_yields_or_trap(
            config,
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")
                    call $bump  ;; bump epoch
                    call $subfunc) ;; call func; will notice new epoch and yield
                (func $subfunc))
            ",
            1,
            InterruptMode::Yield(1),
            |_| {},
        )
        .await?
    );
    Ok(())
}

#[wasmtime_test]
async fn epoch_yield_at_loop_header(config: &mut Config) -> Result<()> {
    // Should yield at top of loop, once per five iters.
    assert_eq!(
        Some((2, 0)),
        run_and_count_yields_or_trap(
            config,
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")
                    (local $i i32)
                    (local.set $i (i32.const 10))
                    (loop $l
                        call $bump
                        (br_if $l (local.tee $i (i32.sub (local.get $i) (i32.const 1)))))))
            ",
            0,
            InterruptMode::Yield(5),
            |_| {},
        )
        .await?
    );
    Ok(())
}

#[wasmtime_test]
async fn epoch_yield_immediate(config: &mut Config) -> Result<()> {
    // We should see one yield immediately when the initial deadline
    // is zero.
    assert_eq!(
        Some((1, 0)),
        run_and_count_yields_or_trap(
            config,
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")))
            ",
            0,
            InterruptMode::Yield(1),
            |_| {},
        )
        .await?
    );
    Ok(())
}

#[wasmtime_test]
async fn epoch_yield_only_once(config: &mut Config) -> Result<()> {
    // We should yield from the subfunction, and then when we return
    // to the outer function and hit another loop header, we should
    // not yield again (the double-check block will reload the correct
    // epoch).
    assert_eq!(
        Some((1, 0)),
        run_and_count_yields_or_trap(
            config,
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")
                  (local $i i32)
                  (call $subfunc)
                  (local.set $i (i32.const 0))
                  (loop $l
                    (br_if $l (i32.eq (i32.const 10)
                                      (local.tee $i (i32.add (i32.const 1) (local.get $i)))))))
                (func $subfunc
                  (call $bump)))
            ",
            1,
            InterruptMode::Yield(1),
            |_| {},
        )
        .await?
    );
    Ok(())
}

#[wasmtime_test]
async fn epoch_interrupt_infinite_loop(config: &mut Config) -> Result<()> {
    assert_eq!(
        None,
        run_and_count_yields_or_trap(
            config,
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")
                  (loop $l
                    (br $l))))
            ",
            1,
            InterruptMode::Trap,
            |engine| {
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    engine.increment_epoch();
                });
            },
        )
        .await?
    );
    Ok(())
}

#[wasmtime_test]
async fn epoch_interrupt_function_entries(config: &mut Config) -> Result<()> {
    assert_eq!(
        None,
        run_and_count_yields_or_trap(
            config,
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")
                  call $f1
                  call $f1
                  call $f1
                  call $f1
                  call $f1
                  call $f1
                  call $f1
                  call $f1
                  call $f1
                  call $f1)
                (func $f1
                  call $f2
                  call $f2
                  call $f2
                  call $f2
                  call $f2
                  call $f2
                  call $f2
                  call $f2
                  call $f2
                  call $f2)
                (func $f2
                  call $f3
                  call $f3
                  call $f3
                  call $f3
                  call $f3
                  call $f3
                  call $f3
                  call $f3
                  call $f3
                  call $f3)
                (func $f3
                  call $f4
                  call $f4
                  call $f4
                  call $f4
                  call $f4
                  call $f4
                  call $f4
                  call $f4
                  call $f4
                  call $f4)
                (func $f4
                  call $f5
                  call $f5
                  call $f5
                  call $f5
                  call $f5
                  call $f5
                  call $f5
                  call $f5
                  call $f5
                  call $f5)
                (func $f5
                  call $f6
                  call $f6
                  call $f6
                  call $f6
                  call $f6
                  call $f6
                  call $f6
                  call $f6
                  call $f6
                  call $f6)
                (func $f6
                  call $f7
                  call $f7
                  call $f7
                  call $f7
                  call $f7
                  call $f7
                  call $f7
                  call $f7
                  call $f7
                  call $f7)
                (func $f7
                  call $f8
                  call $f8
                  call $f8
                  call $f8
                  call $f8
                  call $f8
                  call $f8
                  call $f8
                  call $f8
                  call $f8)
                (func $f8
                  call $f9
                  call $f9
                  call $f9
                  call $f9
                  call $f9
                  call $f9
                  call $f9
                  call $f9
                  call $f9
                  call $f9)
                (func $f9))
            ",
            1,
            InterruptMode::Trap,
            |engine| {
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    engine.increment_epoch();
                });
            },
        )
        .await?
    );
    Ok(())
}

#[wasmtime_test]
async fn epoch_callback_continue(config: &mut Config) -> Result<()> {
    assert_eq!(
        Some((0, 1)),
        run_and_count_yields_or_trap(
            config,
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")
                    call $bump  ;; bump epoch
                    call $subfunc) ;; call func; will notice new epoch and yield
                (func $subfunc))
            ",
            1,
            InterruptMode::Callback(|mut cx| {
                let s = cx.data_mut();
                *s += 1;
                Ok(UpdateDeadline::Continue(1))
            }),
            |_| {},
        )
        .await?
    );
    Ok(())
}

#[wasmtime_test]
async fn epoch_callback_yield(config: &mut Config) -> Result<()> {
    assert_eq!(
        Some((1, 1)),
        run_and_count_yields_or_trap(
            config,
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")
                    call $bump  ;; bump epoch
                    call $subfunc) ;; call func; will notice new epoch and yield
                (func $subfunc))
            ",
            1,
            InterruptMode::Callback(|mut cx| {
                let s = cx.data_mut();
                *s += 1;
                Ok(UpdateDeadline::Yield(1))
            }),
            |_| {},
        )
        .await?
    );

    Ok(())
}

#[wasmtime_test]
async fn epoch_callback_yield_custom(config: &mut Config) -> Result<()> {
    assert_eq!(
        Some((1, 1)),
        run_and_count_yields_or_trap(
            config,
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")
                    call $bump  ;; bump epoch
                    call $subfunc) ;; call func; will notice new epoch and yield
                (func $subfunc))
            ",
            1,
            InterruptMode::Callback(|mut cx| {
                let s = cx.data_mut();
                *s += 1;
                let fut = Box::pin(tokio::task::yield_now());
                Ok(UpdateDeadline::YieldCustom(1, fut))
            }),
            |_| {},
        )
        .await?
    );
    Ok(())
}

#[wasmtime_test]
async fn epoch_callback_trap(config: &mut Config) -> Result<()> {
    assert_eq!(
        None,
        run_and_count_yields_or_trap(
            config,
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")
                    call $bump  ;; bump epoch
                    call $subfunc) ;; call func; will notice new epoch and yield
                (func $subfunc))
            ",
            1,
            InterruptMode::Callback(|_| Err(format_err!("Failing in callback"))),
            |_| {},
        )
        .await?
    );
    Ok(())
}

#[wasmtime_test]
async fn drop_future_on_epoch_yield(config: &mut Config) -> Result<()> {
    let wasm = "
    (module
      (import \"\" \"bump_epoch\" (func $bump))
      (import \"\" \"im_alive\" (func $im_alive))
      (import \"\" \"oops\" (func $oops))
      (func (export \"run\")
        (call $im_alive)
        (call $bump)
        (call $subfunc)  ;; subfunc entry to do epoch check
        (call $oops))
      (func $subfunc))
    ";

    let engine = build_engine(config)?;
    let mut linker = make_env::<()>(&engine);

    // Create a few helpers for the Wasm to call.
    let alive_flag = Arc::new(AtomicBool::new(false));
    let alive_flag_clone = alive_flag.clone();
    linker
        .func_new(
            "",
            "oops",
            FuncType::new(&engine, None, None),
            move |_caller, _params, _results| {
                panic!("Should not have reached this point!");
            },
        )
        .unwrap();
    linker
        .func_new(
            "",
            "im_alive",
            FuncType::new(&engine, None, None),
            move |_caller, _params, _results| {
                alive_flag_clone.store(true, Ordering::Release);
                Ok(())
            },
        )
        .unwrap();

    let module = Module::new(&engine, wasm).unwrap();
    let mut store = Store::new(&engine, ());

    store.set_epoch_deadline(1);
    store.epoch_deadline_async_yield_and_update(1);

    let instance = linker.instantiate_async(&mut store, &module).await.unwrap();
    let f = instance.get_func(&mut store, "run").unwrap();
    let _ = PollOnce::new(Box::pin(f.call_async(&mut store, &[], &mut []))).await;

    assert_eq!(true, alive_flag.load(Ordering::Acquire));
    Ok(())
}

#[test]
fn memory_grow_in_epoch_callback() -> Result<()> {
    let mut config = Config::new();
    config.epoch_interruption(true);
    config.memory_reservation(0);
    config.memory_reservation_for_growth(0);
    config.memory_guard_size(0);
    config.memory_may_move(true);
    config.memory_init_cow(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
              (memory (export "mem") 1)
              (func (export "go")
                (memory.fill (i32.const 0) (i32.const 0x41) (i32.const 65536))))
        "#,
    )?;

    let mut store: Store<Option<Memory>> = Store::new(&engine, None);
    store.set_epoch_deadline(1);
    store.epoch_deadline_callback(move |mut cx| {
        if let Some(mem) = *cx.data() {
            mem.grow(&mut cx, 5)?;
        }
        Ok(UpdateDeadline::Continue(0))
    });

    let instance = Instance::new(&mut store, &module, &[])?;
    let mem = instance.get_memory(&mut store, "mem").unwrap();
    *store.data_mut() = Some(mem);
    engine.increment_epoch();

    instance
        .get_typed_func::<(), ()>(&mut store, "go")?
        .call(&mut store, ())?;

    let data = mem.data(&store);
    assert_eq!(data[0], 0x41);
    assert_eq!(data[65535], 0x41);
    Ok(())
}

#[test]
fn table_grow_in_epoch_callback() -> Result<()> {
    let mut config = Config::new();
    config.epoch_interruption(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
              (table $t (export "t") 1 funcref)
              (func (export "go")
                (table.fill $t (i32.const 0) (ref.null func) (i32.const 1))))
        "#,
    )?;

    let mut store: Store<Option<Table>> = Store::new(&engine, None);
    store.set_epoch_deadline(1);
    store.epoch_deadline_callback(move |mut cx| {
        if let Some(t) = *cx.data() {
            t.grow(&mut cx, 5, Ref::Func(None))?;
        }
        Ok(UpdateDeadline::Continue(0))
    });

    let instance = Instance::new(&mut store, &module, &[])?;
    let t = instance.get_table(&mut store, "t").unwrap();
    *store.data_mut() = Some(t);
    engine.increment_epoch();

    instance
        .get_typed_func::<(), ()>(&mut store, "go")?
        .call(&mut store, ())?;
    Ok(())
}

#[test]
fn gc_during_epoch_callback() -> Result<()> {
    let mut config = Config::new();
    config.epoch_interruption(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
              (type $box (struct (field i32)))
              (type $arr (array (mut (ref null $box))))
              (global $sink (mut (ref null $arr)) (ref.null $arr))
              (func $mk (param $n i32) (result (ref $arr))
                (array.new_default $arr (local.get $n)))
              (func (export "run") (param $n i32) (result i32)
                (local $i i32)
                (block $done
                    (loop $l
                      (br_if $done (i32.ge_u (local.get $i) (i32.const 40)))
                      (array.fill $arr (call $mk (local.get $n)) (i32.const 0)
                                  (struct.new $box (i32.const 7)) (local.get $n))
                      (global.set $sink (call $mk (i32.const 8)))
                      (local.set $i (i32.add (local.get $i) (i32.const 1)))
                      (br $l)
                    )
                )
                (i32.mul (local.get $n) (i32.const 7))))
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    store.set_epoch_deadline(1);
    store.epoch_deadline_callback(|mut caller| {
        caller.gc(None)?;
        Ok(UpdateDeadline::Continue(0))
    });
    engine.increment_epoch();

    let instance = Instance::new(&mut store, &module, &[])?;
    let run = instance.get_typed_func::<u32, u32>(&mut store, "run")?;
    let n = 200;
    for i in 0..5 {
        let got = run.call(&mut store, n)?;
        assert_eq!(got, 7 * n, "iteration {i} read back {got}");
    }
    Ok(())
}
