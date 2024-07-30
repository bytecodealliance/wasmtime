#![cfg(not(miri))]

use anyhow::bail;
use std::panic::{self, AssertUnwindSafe};
use std::process::Command;
use std::sync::{Arc, Mutex};
use wasmtime::*;

#[test]
fn test_trap_return() -> Result<()> {
    let mut store = Store::<()>::default();
    let wat = r#"
        (module
        (func $hello (import "" "hello"))
        (func (export "run") (call $hello))
        )
    "#;

    let module = Module::new(store.engine(), wat)?;
    let hello_type = FuncType::new(store.engine(), None, None);
    let hello_func = Func::new(&mut store, hello_type, |_, _, _| bail!("test 123"));

    let instance = Instance::new(&mut store, &module, &[hello_func.into()])?;
    let run_func = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    let e = run_func.call(&mut store, ()).unwrap_err();
    assert!(format!("{e:?}").contains("test 123"));

    assert!(
        e.downcast_ref::<WasmBacktrace>().is_some(),
        "error should contain a WasmBacktrace"
    );

    Ok(())
}

#[test]
fn test_anyhow_error_return() -> Result<()> {
    let mut store = Store::<()>::default();
    let wat = r#"
        (module
        (func $hello (import "" "hello"))
        (func (export "run") (call $hello))
        )
    "#;

    let module = Module::new(store.engine(), wat)?;
    let hello_type = FuncType::new(store.engine(), None, None);
    let hello_func = Func::new(&mut store, hello_type, |_, _, _| {
        Err(anyhow::Error::msg("test 1234"))
    });

    let instance = Instance::new(&mut store, &module, &[hello_func.into()])?;
    let run_func = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    let e = run_func.call(&mut store, ()).unwrap_err();
    assert!(!e.to_string().contains("test 1234"));
    assert!(format!("{e:?}").contains("Caused by:\n    test 1234"));

    assert!(e.downcast_ref::<Trap>().is_none());
    assert!(e.downcast_ref::<WasmBacktrace>().is_some());

    Ok(())
}

#[test]
fn test_trap_return_downcast() -> Result<()> {
    let mut store = Store::<()>::default();
    let wat = r#"
        (module
        (func $hello (import "" "hello"))
        (func (export "run") (call $hello))
        )
    "#;

    #[derive(Debug)]
    struct MyTrap;
    impl std::fmt::Display for MyTrap {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "my trap")
        }
    }
    impl std::error::Error for MyTrap {}

    let module = Module::new(store.engine(), wat)?;
    let hello_type = FuncType::new(store.engine(), None, None);
    let hello_func = Func::new(&mut store, hello_type, |_, _, _| {
        Err(anyhow::Error::from(MyTrap))
    });

    let instance = Instance::new(&mut store, &module, &[hello_func.into()])?;
    let run_func = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    let e = run_func
        .call(&mut store, ())
        .err()
        .expect("error calling function");
    let dbg = format!("{e:?}");
    println!("{dbg}");

    assert!(!e.to_string().contains("my trap"));
    assert!(dbg.contains("Caused by:\n    my trap"));

    e.downcast_ref::<MyTrap>()
        .expect("error downcasts to MyTrap");
    let bt = e
        .downcast_ref::<WasmBacktrace>()
        .expect("error downcasts to WasmBacktrace");
    assert_eq!(bt.frames().len(), 1);
    println!("{bt:?}");

    Ok(())
}

#[test]
fn test_trap_trace() -> Result<()> {
    let mut store = Store::<()>::default();
    let wat = r#"
        (module $hello_mod
            (func (export "run") (call $hello))
            (func $hello (unreachable))
        )
    "#;

    let module = Module::new(store.engine(), wat)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let run_func = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    let e = run_func.call(&mut store, ()).unwrap_err();

    let trace = e.downcast_ref::<WasmBacktrace>().unwrap().frames();
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0].module().name().unwrap(), "hello_mod");
    assert_eq!(trace[0].func_index(), 1);
    assert_eq!(trace[0].func_name(), Some("hello"));
    assert_eq!(trace[0].func_offset(), Some(1));
    assert_eq!(trace[0].module_offset(), Some(0x26));
    assert_eq!(trace[1].module().name().unwrap(), "hello_mod");
    assert_eq!(trace[1].func_index(), 0);
    assert_eq!(trace[1].func_name(), None);
    assert_eq!(trace[1].func_offset(), Some(1));
    assert_eq!(trace[1].module_offset(), Some(0x21));
    assert_eq!(e.downcast::<Trap>()?, Trap::UnreachableCodeReached);

    Ok(())
}

