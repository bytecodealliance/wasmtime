//! Tail-call returns, synchronous frame walking, and recovery after traps.
use std::panic::{AssertUnwindSafe, catch_unwind};
use wasmtime::*;

use wasmtime_test_macros::wasmtime_test;

#[wasmtime_test(wasm_features(tail_call))]
#[cfg_attr(miri, ignore)]
fn tail_calls_to_host_functions(config: &mut Config) -> Result<()> {
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"(module
            (type $no-args (func (result i32)))
            (type $with-args (func (param i32 i32 i32 i32 i32) (result i32)))
            (import "" "no-args" (func $no-args (type $no-args)))
            (import "" "with-args" (func $with-args (type $with-args)))
            (table funcref (elem $no-args $with-args))
            (func (export "direct-no-args") (result i32)
                return_call $no-args)
            (func (export "indirect-no-args") (result i32)
                i32.const 0
                return_call_indirect (type $no-args))
            (func (export "direct-with-args") (param i32) (result i32)
                local.get 0
                i32.const 2
                i32.const 3
                i32.const 4
                i32.const 5
                return_call $with-args)
            (func (export "indirect-with-args") (param i32) (result i32)
                local.get 0
                i32.const 2
                i32.const 3
                i32.const 4
                i32.const 5
                i32.const 1
                return_call_indirect (type $with-args)))"#,
    )?;
    let mut store = Store::new(&engine, ());
    let no_args = Func::wrap(&mut store, || 42i32);
    let with_args = Func::wrap(&mut store, |first: i32, _: i32, _: i32, _: i32, _: i32| {
        first
    });
    let instance = Instance::new(&mut store, &module, &[no_args.into(), with_args.into()])?;
    for name in ["direct-no-args", "indirect-no-args"] {
        let run = instance.get_typed_func::<(), i32>(&mut store, name)?;
        assert_eq!(run.call(&mut store, ())?, 42);
    }
    for name in ["direct-with-args", "indirect-with-args"] {
        let run = instance.get_typed_func::<i32, i32>(&mut store, name)?;
        assert_eq!(run.call(&mut store, 84)?, 84);
    }
    Ok(())
}

#[wasmtime_test(wasm_features(tail_call))]
#[cfg_attr(miri, ignore)]
fn tail_calls_skip_unreachable_loops(config: &mut Config) -> Result<()> {
    for (fuel, epoch) in [(true, false), (false, true), (true, true)] {
        let mut config = config.clone();
        config.consume_fuel(fuel).epoch_interruption(epoch);
        let engine = Engine::new(&config)?;
        for call in [
            "return_call $leaf",
            "i32.const 0 return_call_indirect (type $t)",
        ] {
            let module = Module::new(
                &engine,
                format!(
                    r#"(module
                        (type $t (func (result i32)))
                        (import "" "interrupt" (func $interrupt))
                        (func $leaf (type $t) i32.const 42)
                        (table funcref (elem $leaf))
                        ;; Tail recursion followed by an unreachable loop.
                        (func $recursive return_call $recursive (loop))
                        (func (export "run") (param i32) (result i32)
                            local.get 0
                            if
                                {call}
                                (loop (loop))
                            end
                            ;; The join restores reachability and the frame.
                            (loop)
                            i32.const 7)
                        (func (export "interrupt") (param i32) (result i32)
                            local.get 0
                            if
                                {call}
                                (loop)
                            end
                            call $interrupt
                            ;; This reachable loop must still check for interruption.
                            (loop)
                            i32.const 7))"#
                ),
            )?;
            let mut store = Store::new(&engine, ());
            if fuel {
                store.set_fuel(1_000_000)?;
            }
            if epoch {
                store.set_epoch_deadline(1);
            }
            let interrupt = Func::wrap(
                &mut store,
                move |mut caller: Caller<'_, ()>| -> Result<()> {
                    if fuel {
                        caller.set_fuel(0)?;
                    } else {
                        caller.engine().increment_epoch();
                    }
                    Ok(())
                },
            );
            let instance = Instance::new(&mut store, &module, &[interrupt.into()])?;
            let run = instance.get_typed_func::<i32, i32>(&mut store, "run")?;
            assert_eq!(run.call(&mut store, 1)?, 42);
            assert_eq!(run.call(&mut store, 0)?, 7);
            let interrupt = instance.get_typed_func::<i32, i32>(&mut store, "interrupt")?;
            assert_eq!(interrupt.call(&mut store, 1)?, 42);
            let trap = interrupt
                .call(&mut store, 0)
                .unwrap_err()
                .downcast::<Trap>()?;
            assert_eq!(
                trap,
                if fuel {
                    Trap::OutOfFuel
                } else {
                    Trap::Interrupt
                }
            );
        }
    }
    Ok(())
}

