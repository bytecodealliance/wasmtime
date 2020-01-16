use anyhow::Result;
use std::rc::Rc;
use wasmtime::*;

#[test]
fn test_trap_return() -> Result<()> {
    struct HelloCallback;

    impl Callable for HelloCallback {
        fn call(&self, _params: &[Val], _results: &mut [Val]) -> Result<(), Trap> {
            Err(Trap::new("test 123"))
        }
    }

    let store = Store::default();
    let binary = wat::parse_str(
        r#"
            (module
            (func $hello (import "" "hello"))
            (func (export "run") (call $hello))
            )
        "#,
    )?;

    let module = Module::new(&store, &binary)?;
    let hello_type = FuncType::new(Box::new([]), Box::new([]));
    let hello_func = Func::new(&store, hello_type, Rc::new(HelloCallback));

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
    let binary = wat::parse_str(
        r#"
            (module $hello_mod
                (func (export "run") (call $hello))
                (func $hello (unreachable))
            )
        "#,
    )?;

    let module = Module::new(&store, &binary)?;
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
    assert!(e.message().contains("unreachable"));

    Ok(())
}

#[test]
fn test_trap_trace_cb() -> Result<()> {
    struct ThrowCallback;

    impl Callable for ThrowCallback {
        fn call(&self, _params: &[Val], _results: &mut [Val]) -> Result<(), Trap> {
            Err(Trap::new("cb throw"))
        }
    }

    let store = Store::default();
    let binary = wat::parse_str(
        r#"
            (module $hello_mod
                (import "" "throw" (func $throw))
                (func (export "run") (call $hello))
                (func $hello (call $throw))
            )
        "#,
    )?;

    let fn_type = FuncType::new(Box::new([]), Box::new([]));
    let fn_func = Func::new(&store, fn_type, Rc::new(ThrowCallback));

    let module = Module::new(&store, &binary)?;
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
    let binary = wat::parse_str(
        r#"
            (module $rec_mod
                (func $run (export "run") (call $run))
            )
        "#,
    )?;

    let module = Module::new(&store, &binary)?;
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