#[test]
fn test_trap_through_host() -> Result<()> {
    let wat = r#"
        (module $hello_mod
            (import "" "" (func $host_func_a))
            (import "" "" (func $host_func_b))
            (func $a (export "a")
                call $host_func_a
            )
            (func $b (export "b")
                call $host_func_b
            )
            (func $c (export "c")
                unreachable
            )
        )
    "#;

    let engine = Engine::default();
    let module = Module::new(&engine, wat)?;
    let mut store = Store::<()>::new(&engine, ());

    let host_func_a = Func::new(
        &mut store,
        FuncType::new(&engine, vec![], vec![]),
        |mut caller, _args, _results| {
            caller
                .get_export("b")
                .unwrap()
                .into_func()
                .unwrap()
                .call(caller, &[], &mut [])?;
            Ok(())
        },
    );
    let host_func_b = Func::new(
        &mut store,
        FuncType::new(&engine, vec![], vec![]),
        |mut caller, _args, _results| {
            caller
                .get_export("c")
                .unwrap()
                .into_func()
                .unwrap()
                .call(caller, &[], &mut [])?;
            Ok(())
        },
    );

    let instance = Instance::new(
        &mut store,
        &module,
        &[host_func_a.into(), host_func_b.into()],
    )?;
    let a = instance.get_typed_func::<(), ()>(&mut store, "a")?;
    let err = a.call(&mut store, ()).unwrap_err();
    let trace = err.downcast_ref::<WasmBacktrace>().unwrap().frames();
    assert_eq!(trace.len(), 3);
    assert_eq!(trace[0].func_name(), Some("c"));
    assert_eq!(trace[1].func_name(), Some("b"));
    assert_eq!(trace[2].func_name(), Some("a"));
    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_trap_backtrace_disabled() -> Result<()> {
    let mut config = Config::default();
    config.wasm_backtrace(false);
    let engine = Engine::new(&config).unwrap();
    let mut store = Store::<()>::new(&engine, ());
    let wat = r#"
        (module $hello_mod
            (func (export "run") (call $hello))
            (func $hello (unreachable))
        )
    "#;

    let module = Module::new(store.engine(), wat)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let run_func = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    let e = run_func.call(&mut store, ()).unwrap_err();
    assert!(e.downcast_ref::<WasmBacktrace>().is_none());
    Ok(())
}

#[test]
fn test_trap_trace_cb() -> Result<()> {
    let mut store = Store::<()>::default();
    let wat = r#"
        (module $hello_mod
            (import "" "throw" (func $throw))
            (func (export "run") (call $hello))
            (func $hello (call $throw))
        )
    "#;

    let fn_type = FuncType::new(store.engine(), None, None);
    let fn_func = Func::new(&mut store, fn_type, |_, _, _| bail!("cb throw"));

    let module = Module::new(store.engine(), wat)?;
    let instance = Instance::new(&mut store, &module, &[fn_func.into()])?;
    let run_func = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    let e = run_func.call(&mut store, ()).unwrap_err();

    let trace = e.downcast_ref::<WasmBacktrace>().unwrap().frames();
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0].module().name().unwrap(), "hello_mod");
    assert_eq!(trace[0].func_index(), 2);
    assert_eq!(trace[1].module().name().unwrap(), "hello_mod");
    assert_eq!(trace[1].func_index(), 1);
    assert!(format!("{e:?}").contains("cb throw"));

    Ok(())
}

#[test]
fn test_trap_stack_overflow() -> Result<()> {
    let mut store = Store::<()>::default();
    let wat = r#"
        (module $rec_mod
            (func $run (export "run") (call $run))
        )
    "#;

    let module = Module::new(store.engine(), wat)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let run_func = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    let e = run_func.call(&mut store, ()).unwrap_err();

    let trace = e.downcast_ref::<WasmBacktrace>().unwrap().frames();
    assert!(trace.len() >= 32);
    for i in 0..trace.len() {
        assert_eq!(trace[i].module().name().unwrap(), "rec_mod");
        assert_eq!(trace[i].func_index(), 0);
        assert_eq!(trace[i].func_name(), Some("run"));
    }
    assert_eq!(e.downcast::<Trap>()?, Trap::StackOverflow);

    Ok(())
}

#[test]
fn trap_display_pretty() -> Result<()> {
    let mut store = Store::<()>::default();
    let wat = r#"
        (module $m
            (func $die unreachable)
            (func call $die)
            (func $foo call 1)
            (func (export "bar") call $foo)
        )
    "#;

    let module = Module::new(store.engine(), wat)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let run_func = instance.get_typed_func::<(), ()>(&mut store, "bar")?;

    let e = run_func.call(&mut store, ()).unwrap_err();
    let e = format!("{e:?}");
    assert!(e.contains(
        "\
error while executing at wasm backtrace:
    0:   0x23 - m!die
    1:   0x27 - m!<wasm function 1>
    2:   0x2c - m!foo
    3:   0x31 - m!<wasm function 3>

Caused by:
    wasm trap: wasm `unreachable` instruction executed\
"
    ));
    Ok(())
}

#[test]
fn trap_display_multi_module() -> Result<()> {
    let mut store = Store::<()>::default();
    let wat = r#"
        (module $a
            (func $die unreachable)
            (func call $die)
            (func $foo call 1)
            (func (export "bar") call $foo)
        )
    "#;

    let module = Module::new(store.engine(), wat)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let bar = instance.get_export(&mut store, "bar").unwrap();

    let wat = r#"
        (module $b
            (import "" "" (func $bar))
            (func $middle call $bar)
            (func (export "bar2") call $middle)
        )
    "#;
    let module = Module::new(store.engine(), wat)?;
    let instance = Instance::new(&mut store, &module, &[bar])?;
    let bar2 = instance.get_typed_func::<(), ()>(&mut store, "bar2")?;

    let e = bar2.call(&mut store, ()).unwrap_err();
    let e = format!("{e:?}");
    assert!(e.contains(
        "\
error while executing at wasm backtrace:
    0:   0x23 - a!die
    1:   0x27 - a!<wasm function 1>
    2:   0x2c - a!foo
    3:   0x31 - a!<wasm function 3>
    4:   0x29 - b!middle
    5:   0x2e - b!<wasm function 2>

Caused by:
    wasm trap: wasm `unreachable` instruction executed\
"
    ));
    Ok(())
}

#[test]
fn trap_start_function_import() -> Result<()> {
    let mut store = Store::<()>::default();
    let binary = wat::parse_str(
        r#"
            (module $a
                (import "" "" (func $foo))
                (start $foo)
            )
        "#,
    )?;

    let module = Module::new(store.engine(), &binary)?;
    let sig = FuncType::new(store.engine(), None, None);
    let func = Func::new(&mut store, sig, |_, _, _| bail!("user trap"));
    let err = Instance::new(&mut store, &module, &[func.into()]).unwrap_err();
    assert!(format!("{err:?}").contains("user trap"));
    Ok(())
}

