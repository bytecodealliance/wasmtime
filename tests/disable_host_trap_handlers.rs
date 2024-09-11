use anyhow::Result;
use wasmtime::{Config, Engine, Instance, Module, Store, Trap};

#[test]
fn no_host_trap_handlers() -> Result<()> {
    let mut config = Config::new();
    config.host_trap_handlers(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
                (memory 1)

                (func (export "load") (param i32) (result i32)
                    (i32.load (local.get 0)))

                (func (export "div") (param i32 i32) (result i32)
                    (i32.div_s (local.get 0) (local.get 1)))

                (func (export "unreachable") unreachable)
                (func $oflow (export "overflow") call $oflow)
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let load = instance.get_typed_func::<i32, i32>(&mut store, "load")?;
    let div = instance.get_typed_func::<(i32, i32), i32>(&mut store, "div")?;
    let unreachable = instance.get_typed_func::<(), ()>(&mut store, "unreachable")?;
    let overflow = instance.get_typed_func::<(), ()>(&mut store, "overflow")?;

    let trap = load
        .call(&mut store, 1 << 20)
        .unwrap_err()
        .downcast::<Trap>()?;
    assert_eq!(trap, Trap::MemoryOutOfBounds);

    let trap = div
        .call(&mut store, (1, 0))
        .unwrap_err()
        .downcast::<Trap>()?;
    assert_eq!(trap, Trap::IntegerDivisionByZero);

    let trap = unreachable
        .call(&mut store, ())
        .unwrap_err()
        .downcast::<Trap>()?;
    assert_eq!(trap, Trap::UnreachableCodeReached);

    let trap = overflow
        .call(&mut store, ())
        .unwrap_err()
        .downcast::<Trap>()?;
    assert_eq!(trap, Trap::StackOverflow);

    assert_host_signal_handlers_are_unset();

    Ok(())
}

fn assert_host_signal_handlers_are_unset() {
    #[cfg(unix)]
    unsafe {
        let mut prev = std::mem::zeroed::<libc::sigaction>();
        let rc = libc::sigaction(libc::SIGILL, std::ptr::null(), &mut prev);
        assert_eq!(rc, 0);
        assert_eq!(
            prev.sa_sigaction,
            libc::SIG_DFL,
            "fault handler was installed when it shouldn't have been"
        );
    }
}