// Keep an operand live across each call and consume every argument and result.
// This exercises combined padding/spill cleanup as well as the stack-result
// path, where result movement must precede the final cleanup.
#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn cleanup_preserves_live_values(config: &mut Config) -> Result<()> {
    config.wasm_tail_call(false);
    let engine = Engine::new(config)?;
    for n in [0, 1, 5, 10, 32, 64, 80] {
        for multi in [false, true] {
            let params = " i64".repeat(n);
            let sum = (0..n)
                .map(|i| format!("local.get {i} i64.add "))
                .collect::<String>();
            let args = (1..=n)
                .map(|i| format!("local.get 0 i64.const {i} i64.add "))
                .collect::<String>();
            let results = if multi { "i64 i64 f64" } else { "i64" };
            let extra = if multi {
                "i64.const 123 f64.const 3.5"
            } else {
                ""
            };
            let consume = if multi {
                "i64.trunc_f64_s i64.add i64.add"
            } else {
                ""
            };
            let mut store = Store::new(&engine, ());
            let provider = Module::new(
                &engine,
                format!(
                    "(module (func (export \"leaf\") (param {params}) (result {results})
                    i64.const 0 {sum} {extra}))"
                ),
            )?;
            let provider = Instance::new(&mut store, &provider, &[])?;
            let imported = provider.get_func(&mut store, "leaf").unwrap();
            let module = Module::new(
                &engine,
                format!(
                    r#"(module
                (type $t (func (param {params}) (result {results})))
                (import "p" "leaf" (func $imported (type $t)))
                (func $local (type $t) i64.const 0 {sum} {extra})
                (table funcref (elem $local $imported))
                (func (export "local") (param i64) (result i64)
                    local.get 0 {args} call $local {consume} i64.add)
                (func (export "imported") (param i64) (result i64)
                    local.get 0 {args} call $imported {consume} i64.add)
                (func (export "indirect") (param i64 i32) (result i64)
                    local.get 0 {args} local.get 1 call_indirect (type $t)
                    {consume} i64.add))"#
                ),
            )?;
            let instance = Instance::new(&mut store, &module, &[imported.into()])?;
            let local = instance.get_typed_func::<i64, i64>(&mut store, "local")?;
            let imported = instance.get_typed_func::<i64, i64>(&mut store, "imported")?;
            let indirect = instance.get_typed_func::<(i64, i32), i64>(&mut store, "indirect")?;
            for seed in [-17, 0, 42] {
                let n = n as i64;
                let expected = (n + 1) * seed + n * (n + 1) / 2 + if multi { 126 } else { 0 };
                for _ in 0..3 {
                    assert_eq!(local.call(&mut store, seed)?, expected);
                    assert_eq!(imported.call(&mut store, seed)?, expected);
                    assert_eq!(indirect.call(&mut store, (seed, 0))?, expected);
                    assert_eq!(indirect.call(&mut store, (seed, 1))?, expected);
                }
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct State {
    mode: u32,
    captures: Vec<Vec<String>>,
}

fn names(bt: &WasmBacktrace) -> Vec<String> {
    bt.frames()
        .iter()
        .map(|f| f.func_name().unwrap_or("?").to_owned())
        .collect()
}

#[wasmtime_test(wasm_features(tail_call))]
#[cfg_attr(miri, ignore)]
fn returns_and_traps(config: &mut Config) -> Result<()> {
    for fuel in [false, true] {
        let mut config = config.clone();
        config.consume_fuel(fuel);
        config.max_wasm_stack(64 * 1024);
        let engine = Engine::new(&config)?;
        // Cover register and stack arguments, AArch64's signed load/store-offset
        // boundary, and entry-SP deltas beyond a single add/sub immediate.
        for n in [0, 1, 5, 10, 32, 38, 39, 40, 64, 80, 528] {
            let params = " i64".repeat(n);
            let args = (1..=n)
                .map(|i| format!("i64.const {i} "))
                .collect::<String>();
            let gets = (0..n)
                .map(|i| format!("local.get {i} "))
                .collect::<String>();
            let sum = (0..n)
                .map(|i| format!("local.get {i} i64.add "))
                .collect::<String>();
            let provider = format!(
                r#"(module
                (import "h" "observe" (func $observe))
                (func $reenter (export "reenter") (param {params}) (result i64)
                    i64.const 0 {sum})
                (func $leaf (export "leaf") (param {params}) (result i64 i64 f64)
                    {gets} call $reenter drop
                    call $observe
                    i64.const 0 {sum} i64.const 123 f64.const 3.5))"#
            );
            for indirect in [false, true] {
                for tail in [false, true] {
                    let mut store = Store::new(&engine, State::default());
                    if fuel {
                        store.set_fuel(1_000_000_000)?;
                    }
                    let observe = Func::wrap(
                        &mut store,
                        move |mut caller: Caller<'_, State>| -> Result<()> {
                            let bt = names(&WasmBacktrace::force_capture(&mut caller));
                            caller.data_mut().captures.push(bt);
                            match caller.data().mode {
                                1 => bail!("expected host error"),
                                2 => panic!("expected host panic"),
                                3 => {
                                    let f =
                                        caller.get_export("reenter").unwrap().into_func().unwrap();
                                    let input =
                                        (1..=n).map(|i| Val::I64(i as i64)).collect::<Vec<_>>();
                                    let mut result = [Val::I64(0)];
                                    f.call(&mut caller, &input, &mut result)?;
                                    assert_eq!(result[0].i64(), Some((n * (n + 1) / 2) as i64));
                                    let bt = names(&WasmBacktrace::force_capture(&mut caller));
                                    caller.data_mut().captures.push(bt);
                                }
                                _ => {}
                            }
                            Ok(())
                        },
                    );
                    let provider = Instance::new(
                        &mut store,
                        &Module::new(&engine, &provider)?,
                        &[observe.into()],
                    )?;
                    let leaf = provider.get_func(&mut store, "leaf").unwrap();
                    let op = if tail { "return_call" } else { "call" };
                    let leaf_call = if indirect {
                        format!("i32.const 0 {op}_indirect (type $t)")
                    } else {
                        format!("{op} $leaf")
                    };
                    let source = format!(
                        r#"(module
                        (type $t (func (param {params}) (result i64 i64 f64)))
                        (import "p" "leaf" (func $leaf (type $t)))
                        (table 1 funcref) (elem (i32.const 0) $leaf)
                        (func $thin (result i64 i64 f64) {args} {leaf_call})
                        (func $wide (param {params}) (result i64 i64 f64) {op} $thin)
                        (func $run (export "run") (result i64 i64 f64) {args} call $wide)
                        (func $after (export "after") call $run drop drop drop unreachable))"#
                    );
                    let module = Module::new(&engine, &source)?;
                    let instance = Instance::new(&mut store, &module, &[leaf.into()])?;
                    let run = instance.get_typed_func::<(), (i64, i64, f64)>(&mut store, "run")?;
                    let after = instance.get_typed_func::<(), ()>(&mut store, "after")?;
                    let expected_frames = if tail {
                        vec!["leaf", "run"]
                    } else {
                        vec!["leaf", "thin", "wide", "run"]
                    };
                    for _ in 0..2 {
                        for mode in [0, 1, 2, 3] {
                            store.data_mut().mode = mode;
                            store.data_mut().captures.clear();
                            match mode {
                                1 => {
                                    let err = run.call(&mut store, ()).unwrap_err();
                                    assert!(format!("{err:#}").contains("expected host error"));
                                    assert_eq!(
                                        names(err.downcast_ref::<WasmBacktrace>().unwrap()),
                                        expected_frames
                                    );
                                }
                                2 => {
                                    let panic =
                                        catch_unwind(AssertUnwindSafe(|| run.call(&mut store, ())))
                                            .unwrap_err();
                                    assert_eq!(
                                        panic.downcast_ref::<&str>(),
                                        Some(&"expected host panic")
                                    );
                                }
                                _ => assert_eq!(
                                    run.call(&mut store, ())?,
                                    ((n * (n + 1) / 2) as i64, 123, 3.5)
                                ),
                            }
                            assert_eq!(store.data().captures.len(), if mode == 3 { 2 } else { 1 });
                            for frames in &store.data().captures {
                                assert_eq!(*frames, expected_frames);
                            }
                        }
                        store.data_mut().mode = 0;
                        let err = after.call(&mut store, ()).unwrap_err();
                        assert_eq!(
                            err.downcast_ref::<Trap>(),
                            Some(&Trap::UnreachableCodeReached)
                        );
                        assert_eq!(
                            names(err.downcast_ref::<WasmBacktrace>().unwrap()),
                            ["after"]
                        );
                    }
                }
            }
        }
        // Ordinary recursion must trap; equivalent tail recursion must not grow
        // the stack. A successful subsequent call checks trap-state restoration.
        let args = " i64.const 7".repeat(12);
        let params = " i64".repeat(12);
        let gets = (1..=12)
            .map(|i| format!("local.get {i} "))
            .collect::<String>();
        let source = format!(
            r#"(module
            (func $ordinary (param i32 {params}) (result i32)
                local.get 0 i32.eqz if (result i32) i32.const 42 else
                    local.get 0 i32.const 1 i32.sub {gets} call $ordinary
                end)
            (func $tail (param i32 {params}) (result i32)
                local.get 0 i32.eqz if (result i32) i32.const 42 else
                    local.get 0 i32.const 1 i32.sub {gets} return_call $tail
                end)
            (func (export "ordinary") (param i32) (result i32) local.get 0 {args} call $ordinary)
            (func (export "tail") (param i32) (result i32) local.get 0 {args} call $tail))"#
        );
        let mut store = Store::new(&engine, ());
        if fuel {
            store.set_fuel(1_000_000_000)?;
        }
        let instance = Instance::new(&mut store, &Module::new(&engine, source)?, &[])?;
        let ordinary = instance.get_typed_func::<i32, i32>(&mut store, "ordinary")?;
        let tail = instance.get_typed_func::<i32, i32>(&mut store, "tail")?;
        for _ in 0..10 {
            let err = ordinary.call(&mut store, 100_000).unwrap_err();
            assert_eq!(err.downcast_ref::<Trap>(), Some(&Trap::StackOverflow));
            assert!(
                !err.downcast_ref::<WasmBacktrace>()
                    .unwrap()
                    .frames()
                    .is_empty()
            );
            assert_eq!(tail.call(&mut store, 100_000)?, 42);
            assert_eq!(ordinary.call(&mut store, 2)?, 42);
        }
    }
    Ok(())
}