#[test]
fn rust_panic_import() -> Result<()> {
    let mut store = Store::<()>::default();
    let binary = wat::parse_str(
        r#"
            (module $a
                (import "" "" (func $foo))
                (import "" "" (func $bar))
                (func (export "foo") call $foo)
                (func (export "bar") call $bar)
            )
        "#,
    )?;

    let module = Module::new(store.engine(), &binary)?;
    let sig = FuncType::new(store.engine(), None, None);
    let func = Func::new(&mut store, sig, |_, _, _| panic!("this is a panic"));
    let func2 = Func::wrap(&mut store, || -> () { panic!("this is another panic") });
    let instance = Instance::new(&mut store, &module, &[func.into(), func2.into()])?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "foo")?;
    let err =
        panic::catch_unwind(AssertUnwindSafe(|| drop(func.call(&mut store, ())))).unwrap_err();
    assert_eq!(err.downcast_ref::<&'static str>(), Some(&"this is a panic"));

    let func = instance.get_typed_func::<(), ()>(&mut store, "bar")?;
    let err = panic::catch_unwind(AssertUnwindSafe(|| {
        drop(func.call(&mut store, ()));
    }))
    .unwrap_err();
    assert_eq!(
        err.downcast_ref::<&'static str>(),
        Some(&"this is another panic")
    );
    Ok(())
}

// Test that we properly save/restore our trampolines' saved Wasm registers
// (used when capturing backtraces) before we resume panics.
#[test]
fn rust_catch_panic_import() -> Result<()> {
    let mut store = Store::<()>::default();

    let binary = wat::parse_str(
        r#"
            (module $a
                (import "" "panic" (func $panic))
                (import "" "catch panic" (func $catch_panic))
                (func (export "panic") call $panic)
                (func (export "run")
                  call $catch_panic
                  call $catch_panic
                  unreachable
                )
            )
        "#,
    )?;

    let module = Module::new(store.engine(), &binary)?;
    let num_panics = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let sig = FuncType::new(store.engine(), None, None);
    let panic = Func::new(&mut store, sig, {
        let num_panics = num_panics.clone();
        move |_, _, _| {
            num_panics.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            panic!("this is a panic");
        }
    });
    let catch_panic = Func::wrap(&mut store, |mut caller: Caller<'_, _>| {
        panic::catch_unwind(AssertUnwindSafe(|| {
            drop(
                caller
                    .get_export("panic")
                    .unwrap()
                    .into_func()
                    .unwrap()
                    .call(&mut caller, &[], &mut []),
            );
        }))
        .unwrap_err();
    });

    let instance = Instance::new(&mut store, &module, &[panic.into(), catch_panic.into()])?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    let trap = run.call(&mut store, ()).unwrap_err();
    let trace = trap.downcast_ref::<WasmBacktrace>().unwrap().frames();
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].func_index(), 3);
    assert_eq!(num_panics.load(std::sync::atomic::Ordering::SeqCst), 2);
    Ok(())
}

#[test]
fn rust_panic_start_function() -> Result<()> {
    let mut store = Store::<()>::default();
    let binary = wat::parse_str(
        r#"
            (module $a
                (import "" "" (func $foo))
                (start $foo)
            )
        "#,
    )?;

    let module = Module::new(store.engine(), &binary)?;
    let sig = FuncType::new(store.engine(), None, None);
    let func = Func::new(&mut store, sig, |_, _, _| panic!("this is a panic"));
    let err = panic::catch_unwind(AssertUnwindSafe(|| {
        drop(Instance::new(&mut store, &module, &[func.into()]));
    }))
    .unwrap_err();
    assert_eq!(err.downcast_ref::<&'static str>(), Some(&"this is a panic"));

    let func = Func::wrap(&mut store, || -> () { panic!("this is another panic") });
    let err = panic::catch_unwind(AssertUnwindSafe(|| {
        drop(Instance::new(&mut store, &module, &[func.into()]));
    }))
    .unwrap_err();
    assert_eq!(
        err.downcast_ref::<&'static str>(),
        Some(&"this is another panic")
    );
    Ok(())
}

#[test]
fn mismatched_arguments() -> Result<()> {
    let mut store = Store::<()>::default();
    let binary = wat::parse_str(
        r#"
            (module $a
                (func (export "foo") (param i32))
            )
        "#,
    )?;

    let module = Module::new(store.engine(), &binary)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_func(&mut store, "foo").unwrap();
    assert_eq!(
        func.call(&mut store, &[], &mut []).unwrap_err().to_string(),
        "expected 1 arguments, got 0"
    );
    let e = func.call(&mut store, &[Val::F32(0)], &mut []).unwrap_err();
    let e = format!("{e:?}");
    assert!(e.contains("argument type mismatch"));
    assert!(e.contains("expected i32, found f32"));
    assert_eq!(
        func.call(&mut store, &[Val::I32(0), Val::I32(1)], &mut [])
            .unwrap_err()
            .to_string(),
        "expected 1 arguments, got 2"
    );
    Ok(())
}

#[test]
fn call_signature_mismatch() -> Result<()> {
    let mut store = Store::<()>::default();
    let binary = wat::parse_str(
        r#"
            (module $a
                (func $foo
                    i32.const 0
                    call_indirect)
                (func $bar (param i32))
                (start $foo)

                (table 1 funcref)
                (elem (i32.const 0) 1)
            )
        "#,
    )?;

    let module = Module::new(store.engine(), &binary)?;
    let err = Instance::new(&mut store, &module, &[])
        .err()
        .unwrap()
        .downcast::<Trap>()
        .unwrap();
    assert!(err
        .to_string()
        .contains("wasm trap: indirect call type mismatch"));
    Ok(())
}

#[test]
fn start_trap_pretty() -> Result<()> {
    let mut store = Store::<()>::default();
    let wat = r#"
        (module $m
            (func $die unreachable)
            (func call $die)
            (func $foo call 1)
            (func $start call $foo)
            (start $start)
        )
    "#;

    let module = Module::new(store.engine(), wat)?;
    let e = match Instance::new(&mut store, &module, &[]) {
        Ok(_) => panic!("expected failure"),
        Err(e) => format!("{e:?}"),
    };

    assert!(e.contains(
        "\
error while executing at wasm backtrace:
    0:   0x1d - m!die
    1:   0x21 - m!<wasm function 1>
    2:   0x26 - m!foo
    3:   0x2b - m!start

Caused by:
    wasm trap: wasm `unreachable` instruction executed\
"
    ));
    Ok(())
}

#[test]
fn present_after_module_drop() -> Result<()> {
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), r#"(func (export "foo") unreachable)"#)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "foo")?;

    println!("asserting before we drop modules");
    assert_trap(func.call(&mut store, ()).unwrap_err());
    drop((instance, module));

    println!("asserting after drop");
    assert_trap(func.call(&mut store, ()).unwrap_err());
    return Ok(());

    fn assert_trap(t: Error) {
        println!("{t:?}");
        let trace = t.downcast_ref::<WasmBacktrace>().unwrap().frames();
        assert_eq!(trace.len(), 1);
        assert_eq!(trace[0].func_index(), 0);
    }
}

