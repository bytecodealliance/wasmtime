#![cfg(feature = "debug")]

use wasmtime::*;

fn engine() -> Result<Engine> {
    let mut config = Config::new();
    config.guest_debug(true);
    Engine::new(&config)
}

#[test]
#[cfg(feature = "component-model")]
fn breakpoint_in_exported_module() -> Result<()> {
    use wasmtime::component::{Component, Linker};

    let engine = engine()?;
    let component = Component::new(
        &engine,
        r#"(component
            (core module $m (func nop))
            (export "m" (core module $m)))"#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let module = instance.get_module(&mut store, "m").unwrap();
    store
        .edit_breakpoints()
        .unwrap()
        .add_breakpoint(&module, ModulePC::new(0))?;
    let breakpoints = store.breakpoints().unwrap().collect::<Vec<_>>();
    assert_eq!(breakpoints.len(), 1);
    assert!(Module::same(&breakpoints[0].module, &module));
    Ok(())
}

#[test]
fn breakpoint_add_remove_is_idempotent() -> Result<()> {
    let engine = engine()?;
    let module = Module::new(&engine, "(module (func nop))")?;
    let mut store = Store::new(&engine, ());
    Instance::new(&mut store, &module, &[])?;
    // Both addresses slip forward to the same first opcode.
    for pc in [ModulePC::new(0), ModulePC::new(1)] {
        for _ in 0..2 {
            store
                .edit_breakpoints()
                .unwrap()
                .add_breakpoint(&module, pc)?;
        }
    }
    let actual = store.breakpoints().unwrap().next().unwrap().pc;
    // Removing the actual PC was never requested, so it must have no effect.
    store
        .edit_breakpoints()
        .unwrap()
        .remove_breakpoint(&module, actual)?;
    assert_eq!(store.breakpoints().unwrap().count(), 1);
    for _ in 0..2 {
        store
            .edit_breakpoints()
            .unwrap()
            .remove_breakpoint(&module, ModulePC::new(0))?;
    }
    assert_eq!(store.breakpoints().unwrap().count(), 1);
    for _ in 0..2 {
        store
            .edit_breakpoints()
            .unwrap()
            .remove_breakpoint(&module, ModulePC::new(1))?;
    }
    assert_eq!(store.breakpoints().unwrap().count(), 0);
    // Exact opcode requests have the same idempotent semantics.
    for _ in 0..2 {
        store
            .edit_breakpoints()
            .unwrap()
            .add_breakpoint(&module, actual)?;
    }
    store
        .edit_breakpoints()
        .unwrap()
        .remove_breakpoint(&module, actual)?;
    assert_eq!(store.breakpoints().unwrap().count(), 0);
    Ok(())
}

#[test]
fn forgotten_breakpoint_editor() -> Result<()> {
    // Isolate execution so an executable-permission regression is an assertion
    // failure in the parent, rather than a crash of the entire test suite.
    const CHILD: &str = "WASMTIME_FORGOTTEN_BREAKPOINT_EDITOR_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", "forgotten_breakpoint_editor", "--nocapture"])
            .env(CHILD, "1")
            .status()?;
        assert!(
            status.success(),
            "forgotten breakpoint editor child failed: {status}"
        );
        return Ok(());
    }
    let engine = engine()?;
    let module = Module::new(&engine, r#"(module (func (export "f") nop))"#)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<(), ()>(&mut store, "f")?;
    for mode in 0..4 {
        let mut edit = store.edit_breakpoints().unwrap();
        match mode {
            0 => edit.add_breakpoint(&module, ModulePC::new(0))?,
            1 => edit.remove_breakpoint(&module, ModulePC::new(0))?,
            2 => edit.single_step(true)?,
            _ => edit.single_step(false)?,
        }
        std::mem::forget(edit);
        f.call(&mut store, ())?;
    }
    Ok(())
}
