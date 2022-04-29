use crate::async_functions::{CountPending, PollOnce};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wasmtime::*;

fn build_engine() -> Arc<Engine> {
    let mut config = Config::new();
    config.async_support(true);
    config.epoch_interruption(true);
    Arc::new(Engine::new(&config).unwrap())
}

fn make_env(engine: &Engine) -> Linker<()> {
    let mut linker = Linker::new(engine);
    let engine = engine.clone();

    linker
        .func_new(
            "",
            "bump_epoch",
            FuncType::new(None, None),
            move |_caller, _params, _results| {
                engine.increment_epoch();
                Ok(())
            },
        )
        .unwrap();

    linker
}

/// Run a test with the given wasm, giving an initial deadline of
/// `initial` ticks in the future, and either configuring the wasm to
/// yield and set a deadline `delta` ticks in the future if `delta` is
/// `Some(..)` or trapping if `delta` is `None`.
///
/// Returns `Some(yields)` if function completed normally, giving the
/// number of yields that occured, or `None` if a trap occurred.
async fn run_and_count_yields_or_trap<F: Fn(Arc<Engine>)>(
    wasm: &str,
    initial: u64,
    delta: Option<u64>,
    setup_func: F,
) -> Option<usize> {
    let engine = build_engine();
    let linker = make_env(&engine);
    let module = Module::new(&engine, wasm).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_epoch_deadline(initial);
    match delta {
        Some(delta) => {
            store.epoch_deadline_async_yield_and_update(delta);
        }
        None => {
            store.epoch_deadline_trap();
        }
    }

    let engine_clone = engine.clone();
    setup_func(engine_clone);

    let instance = linker.instantiate_async(&mut store, &module).await.unwrap();
    let f = instance.get_func(&mut store, "run").unwrap();
    let (result, yields) =
        CountPending::new(Box::pin(f.call_async(&mut store, &[], &mut []))).await;
    return result.ok().map(|_| yields);
}

#[tokio::test]
async fn epoch_yield_at_func_entry() {
    // Should yield at start of call to func $subfunc.
    assert_eq!(
        Some(1),
        run_and_count_yields_or_trap(
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")
                    call $bump  ;; bump epoch
                    call $subfunc) ;; call func; will notice new epoch and yield
                (func $subfunc))
            ",
            1,
            Some(1),
            |_| {},
        )
        .await
    );
}

#[tokio::test]
async fn epoch_yield_at_loop_header() {
    // Should yield at top of loop, once per five iters.
    assert_eq!(
        Some(2),
        run_and_count_yields_or_trap(
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
            Some(5),
            |_| {},
        )
        .await
    );
}

#[tokio::test]
async fn epoch_yield_immediate() {
    // We should see one yield immediately when the initial deadline
    // is zero.
    assert_eq!(
        Some(1),
        run_and_count_yields_or_trap(
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")))
            ",
            0,
            Some(1),
            |_| {},
        )
        .await
    );
}

#[tokio::test]
async fn epoch_yield_only_once() {
    // We should yield from the subfunction, and then when we return
    // to the outer function and hit another loop header, we should
    // not yield again (the double-check block will reload the correct
    // epoch).
    assert_eq!(
        Some(1),
        run_and_count_yields_or_trap(
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
            Some(1),
            |_| {},
        )
        .await
    );
}

#[tokio::test]
async fn epoch_interrupt_infinite_loop() {
    assert_eq!(
        None,
        run_and_count_yields_or_trap(
            "
            (module
                (import \"\" \"bump_epoch\" (func $bump))
                (func (export \"run\")
                  (loop $l
                    (br $l))))
            ",
            1,
            None,
            |engine| {
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    engine.increment_epoch();
                });
            },
        )
        .await
    );
}

#[tokio::test]
async fn epoch_interrupt_function_entries() {
    assert_eq!(
        None,
        run_and_count_yields_or_trap(
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
            None,
            |engine| {
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    engine.increment_epoch();
                });
            },
        )
        .await
    );
}

#[tokio::test]
async fn drop_future_on_epoch_yield() {
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

    let engine = build_engine();
    let mut linker = make_env(&engine);

    // Create a few helpers for the Wasm to call.
    let alive_flag = Arc::new(AtomicBool::new(false));
    let alive_flag_clone = alive_flag.clone();
    linker
        .func_new(
            "",
            "oops",
            FuncType::new(None, None),
            move |_caller, _params, _results| {
                panic!("Should not have reached this point!");
            },
        )
        .unwrap();
    linker
        .func_new(
            "",
            "im_alive",
            FuncType::new(None, None),
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
    PollOnce::new(Box::pin(f.call_async(&mut store, &[], &mut []))).await;

    assert_eq!(true, alive_flag.load(Ordering::Acquire));
}