fn assert_trap_code(wat: &str, code: wasmtime::Trap) {
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), wat).unwrap();

    let err = match Instance::new(&mut store, &module, &[]) {
        Ok(_) => unreachable!(),
        Err(e) => e,
    };
    let trap = err.downcast_ref::<Trap>().unwrap();
    assert_eq!(*trap, code);

    let trace = err.downcast_ref::<WasmBacktrace>().unwrap().frames();
    assert!(trace.len() > 0);
    assert_eq!(trace[0].func_index(), 0);
    assert!(trace[0].func_offset().is_some());
}

#[test]
fn trap_codes() {
    assert_trap_code(
        r#"
            (module
              (memory 0)
              (func $start (drop (i32.load (i32.const 1000000))))
              (start $start)
            )
         "#,
        Trap::MemoryOutOfBounds,
    );

    assert_trap_code(
        r#"
            (module
              (memory 0)
              (func $start (drop (i32.load memory.size)))
              (start $start)
            )
         "#,
        Trap::MemoryOutOfBounds,
    );

    for (ty, min) in [("i32", i32::MIN as u32 as u64), ("i64", i64::MIN as u64)] {
        for op in ["rem", "div"] {
            for sign in ["u", "s"] {
                println!("testing {ty}.{op}_{sign}");
                assert_trap_code(
                    &format!(
                        r#"
                           (module
                             (func $div (param {ty} {ty}) (result {ty})
                               local.get 0
                               local.get 1
                               {ty}.{op}_{sign})
                             (func $start (drop (call $div ({ty}.const 1) ({ty}.const 0))))
                             (start $start)
                           )
                        "#
                    ),
                    Trap::IntegerDivisionByZero,
                );
            }
        }

        println!("testing {ty}.div_s INT_MIN/-1");
        assert_trap_code(
            &format!(
                r#"
                    (module
                     (func $div (param {ty} {ty}) (result {ty})
                      local.get 0
                      local.get 1
                      {ty}.div_s)
                     (func $start (drop (call $div ({ty}.const {min}) ({ty}.const -1))))
                     (start $start)
                    )
                "#
            ),
            Trap::IntegerOverflow,
        );
    }
}

fn rustc(src: &str) -> Vec<u8> {
    let td = tempfile::TempDir::new().unwrap();
    let output = td.path().join("foo.wasm");
    let input = td.path().join("input.rs");
    std::fs::write(&input, src).unwrap();
    let result = Command::new("rustc")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("--target")
        .arg("wasm32-wasip1")
        .arg("-g")
        .output()
        .unwrap();
    if result.status.success() {
        return std::fs::read(&output).unwrap();
    }
    panic!(
        "rustc failed: {}\n{}",
        result.status,
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn parse_dwarf_info() -> Result<()> {
    let wasm = rustc(
        "
            fn main() {
                panic!();
            }
        ",
    );
    let mut config = Config::new();
    config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, &wasm)?;
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t)?;
    let mut store = Store::new(&engine, wasmtime_wasi::WasiCtxBuilder::new().build_p1());
    linker.module(&mut store, "", &module)?;
    let run = linker.get_default(&mut store, "")?;
    let trap = run.call(&mut store, &[], &mut []).unwrap_err();

    let mut found = false;
    let frames = trap.downcast_ref::<WasmBacktrace>().unwrap().frames();
    for frame in frames {
        for symbol in frame.symbols() {
            if let Some(file) = symbol.file() {
                if file.ends_with("input.rs") {
                    found = true;
                    assert!(symbol.name().unwrap().contains("main"));
                    assert_eq!(symbol.line(), Some(3));
                }
            }
        }
    }
    assert!(found);
    Ok(())
}

#[test]
fn no_hint_even_with_dwarf_info() -> Result<()> {
    let mut config = Config::new();
    config.wasm_backtrace_details(WasmBacktraceDetails::Disable);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
            (module
                (@custom ".debug_info" (after last) "")
                (func $start
                    unreachable)
                (start $start)
            )
        "#,
    )?;
    let trap = Instance::new(&mut store, &module, &[]).unwrap_err();
    let trap = format!("{trap:?}");
    assert!(trap.contains(
        "\
error while executing at wasm backtrace:
    0:   0x1a - <unknown>!start

Caused by:
    wasm trap: wasm `unreachable` instruction executed\
"
    ));
    assert!(!trap.contains("WASM_BACKTRACE_DETAILS"));
    Ok(())
}

#[test]
fn hint_with_dwarf_info() -> Result<()> {
    // Skip this test if the env var is already configure, but in CI we're sure
    // to run tests without this env var configured.
    if std::env::var("WASMTIME_BACKTRACE_DETAILS").is_ok() {
        return Ok(());
    }
    let mut store = Store::<()>::default();
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (@custom ".debug_info" (after last) "")
                (func $start
                    unreachable)
                (start $start)
            )
        "#,
    )?;
    let trap = Instance::new(&mut store, &module, &[]).unwrap_err();
    let trap = format!("{trap:?}");
    assert!(trap.contains(
        "\
error while executing at wasm backtrace:
    0:   0x1a - <unknown>!start
note: using the `WASMTIME_BACKTRACE_DETAILS=1` environment variable may show more debugging information

Caused by:
    wasm trap: wasm `unreachable` instruction executed"
    ));
    Ok(())
}

