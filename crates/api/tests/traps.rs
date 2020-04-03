use anyhow::Result;
use std::panic::{self, AssertUnwindSafe};
use wasmtime::*;

#[test]
fn test_trap_return() -> Result<()> {
    let store = Store::default();
    let wat = r#"
        (module
        (func $hello (import "" "hello"))
        (func (export "run") (call $hello))
        )
    "#;

    let module = Module::new(&store, wat)?;
    let hello_type = FuncType::new(Box::new([]), Box::new([]));
    let hello_func = Func::new(&store, hello_type, |_, _, _| Err(Trap::new("test 123")));

    let instance = Instance::new(&module, &[hello_func.into()])?;
    let run_func = instance.exports()[0]
        .func()
        .expect("expected function export");

    let e = run_func.call(&[]).err().expect("error calling function");

    assert_eq!(e.message(), "test 123");

    Ok(())
}

#[test]
fn test_trap_trace() -> Result<()> {
    let store = Store::default();
    let wat = r#"
        (module $hello_mod
            (func (export "run") (call $hello))
            (func $hello (unreachable))
        )
    "#;

    let module = Module::new(&store, wat)?;
    let instance = Instance::new(&module, &[])?;
    let run_func = instance.exports()[0]
        .func()
        .expect("expected function export");

    let e = run_func.call(&[]).err().expect("error calling function");

    let trace = e.trace();
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0].module_name().unwrap(), "hello_mod");
    assert_eq!(trace[0].func_index(), 1);
    assert_eq!(trace[0].func_name(), Some("hello"));
    assert_eq!(trace[1].module_name().unwrap(), "hello_mod");
    assert_eq!(trace[1].func_index(), 0);
    assert_eq!(trace[1].func_name(), None);
    assert!(
        e.message().contains("unreachable"),
        "wrong message: {}",
        e.message()
    );

    Ok(())
}

#[test]
fn test_trap_trace_cb() -> Result<()> {
    let store = Store::default();
    let wat = r#"
        (module $hello_mod
            (import "" "throw" (func $throw))
            (func (export "run") (call $hello))
            (func $hello (call $throw))
        )
    "#;

    let fn_type = FuncType::new(Box::new([]), Box::new([]));
    let fn_func = Func::new(&store, fn_type, |_, _, _| Err(Trap::new("cb throw")));

    let module = Module::new(&store, wat)?;
    let instance = Instance::new(&module, &[fn_func.into()])?;
    let run_func = instance.exports()[0]
        .func()
        .expect("expected function export");

    let e = run_func.call(&[]).err().expect("error calling function");

    let trace = e.trace();
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0].module_name().unwrap(), "hello_mod");
    assert_eq!(trace[0].func_index(), 2);
    assert_eq!(trace[1].module_name().unwrap(), "hello_mod");
    assert_eq!(trace[1].func_index(), 1);
    assert_eq!(e.message(), "cb throw");

    Ok(())
}

#[test]
fn test_trap_stack_overflow() -> Result<()> {
    let store = Store::default();
    let wat = r#"
        (module $rec_mod
            (func $run (export "run") (call $run))
        )
    "#;

    let module = Module::new(&store, wat)?;
    let instance = Instance::new(&module, &[])?;
    let run_func = instance.exports()[0]
        .func()
        .expect("expected function export");

    let e = run_func.call(&[]).err().expect("error calling function");

    let trace = e.trace();
    assert!(trace.len() >= 32);
    for i in 0..trace.len() {
        assert_eq!(trace[i].module_name().unwrap(), "rec_mod");
        assert_eq!(trace[i].func_index(), 0);
        assert_eq!(trace[i].func_name(), Some("run"));
    }
    assert!(e.message().contains("call stack exhausted"));

    Ok(())
}

#[test]
fn trap_display_pretty() -> Result<()> {
    let store = Store::default();
    let wat = r#"
        (module $m
            (func $die unreachable)
            (func call $die)
            (func $foo call 1)
            (func (export "bar") call $foo)
        )
    "#;

    let module = Module::new(&store, wat)?;
    let instance = Instance::new(&module, &[])?;
    let run_func = instance.exports()[0]
        .func()
        .expect("expected function export");

    let e = run_func.call(&[]).err().expect("error calling function");
    assert_eq!(
        e.to_string(),
        "\
wasm trap: unreachable, source location: @0023
wasm backtrace:
  0: m!die
  1: m!<wasm function 1>
  2: m!foo
  3: m!<wasm function 3>
"
    );
    Ok(())
}

#[test]
fn trap_display_multi_module() -> Result<()> {
    let store = Store::default();
    let wat = r#"
        (module $a
            (func $die unreachable)
            (func call $die)
            (func $foo call 1)
            (func (export "bar") call $foo)
        )
    "#;

    let module = Module::new(&store, wat)?;
    let instance = Instance::new(&module, &[])?;
    let bar = instance.exports()[0].clone();

    let wat = r#"
        (module $b
            (import "" "" (func $bar))
            (func $middle call $bar)
            (func (export "bar2") call $middle)
        )
    "#;
    let module = Module::new(&store, wat)?;
    let instance = Instance::new(&module, &[bar])?;
    let bar2 = instance.exports()[0]
        .func()
        .expect("expected function export");

    let e = bar2.call(&[]).err().expect("error calling function");
    assert_eq!(
        e.to_string(),
        "\
wasm trap: unreachable, source location: @0023
wasm backtrace:
  0: a!die
  1: a!<wasm function 1>
  2: a!foo
  3: a!<wasm function 3>
  4: b!middle
  5: b!<wasm function 2>
"
    );
    Ok(())
}

