//! A standalone test to assert that Wasmtime can operate in "no signal handlers
//! mode"
//!
//! This is a test for `Config::signals_based_traps(false)` which resides in its
//! own binary to assert properties about signal handlers that Wasmtime uses.
//! Due to the global nature of signals no other tests can be in this binary.
//! This will ensure that various trapping scenarios all work and additionally
//! signal handlers are not registered.

use anyhow::Result;
use wasmtime::{Config, Engine, Instance, Module, Store, Trap};

#[test]
fn no_host_trap_handlers() -> Result<()> {
    let mut config = Config::new();
    config.signals_based_traps(false);
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
    #[cfg(windows)]
    {
        // Note that this can't be checked on Windows because vectored exception
        // handlers work a bit differently and aren't as "global" as a signal
        // handler. For now rely on the check above on unix to also guarantee
        // that on Windows we don't register any vectored exception handlers.
    }
}
