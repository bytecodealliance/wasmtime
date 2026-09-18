//! Callee-pop returns, synchronous frame walking, and recovery after traps.
use std::panic::{AssertUnwindSafe, catch_unwind};
use wasmtime::*;

use wasmtime_test_macros::wasmtime_test;

#[wasmtime_test(strategies(only(Winch)))]
#[cfg_attr(miri, ignore)]
fn callee_pop_epilogue_boundary(config: &mut Config) -> Result<()> {
    if !cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
        return Ok(());
    }
    config.wasm_tail_call(false);
    let engine = Engine::new(config)?;
    // On x86 these land immediately below and above the compact-frame cutoff.
    // Reuse the disassembly fixture to check both encoding and execution.
    let module = Module::new(
        &engine,
        include_str!("../disas/winch/x64/callee_pop/epilogues.wat"),
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let no_args = instance.get_typed_func::<(), i64>(&mut store, "no_stack_args")?;
    for name in ["compact", "fallback"] {
        let f = instance
            .get_typed_func::<(i64, i64, i64, i64, i64, i64, i64, i64), i64>(&mut store, name)?;
        for seed in [-17, 0, 42] {
            for _ in 0..10 {
                assert_eq!(
                    f.call(&mut store, (seed, 2, 3, 4, 5, 6, 7, seed + 8))?,
                    2 * seed + 8
                );
                assert_eq!(no_args.call(&mut store, ())?, 42);
            }
        }
    }
    Ok(())
}

// Keep an operand live across each call and consume every argument and result.
// This exercises combined padding/spill cleanup as well as the stack-result
// path, where result movement must precede the final cleanup.
#[wasmtime_test(strategies(only(Winch)))]
#[cfg_attr(miri, ignore)]
fn callee_pop_cleanup_preserves_live_values(config: &mut Config) -> Result<()> {
    if !cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
        return Ok(());
    }
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

#[wasmtime_test(strategies(only(Winch)))]
#[cfg_attr(miri, ignore)]
fn callee_pop_returns_and_traps(config: &mut Config) -> Result<()> {
    if !cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
        return Ok(());
    }
    for fuel in [false, true] {
        let mut config = config.clone();
        config
            .strategy(Strategy::Winch)
            .wasm_tail_call(true)
            .consume_fuel(fuel);
        config.max_wasm_stack(64 * 1024);
        let engine = Engine::new(&config)?;
        // Exercise both sides of AArch64's signed load/store-offset range,
        // the previously failing 64/80-argument tails, and an entry-SP delta
        // that cannot be encoded by one add/sub immediate.
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