#[test]
fn multithreaded_traps() -> Result<()> {
    // Compile and run unreachable on a thread, then moves over the whole store to another thread,
    // and make sure traps are still correctly caught after notifying the store of the move.
    let mut store = Store::<()>::default();
    let module = Module::new(
        store.engine(),
        r#"(module (func (export "run") unreachable))"#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;

    assert!(instance
        .get_typed_func::<(), ()>(&mut store, "run")?
        .call(&mut store, ())
        .is_err());

    let handle = std::thread::spawn(move || {
        assert!(instance
            .get_typed_func::<(), ()>(&mut store, "run")
            .unwrap()
            .call(&mut store, ())
            .is_err());
    });

    handle.join().expect("couldn't join thread");

    Ok(())
}

#[test]
fn traps_without_address_map() -> Result<()> {
    let mut config = Config::new();
    config.generate_address_map(false);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let wat = r#"
        (module $hello_mod
            (func (export "run") (call $hello))
            (func $hello (unreachable))
        )
    "#;

    let module = Module::new(store.engine(), wat)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let run_func = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    let e = run_func.call(&mut store, ()).unwrap_err();

    let trace = e.downcast_ref::<WasmBacktrace>().unwrap().frames();
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0].func_name(), Some("hello"));
    assert_eq!(trace[0].func_index(), 1);
    assert_eq!(trace[0].module_offset(), None);
    assert_eq!(trace[1].func_name(), None);
    assert_eq!(trace[1].func_index(), 0);
    assert_eq!(trace[1].module_offset(), None);
    Ok(())
}

#[test]
fn catch_trap_calling_across_stores() -> Result<()> {
    let _ = env_logger::try_init();

    let engine = Engine::default();

    let mut child_store = Store::new(&engine, ());
    let child_module = Module::new(
        child_store.engine(),
        r#"
            (module $child
              (func $trap (export "trap")
                unreachable
              )
            )
        "#,
    )?;
    let child_instance = Instance::new(&mut child_store, &child_module, &[])?;

    struct ParentCtx {
        child_store: Store<()>,
        child_instance: Instance,
    }

    let mut linker = Linker::new(&engine);
    linker.func_wrap(
        "host",
        "catch_child_trap",
        move |mut caller: Caller<'_, ParentCtx>| {
            let mut ctx = caller.as_context_mut();
            let data = ctx.data_mut();
            let func = data
                .child_instance
                .get_typed_func::<(), ()>(&mut data.child_store, "trap")
                .expect("trap function should be exported");

            let trap = func.call(&mut data.child_store, ()).unwrap_err();
            assert!(
                format!("{trap:?}").contains("unreachable"),
                "trap should contain 'unreachable', got: {trap:?}"
            );

            let trace = trap.downcast_ref::<WasmBacktrace>().unwrap().frames();

            assert_eq!(trace.len(), 1);
            assert_eq!(trace[0].func_name(), Some("trap"));
            // For now, we only get stack frames for Wasm in this store, not
            // across all stores.
            //
            // assert_eq!(trace[1].func_name(), Some("run"));

            Ok(())
        },
    )?;

    let mut store = Store::new(
        &engine,
        ParentCtx {
            child_store,
            child_instance,
        },
    );

    let parent_module = Module::new(
        store.engine(),
        r#"
            (module $parent
              (func $host.catch_child_trap (import "host" "catch_child_trap"))
              (func $run (export "run")
                call $host.catch_child_trap
              )
            )
        "#,
    )?;

    let parent_instance = linker.instantiate(&mut store, &parent_module)?;

    let func = parent_instance.get_typed_func::<(), ()>(&mut store, "run")?;
    func.call(store, ())?;

    Ok(())
}