#[test]
fn trap_start_function_import() -> Result<()> {
    let store = Store::default();
    let binary = wat::parse_str(
        r#"
            (module $a
                (import "" "" (func $foo))
                (start $foo)
            )
        "#,
    )?;

    let module = Module::new(&store, &binary)?;
    let sig = FuncType::new(Box::new([]), Box::new([]));
    let func = Func::new(&store, sig, |_, _, _| Err(Trap::new("user trap")));
    let err = Instance::new(&module, &[func.into()]).err().unwrap();
    assert_eq!(err.downcast_ref::<Trap>().unwrap().message(), "user trap");
    Ok(())
}

#[test]
fn rust_panic_import() -> Result<()> {
    let store = Store::default();
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

    let module = Module::new(&store, &binary)?;
    let sig = FuncType::new(Box::new([]), Box::new([]));
    let func = Func::new(&store, sig, |_, _, _| panic!("this is a panic"));
    let instance = Instance::new(
        &module,
        &[
            func.into(),
            Func::wrap(&store, || panic!("this is another panic")).into(),
        ],
    )?;
    let func = instance.exports()[0].func().unwrap().clone();
    let err = panic::catch_unwind(AssertUnwindSafe(|| {
        drop(func.call(&[]));
    }))
    .unwrap_err();
    assert_eq!(err.downcast_ref::<&'static str>(), Some(&"this is a panic"));

    let func = instance.exports()[1].func().unwrap().clone();
    let err = panic::catch_unwind(AssertUnwindSafe(|| {
        drop(func.call(&[]));
    }))
    .unwrap_err();
    assert_eq!(
        err.downcast_ref::<&'static str>(),
        Some(&"this is another panic")
    );
    Ok(())
}

#[test]
fn rust_panic_start_function() -> Result<()> {
    let store = Store::default();
    let binary = wat::parse_str(
        r#"
            (module $a
                (import "" "" (func $foo))
                (start $foo)
            )
        "#,
    )?;

    let module = Module::new(&store, &binary)?;
    let sig = FuncType::new(Box::new([]), Box::new([]));
    let func = Func::new(&store, sig, |_, _, _| panic!("this is a panic"));
    let err = panic::catch_unwind(AssertUnwindSafe(|| {
        drop(Instance::new(&module, &[func.into()]));
    }))
    .unwrap_err();
    assert_eq!(err.downcast_ref::<&'static str>(), Some(&"this is a panic"));

    let func = Func::wrap(&store, || panic!("this is another panic"));
    let err = panic::catch_unwind(AssertUnwindSafe(|| {
        drop(Instance::new(&module, &[func.into()]));
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
    let store = Store::default();
    let binary = wat::parse_str(
        r#"
            (module $a
                (func (export "foo") (param i32))
            )
        "#,
    )?;

    let module = Module::new(&store, &binary)?;
    let instance = Instance::new(&module, &[])?;
    let func = instance.exports()[0].func().unwrap().clone();
    assert_eq!(
        func.call(&[]).unwrap_err().message(),
        "expected 1 arguments, got 0"
    );
    assert_eq!(
        func.call(&[Val::F32(0)]).unwrap_err().message(),
        "argument type mismatch",
    );
    assert_eq!(
        func.call(&[Val::I32(0), Val::I32(1)])
            .unwrap_err()
            .message(),
        "expected 1 arguments, got 2"
    );
    Ok(())
}

#[test]
fn call_signature_mismatch() -> Result<()> {
    let store = Store::default();
    let binary = wat::parse_str(
        r#"
            (module $a
                (func $foo
                    i32.const 0
                    call_indirect)
                (func $bar (param i32))
                (start $foo)

                (table 1 anyfunc)
                (elem (i32.const 0) 1)
            )
        "#,
    )?;

    let module = Module::new(&store, &binary)?;
    let err = Instance::new(&module, &[])
        .err()
        .unwrap()
        .downcast::<Trap>()
        .unwrap();
    assert_eq!(
        err.message(),
        "wasm trap: indirect call type mismatch, source location: @0030"
    );
    Ok(())
}

#[test]
fn start_trap_pretty() -> Result<()> {
    let store = Store::default();
    let wat = r#"
        (module $m
            (func $die unreachable)
            (func call $die)
            (func $foo call 1)
            (func $start call $foo)
            (start $start)
        )
    "#;

    let module = Module::new(&store, wat)?;
    let e = match Instance::new(&module, &[]) {
        Ok(_) => panic!("expected failure"),
        Err(e) => e.downcast::<Trap>()?,
    };

    assert_eq!(
        e.to_string(),
        "\
wasm trap: unreachable, source location: @001d
wasm backtrace:
  0: m!die
  1: m!<wasm function 1>
  2: m!foo
  3: m!start
"
    );
    Ok(())
}
