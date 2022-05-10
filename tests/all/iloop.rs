use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use wasmtime::*;

fn interruptable_store() -> Store<()> {
    let engine = Engine::new(Config::new().epoch_interruption(true)).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_epoch_deadline(1);
    store
}

fn hugely_recursive_module(engine: &Engine) -> anyhow::Result<Module> {
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

    Module::new(engine, &wat)
}

#[test]
fn loops_interruptable() -> anyhow::Result<()> {
    let mut store = interruptable_store();
    let module = Module::new(store.engine(), r#"(func (export "loop") (loop br 0))"#)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let iloop = instance.get_typed_func::<(), (), _>(&mut store, "loop")?;
    store.engine().increment_epoch();
    let trap = iloop.call(&mut store, ()).unwrap_err();
    assert!(
        trap.trap_code().unwrap() == TrapCode::Interrupt,
        "bad message: {}",
        trap
    );
    Ok(())
}

#[test]
fn functions_interruptable() -> anyhow::Result<()> {
    let mut store = interruptable_store();
    let module = hugely_recursive_module(store.engine())?;
    let func = Func::wrap(&mut store, || {});
    let instance = Instance::new(&mut store, &module, &[func.into()])?;
    let iloop = instance.get_typed_func::<(), (), _>(&mut store, "loop")?;
    store.engine().increment_epoch();
    let trap = iloop.call(&mut store, ()).unwrap_err();
    assert!(
        trap.trap_code().unwrap() == TrapCode::Interrupt,
        "{}",
        trap.to_string()
    );
    Ok(())
}

const NUM_HITS: usize = 100_000;

#[test]
fn loop_interrupt_from_afar() -> anyhow::Result<()> {
    // Create an instance which calls an imported function on each iteration of
    // the loop so we can count the number of loop iterations we've executed so
    // far.
    static HITS: AtomicUsize = AtomicUsize::new(0);
    static STOP: AtomicBool = AtomicBool::new(false);
    let mut store = interruptable_store();
    let module = Module::new(
        store.engine(),
        r#"
            (import "" "" (func))

            (func (export "loop")
                (loop
                    call 0
                    br 0)
            )
        "#,
    )?;
    let func = Func::wrap(&mut store, || {
        HITS.fetch_add(1, SeqCst);
    });
    let instance = Instance::new(&mut store, &module, &[func.into()])?;

    // Use the engine to wait for it to enter the loop long enough and then we
    // signal an interrupt happens.
    let engine = store.engine().clone();
    let thread = std::thread::spawn(move || {
        while HITS.load(SeqCst) <= NUM_HITS && !STOP.load(SeqCst) {
            // continue ...
        }
        println!("interrupting");
        engine.increment_epoch();
    });

    // Enter the infinitely looping function and assert that our interrupt
    // handle does indeed actually interrupt the function.
    let iloop = instance.get_typed_func::<(), (), _>(&mut store, "loop")?;
    let trap = iloop.call(&mut store, ()).unwrap_err();
    STOP.store(true, SeqCst);
    thread.join().unwrap();
    assert!(HITS.load(SeqCst) > NUM_HITS);
    assert!(
        trap.trap_code().unwrap() == TrapCode::Interrupt,
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
    static STOP: AtomicBool = AtomicBool::new(false);

    let mut store = interruptable_store();
    let module = hugely_recursive_module(store.engine())?;
    let func = Func::wrap(&mut store, || {
        HITS.fetch_add(1, SeqCst);
    });
    let instance = Instance::new(&mut store, &module, &[func.into()])?;

    // Use the instance's interrupt handle to wait for it to enter the loop long
    // enough and then we signal an interrupt happens.
    let engine = store.engine().clone();
    let thread = std::thread::spawn(move || {
        while HITS.load(SeqCst) <= NUM_HITS && !STOP.load(SeqCst) {
            // continue ...
        }
        engine.increment_epoch();
    });

    // Enter the infinitely looping function and assert that our interrupt
    // handle does indeed actually interrupt the function.
    let iloop = instance.get_typed_func::<(), (), _>(&mut store, "loop")?;
    let trap = iloop.call(&mut store, ()).unwrap_err();
    STOP.store(true, SeqCst);
    thread.join().unwrap();
    assert!(HITS.load(SeqCst) > NUM_HITS);
    assert!(
        trap.trap_code().unwrap() == TrapCode::Interrupt,
        "bad message: {}",
        trap.to_string()
    );
    Ok(())
}