#[tokio::test]
async fn async_then_sync_trap() -> Result<()> {
    // Test the trapping and capturing the stack with the following sequence of
    // calls:
    //
    // a[async] ---> b[host] ---> c[sync]

    drop(env_logger::try_init());

    let wat = r#"
        (module
            (import "" "b" (func $b))
            (func $a (export "a")
                call $b
            )
            (func $c (export "c")
                unreachable
            )
        )
    "#;

    let mut sync_store = Store::new(&Engine::default(), ());

    let sync_module = Module::new(sync_store.engine(), wat)?;

    let mut sync_linker = Linker::new(sync_store.engine());
    sync_linker.func_wrap("", "b", |_caller: Caller<_>| -> () { unreachable!() })?;

    let sync_instance = sync_linker.instantiate(&mut sync_store, &sync_module)?;

    struct AsyncCtx {
        sync_instance: Instance,
        sync_store: Store<()>,
    }

    let mut async_store = Store::new(
        &Engine::new(Config::new().async_support(true)).unwrap(),
        AsyncCtx {
            sync_instance,
            sync_store,
        },
    );

    let async_module = Module::new(async_store.engine(), wat)?;

    let mut async_linker = Linker::new(async_store.engine());
    async_linker.func_wrap("", "b", move |mut caller: Caller<AsyncCtx>| {
        log::info!("Called `b`...");
        let sync_instance = caller.data().sync_instance;
        let sync_store = &mut caller.data_mut().sync_store;

        log::info!("Calling `c`...");
        let c = sync_instance
            .get_typed_func::<(), ()>(&mut *sync_store, "c")
            .unwrap();
        c.call(sync_store, ())?;
        Ok(())
    })?;

    let async_instance = async_linker
        .instantiate_async(&mut async_store, &async_module)
        .await?;

    log::info!("Calling `a`...");
    let a = async_instance
        .get_typed_func::<(), ()>(&mut async_store, "a")
        .unwrap();
    let trap = a.call_async(&mut async_store, ()).await.unwrap_err();

    let trace = trap.downcast_ref::<WasmBacktrace>().unwrap().frames();
    // We don't support cross-store or cross-engine symbolication currently, so
    // the other frames are ignored.
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].func_name(), Some("c"));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_then_async_trap() -> Result<()> {
    // Test the trapping and capturing the stack with the following sequence of
    // calls:
    //
    // a[sync] ---> b[host] ---> c[async]

    drop(env_logger::try_init());

    let wat = r#"
        (module
            (import "" "b" (func $b))
            (func $a (export "a")
                call $b
            )
            (func $c (export "c")
                unreachable
            )
        )
    "#;

    let mut async_store = Store::new(&Engine::new(Config::new().async_support(true)).unwrap(), ());

    let async_module = Module::new(async_store.engine(), wat)?;

    let mut async_linker = Linker::new(async_store.engine());
    async_linker.func_wrap("", "b", |_caller: Caller<_>| -> () { unreachable!() })?;

    let async_instance = async_linker
        .instantiate_async(&mut async_store, &async_module)
        .await?;

    struct SyncCtx {
        async_instance: Instance,
        async_store: Store<()>,
    }

    let mut sync_store = Store::new(
        &Engine::default(),
        SyncCtx {
            async_instance,
            async_store,
        },
    );

    let sync_module = Module::new(sync_store.engine(), wat)?;

    let mut sync_linker = Linker::new(sync_store.engine());
    sync_linker.func_wrap("", "b", move |mut caller: Caller<SyncCtx>| -> Result<()> {
        log::info!("Called `b`...");
        let async_instance = caller.data().async_instance;
        let async_store = &mut caller.data_mut().async_store;

        log::info!("Calling `c`...");
        let c = async_instance
            .get_typed_func::<(), ()>(&mut *async_store, "c")
            .unwrap();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async move { c.call_async(async_store, ()).await })
        })?;
        Ok(())
    })?;

    let sync_instance = sync_linker.instantiate(&mut sync_store, &sync_module)?;

    log::info!("Calling `a`...");
    let a = sync_instance
        .get_typed_func::<(), ()>(&mut sync_store, "a")
        .unwrap();
    let trap = a.call(&mut sync_store, ()).unwrap_err();

    let trace = trap.downcast_ref::<WasmBacktrace>().unwrap().frames();
    // We don't support cross-store or cross-engine symbolication currently, so
    // the other frames are ignored.
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].func_name(), Some("c"));

    Ok(())
}

#[test]
fn standalone_backtrace() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let trace = WasmBacktrace::capture(&store);
    assert!(trace.frames().is_empty());
    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "" (func $host))
                (func $foo (export "f") call $bar)
                (func $bar call $host)
            )
        "#,
    )?;
    let func = Func::wrap(&mut store, |cx: Caller<'_, ()>| {
        let trace = WasmBacktrace::capture(&cx);
        assert_eq!(trace.frames().len(), 2);
        let frame1 = &trace.frames()[0];
        let frame2 = &trace.frames()[1];
        assert_eq!(frame1.func_index(), 2);
        assert_eq!(frame1.func_name(), Some("bar"));
        assert_eq!(frame2.func_index(), 1);
        assert_eq!(frame2.func_name(), Some("foo"));
    });
    let instance = Instance::new(&mut store, &module, &[func.into()])?;
    let f = instance.get_typed_func::<(), ()>(&mut store, "f")?;
    f.call(&mut store, ())?;
    Ok(())
}

#[test]
#[allow(deprecated)]
fn standalone_backtrace_disabled() -> Result<()> {
    let mut config = Config::new();
    config.wasm_backtrace(false);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "" (func $host))
                (func $foo (export "f") call $bar)
                (func $bar call $host)
            )
        "#,
    )?;
    let func = Func::wrap(&mut store, |cx: Caller<'_, ()>| {
        let trace = WasmBacktrace::capture(&cx);
        assert_eq!(trace.frames().len(), 0);
        let trace = WasmBacktrace::force_capture(&cx);
        assert_eq!(trace.frames().len(), 2);
    });
    let instance = Instance::new(&mut store, &module, &[func.into()])?;
    let f = instance.get_typed_func::<(), ()>(&mut store, "f")?;
    f.call(&mut store, ())?;
    Ok(())
}

#[test]
fn host_return_error_no_backtrace() -> Result<()> {
    let mut config = Config::new();
    config.wasm_backtrace(false);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "" (func $host))
                (func $foo (export "f") call $bar)
                (func $bar call $host)
            )
        "#,
    )?;
    let func = Func::wrap(&mut store, |_cx: Caller<'_, ()>| -> Result<()> {
        bail!("test")
    });
    let instance = Instance::new(&mut store, &module, &[func.into()])?;
    let f = instance.get_typed_func::<(), ()>(&mut store, "f")?;
    assert!(f.call(&mut store, ()).is_err());
    Ok(())
}

