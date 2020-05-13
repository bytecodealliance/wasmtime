use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasmtime::*;

fn interruptable_store() -> Store {
    let engine = Engine::new(Config::new().interruptable(true));
    Store::new(&engine)
}

fn hugely_recursive_module(store: &Store) -> anyhow::Result<Module> {
    let mut wat = String::new();
    wat.push_str(
        r#"
        (import "" "" (func))
        (func (export "loop") call 2 call 2)
    "#,
    );
    for i in 0..100 {
        wat.push_str(&format!("(func call {0} call {0})\n", i + 3));
    }
    wat.push_str("(func call 0)\n");

    Module::new(&store, &wat)
}

#[test]
fn loops_interruptable() -> anyhow::Result<()> {
    let store = interruptable_store();
    let module = Module::new(&store, r#"(func (export "loop") (loop br 0))"#)?;
    let instance = Instance::new(&module, &[])?;
    let iloop = instance.get_func("loop").unwrap().get0::<()>()?;
    store.interrupt_handle()?.interrupt();
    let trap = iloop().unwrap_err();
    assert!(trap.to_string().contains("wasm trap: interrupt"));
    Ok(())
}

#[test]
fn functions_interruptable() -> anyhow::Result<()> {
    let store = interruptable_store();
    let module = hugely_recursive_module(&store)?;
    let func = Func::wrap(&store, || {});
    let instance = Instance::new(&module, &[func.into()])?;
    let iloop = instance.get_func("loop").unwrap().get0::<()>()?;
    store.interrupt_handle()?.interrupt();
    let trap = iloop().unwrap_err();
    assert!(
        trap.to_string().contains("wasm trap: interrupt"),
        "{}",
        trap.to_string()
    );
    Ok(())
}

#[test]
fn loop_interrupt_from_afar() -> anyhow::Result<()> {
    // Create an instance which calls an imported function on each iteration of
    // the loop so we can count the number of loop iterations we've executed so
    // far.
    static HITS: AtomicUsize = AtomicUsize::new(0);
    let store = interruptable_store();
    let module = Module::new(
        &store,
        r#"
            (import "" "" (func))

            (func (export "loop")
                (loop
                    call 0
                    br 0)
            )
        "#,
    )?;
    let func = Func::wrap(&store, || {
        HITS.fetch_add(1, SeqCst);
    });
    let instance = Instance::new(&module, &[func.into()])?;

    // Use the instance's interrupt handle to wait for it to enter the loop long
    // enough and then we signal an interrupt happens.
    let handle = store.interrupt_handle()?;
    let thread = std::thread::spawn(move || {
        while HITS.load(SeqCst) <= 100_000 {
            // continue ...
        }
        handle.interrupt();
    });

    // Enter the infinitely looping function and assert that our interrupt
    // handle does indeed actually interrupt the function.
    let iloop = instance.get_func("loop").unwrap().get0::<()>()?;
    let trap = iloop().unwrap_err();
    thread.join().unwrap();
    assert!(
        trap.to_string().contains("wasm trap: interrupt"),
        "bad message: {}",
        trap.to_string()
    );
    Ok(())
}

#[test]
fn function_interrupt_from_afar() -> anyhow::Result<()> {
    // Create an instance which calls an imported function on each iteration of
    // the loop so we can count the number of loop iterations we've executed so
    // far.
    static HITS: AtomicUsize = AtomicUsize::new(0);
    let store = interruptable_store();
    let module = hugely_recursive_module(&store)?;
    let func = Func::wrap(&store, || {
        HITS.fetch_add(1, SeqCst);
    });
    let instance = Instance::new(&module, &[func.into()])?;

    // Use the instance's interrupt handle to wait for it to enter the loop long
    // enough and then we signal an interrupt happens.
    let handle = store.interrupt_handle()?;
    let thread = std::thread::spawn(move || {
        while HITS.load(SeqCst) <= 100_000 {
            // continue ...
        }
        handle.interrupt();
    });

    // Enter the infinitely looping function and assert that our interrupt
    // handle does indeed actually interrupt the function.
    let iloop = instance.get_func("loop").unwrap().get0::<()>()?;
    let trap = iloop().unwrap_err();
    thread.join().unwrap();
    assert!(
        trap.to_string().contains("wasm trap: interrupt"),
        "bad message: {}",
        trap.to_string()
    );
    Ok(())
}