#[test]
fn div_plus_load_reported_right() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
            (module
                (memory (export "memory") 1)
                (func (export "i32.div_s") (param i32 i32) (result i32)
                    (i32.div_s (local.get 0) (i32.load (local.get 1))))
                (func (export "i32.div_u") (param i32 i32) (result i32)
                    (i32.div_u (local.get 0) (i32.load (local.get 1))))
                (func (export "i32.rem_s") (param i32 i32) (result i32)
                    (i32.rem_s (local.get 0) (i32.load (local.get 1))))
                (func (export "i32.rem_u") (param i32 i32) (result i32)
                    (i32.rem_u (local.get 0) (i32.load (local.get 1))))
            )
        "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let memory = instance.get_memory(&mut store, "memory").unwrap();
    let i32_div_s = instance.get_typed_func::<(i32, i32), i32>(&mut store, "i32.div_s")?;
    let i32_div_u = instance.get_typed_func::<(u32, u32), u32>(&mut store, "i32.div_u")?;
    let i32_rem_s = instance.get_typed_func::<(i32, i32), i32>(&mut store, "i32.rem_s")?;
    let i32_rem_u = instance.get_typed_func::<(u32, u32), u32>(&mut store, "i32.rem_u")?;

    memory.write(&mut store, 0, &1i32.to_le_bytes()).unwrap();
    memory.write(&mut store, 4, &0i32.to_le_bytes()).unwrap();
    memory.write(&mut store, 8, &(-1i32).to_le_bytes()).unwrap();

    assert_eq!(i32_div_s.call(&mut store, (100, 0))?, 100);
    assert_eq!(i32_div_u.call(&mut store, (101, 0))?, 101);
    assert_eq!(i32_rem_s.call(&mut store, (102, 0))?, 0);
    assert_eq!(i32_rem_u.call(&mut store, (103, 0))?, 0);

    assert_trap(
        i32_div_s.call(&mut store, (100, 4)),
        Trap::IntegerDivisionByZero,
    );
    assert_trap(
        i32_div_u.call(&mut store, (100, 4)),
        Trap::IntegerDivisionByZero,
    );
    assert_trap(
        i32_rem_s.call(&mut store, (100, 4)),
        Trap::IntegerDivisionByZero,
    );
    assert_trap(
        i32_rem_u.call(&mut store, (100, 4)),
        Trap::IntegerDivisionByZero,
    );

    assert_trap(
        i32_div_s.call(&mut store, (i32::MIN, 8)),
        Trap::IntegerOverflow,
    );
    assert_eq!(i32_rem_s.call(&mut store, (i32::MIN, 8))?, 0);

    assert_trap(
        i32_div_s.call(&mut store, (100, 100_000)),
        Trap::MemoryOutOfBounds,
    );
    assert_trap(
        i32_div_u.call(&mut store, (100, 100_000)),
        Trap::MemoryOutOfBounds,
    );
    assert_trap(
        i32_rem_s.call(&mut store, (100, 100_000)),
        Trap::MemoryOutOfBounds,
    );
    assert_trap(
        i32_rem_u.call(&mut store, (100, 100_000)),
        Trap::MemoryOutOfBounds,
    );

    return Ok(());

    #[track_caller]
    fn assert_trap<T>(result: Result<T>, expected: Trap) {
        match result {
            Ok(_) => panic!("expected failure"),
            Err(e) => {
                if let Some(code) = e.downcast_ref::<Trap>() {
                    if *code == expected {
                        return;
                    }
                }
                panic!("unexpected error {e:?}");
            }
        }
    }
}

#[test]
fn wasm_fault_address_reported_by_default() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
            (module
                (memory 1)
                (func $start
                    i32.const 0xdeadbeef
                    i32.load
                    drop)
                (start $start)
            )
        "#,
    )?;
    let err = Instance::new(&mut store, &module, &[]).unwrap_err();

    // NB: at this time there's no programmatic access to the fault address
    // because it's not always available for load/store traps. Only static
    // memories on 32-bit have this information, but bounds-checked memories use
    // manual trapping instructions and otherwise don't have a means of
    // communicating the faulting address at this time.
    //
    // It looks like the exact reported fault address may not be deterministic,
    // so assert that we have the right error message, but not the exact
    // address.
    let err = format!("{err:?}");
    assert!(
        err.contains("memory fault at wasm address ")
            && err.contains(" in linear memory of size 0x10000"),
        "bad error: {err}"
    );
    Ok(())
}

#[cfg(target_arch = "x86_64")]
#[test]
fn wasm_fault_address_reported_from_mpk_protected_memory() -> Result<()> {
    // Trigger the case where an OOB memory access causes a segfault and the
    // store attempts to convert it into a `WasmFault`, calculating the Wasm
    // address from the raw faulting address. Previously, a store could not do
    // this calculation for MPK-protected, causing an abort.
    let mut pool = crate::small_pool_config();
    pool.total_memories(16);
    pool.memory_protection_keys(MpkEnabled::Auto);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config)?;

    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
            (module
                (memory 1)
                (func $start
                    i32.const 0xdeadbeef
                    i32.load
                    drop)
                (start $start)
            )
        "#,
    )?;
    let err = Instance::new(&mut store, &module, &[]).unwrap_err();

    // We expect an error here, not an abort; but we also check that the store
    // can now calculate the correct Wasm address. If this test is failing with
    // an abort, use `--nocapture` to see more details.
    let err = format!("{err:?}");
    assert!(err.contains("0xdeadbeef"), "bad error: {err}");
    Ok(())
}

#[test]
fn trap_with_array_to_wasm_stack_args() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
            (module
                (func $trap
                    unreachable)
                (func $run (param i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64)
                    call $trap)
                (export "run" (func $run))
            )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let run = instance.get_func(&mut store, "run").unwrap();

    let err = run
        .call(
            &mut store,
            &[
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
                Val::I64(0),
            ],
            &mut [],
        )
        .unwrap_err();
    assert!(err.is::<Trap>());

    let trace = err.downcast_ref::<WasmBacktrace>().unwrap();
    assert_eq!(trace.frames().len(), 2);
    assert_eq!(trace.frames()[0].func_name(), Some("trap"));
    assert_eq!(trace.frames()[1].func_name(), Some("run"));

    Ok(())
}

#[test]
fn trap_with_native_to_wasm_stack_args() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
            (module
                (func $trap
                    unreachable)
                (func $run (param i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64)
                    call $trap)
                (export "run" (func $run))
            )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let run = instance.get_func(&mut store, "run").unwrap();

    let err = run
        .typed::<(
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
        ), ()>(&mut store)?
        .call(&mut store, (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0))
        .unwrap_err();
    assert!(err.is::<Trap>());

    let trace = err.downcast_ref::<WasmBacktrace>().unwrap();
    assert_eq!(trace.frames().len(), 2);
    assert_eq!(trace.frames()[0].func_name(), Some("trap"));
    assert_eq!(trace.frames()[1].func_name(), Some("run"));

    Ok(())
}

#[test]
fn dont_see_stale_stack_walking_registers() -> Result<()> {
    let engine = Engine::default();

    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "host_start" (func $host_start))
                (import "" "host_get_trap" (func $host_get_trap))
                (export "get_trap" (func $host_get_trap))

                ;; We enter and exit Wasm, which saves registers in the
                ;; `VMRuntimeLimits`. Later, when we call a re-exported host
                ;; function, we should not accidentally reuse those saved
                ;; registers.
                (start $start)
                (func $start
                    (call $host_start)
                )
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    let host_start = Func::new(
        &mut store,
        FuncType::new(&engine, [], []),
        |_caller, _args, _results| Ok(()),
    );
    linker.define(&store, "", "host_start", host_start)?;

    let host_get_trap = Func::new(
        &mut store,
        FuncType::new(&engine, [], []),
        |_caller, _args, _results| Err(anyhow::anyhow!("trap!!!")),
    );
    linker.define(&store, "", "host_get_trap", host_get_trap)?;

    let instance = linker.instantiate(&mut store, &module)?;
    let get_trap = instance.get_func(&mut store, "get_trap").unwrap();

    let err = get_trap.call(&mut store, &[], &mut []).unwrap_err();
    assert!(err.to_string().contains("trap!!!"));

    Ok(())
}

#[test]
fn same_module_multiple_stores() -> Result<()> {
    let _ = env_logger::try_init();

    let engine = Engine::default();

    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "f" (func $f))
                (import "" "call_ref" (func $call_ref (param funcref)))
                (global $g (mut i32) (i32.const 0))
                (func $a (export "a")
                    call $b
                )
                (func $b
                    call $c
                )
                (func $c
                    global.get $g
                    if
                        call $f
                    else
                        i32.const 1
                        global.set $g
                        ref.func $a
                        call $call_ref
                    end
                )
            )
        "#,
    )?;

    let stacks = Arc::new(Mutex::new(vec![]));

    let mut store3 = Store::new(&engine, ());
    let f3 = Func::new(&mut store3, FuncType::new(&engine, [], []), {
        let stacks = stacks.clone();
        move |caller, _params, _results| {
            stacks
                .lock()
                .unwrap()
                .push(WasmBacktrace::force_capture(caller));
            Ok(())
        }
    });
    let call_ref3 = Func::wrap(&mut store3, |caller: Caller<'_, _>, f: Option<Func>| {
        f.unwrap().call(caller, &[], &mut [])
    });
    let instance3 = Instance::new(&mut store3, &module, &[f3.into(), call_ref3.into()])?;

    let mut store2 = Store::new(&engine, store3);
    let f2 = Func::new(&mut store2, FuncType::new(&engine, [], []), {
        let stacks = stacks.clone();
        move |mut caller, _params, _results| {
            stacks
                .lock()
                .unwrap()
                .push(WasmBacktrace::force_capture(&mut caller));
            instance3
                .get_typed_func::<(), ()>(caller.data_mut(), "a")
                .unwrap()
                .call(caller.data_mut(), ())
                .unwrap();
            Ok(())
        }
    });
    let call_ref2 = Func::wrap(&mut store2, |caller: Caller<'_, _>, f: Option<Func>| {
        f.unwrap().call(caller, &[], &mut [])
    });
    let instance2 = Instance::new(&mut store2, &module, &[f2.into(), call_ref2.into()])?;

    let mut store1 = Store::new(&engine, store2);
    let f1 = Func::new(&mut store1, FuncType::new(&engine, [], []), {
        let stacks = stacks.clone();
        move |mut caller, _params, _results| {
            stacks
                .lock()
                .unwrap()
                .push(WasmBacktrace::force_capture(&mut caller));
            instance2
                .get_typed_func::<(), ()>(caller.data_mut(), "a")
                .unwrap()
                .call(caller.data_mut(), ())
                .unwrap();
            Ok(())
        }
    });
    let call_ref1 = Func::wrap(&mut store1, |caller: Caller<'_, _>, f: Option<Func>| {
        f.unwrap().call(caller, &[], &mut [])
    });
    let instance1 = Instance::new(&mut store1, &module, &[f1.into(), call_ref1.into()])?;

    instance1
        .get_typed_func::<(), ()>(&mut store1, "a")?
        .call(&mut store1, ())?;

    let expected_stacks = vec![
        // [f1, c1, b1, a1, call_ref1, c1, b1, a1]
        vec!["c", "b", "a", "c", "b", "a"],
        // [f2, c2, b2, a2, call_ref2, c2, b2, a2, f1, c1, b1, a1, call_ref1, c1, b1, a1]
        vec!["c", "b", "a", "c", "b", "a"],
        // [f3, c3, b3, a3, call_ref3, c3, b3, a3, f2, c2, b2, a2, call_ref2, c2, b2, a2, f1, c1, b1, a1, call_ref1, c1, b1, a1]
        vec!["c", "b", "a", "c", "b", "a"],
    ];
    eprintln!("expected = {expected_stacks:#?}");
    let actual_stacks = stacks.lock().unwrap();
    eprintln!("actual = {actual_stacks:#?}");

    assert_eq!(actual_stacks.len(), expected_stacks.len());
    for (expected_stack, actual_stack) in expected_stacks.into_iter().zip(actual_stacks.iter()) {
        assert_eq!(expected_stack.len(), actual_stack.frames().len());
        for (expected_frame, actual_frame) in expected_stack.into_iter().zip(actual_stack.frames())
        {
            assert_eq!(actual_frame.func_name(), Some(expected_frame));
        }
    }

    Ok(())
}

#[test]
fn async_stack_size_ignored_if_disabled() -> Result<()> {
    let mut config = Config::new();
    config.async_support(false);
    config.max_wasm_stack(8 << 20);
    Engine::new(&config)?;

    Ok(())
}
