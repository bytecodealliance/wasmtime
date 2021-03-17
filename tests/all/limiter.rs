use anyhow::Result;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasmtime::*;

#[test]
fn test_static_limiter() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (table (export "t") 0 anyfunc))"#,
    )?;

    let store = Store::new_with_limiter(&engine, StaticResourceLimiter::new(Some(10), Some(5)));

    let instance = Instance::new(&store, &module, &[])?;

    let memory = instance.get_memory("m").unwrap();

    memory.grow(3)?;
    memory.grow(5)?;
    memory.grow(2)?;

    assert_eq!(
        memory.grow(1).map_err(|e| e.to_string()).unwrap_err(),
        "failed to grow memory by `1`"
    );

    let table = instance.get_table("t").unwrap();

    table.grow(2, Val::FuncRef(None))?;
    table.grow(1, Val::FuncRef(None))?;
    table.grow(2, Val::FuncRef(None))?;

    assert_eq!(
        table
            .grow(1, Val::FuncRef(None))
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "failed to grow table by `1`"
    );

    Ok(())
}

#[test]
fn test_static_limiter_memory_only() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (table (export "t") 0 anyfunc))"#,
    )?;

    let store = Store::new_with_limiter(&engine, StaticResourceLimiter::new(Some(10), None));

    let instance = Instance::new(&store, &module, &[])?;

    let memory = instance.get_memory("m").unwrap();

    memory.grow(3)?;
    memory.grow(5)?;
    memory.grow(2)?;

    assert_eq!(
        memory.grow(1).map_err(|e| e.to_string()).unwrap_err(),
        "failed to grow memory by `1`"
    );

    let table = instance.get_table("t").unwrap();

    table.grow(2, Val::FuncRef(None))?;
    table.grow(1, Val::FuncRef(None))?;
    table.grow(2, Val::FuncRef(None))?;
    table.grow(1, Val::FuncRef(None))?;

    Ok(())
}

#[test]
fn test_static_limiter_table_only() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (table (export "t") 0 anyfunc))"#,
    )?;

    let store = Store::new_with_limiter(&engine, StaticResourceLimiter::new(None, Some(5)));

    let instance = Instance::new(&store, &module, &[])?;

    let memory = instance.get_memory("m").unwrap();

    memory.grow(3)?;
    memory.grow(5)?;
    memory.grow(2)?;
    memory.grow(1)?;

    let table = instance.get_table("t").unwrap();

    table.grow(2, Val::FuncRef(None))?;
    table.grow(1, Val::FuncRef(None))?;
    table.grow(2, Val::FuncRef(None))?;

    assert_eq!(
        table
            .grow(1, Val::FuncRef(None))
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "failed to grow table by `1`"
    );

    Ok(())
}

struct MemoryContext {
    host_memory_used: usize,
    wasm_memory_used: usize,
    memory_limit: usize,
    limit_exceeded: bool,
}

struct HostMemoryLimiter(Rc<Cell<bool>>);

impl ResourceLimiter for HostMemoryLimiter {
    fn memory_growing(
        &self,
        store: &Store,
        current: u32,
        desired: u32,
        maximum: Option<u32>,
    ) -> bool {
        if let Some(ctx) = store.get::<Rc<RefCell<MemoryContext>>>() {
            let mut ctx = ctx.borrow_mut();

            // Check if the desired exceeds a maximum (either from Wasm or from the host)
            if desired > maximum.unwrap_or(u32::MAX) {
                ctx.limit_exceeded = true;
                return false;
            }

            assert_eq!(current as usize * 0x10000, ctx.wasm_memory_used);
            let desired = desired as usize * 0x10000;

            if desired + ctx.host_memory_used > ctx.memory_limit {
                ctx.limit_exceeded = true;
                return false;
            }

            ctx.wasm_memory_used = desired;
        }

        true
    }

    fn table_growing(
        &self,
        _store: &Store,
        _current: u32,
        _desired: u32,
        _maximum: Option<u32>,
    ) -> bool {
        true
    }
}

impl Drop for HostMemoryLimiter {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

#[test]
fn test_custom_limiter() -> Result<()> {
    let mut config = Config::default();

    // This approximates a function that would "allocate" resources that the host tracks.
    // Here this is a simple function that increments the current host memory "used".
    config.wrap_host_func("", "alloc", |caller: Caller, size: u32| -> u32 {
        if let Some(ctx) = caller.store().get::<Rc<RefCell<MemoryContext>>>() {
            let mut ctx = ctx.borrow_mut();
            let size = size as usize;

            if size + ctx.host_memory_used + ctx.wasm_memory_used <= ctx.memory_limit {
                ctx.host_memory_used += size;
                return 1;
            }

            ctx.limit_exceeded = true;
        }

        0
    });

    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"(module (import "" "alloc" (func $alloc (param i32) (result i32))) (memory (export "m") 0) (func (export "f") (param i32) (result i32) local.get 0 call $alloc))"#,
    )?;

    let dropped = Rc::new(Cell::new(false));
    let store = Store::new_with_limiter(&engine, HostMemoryLimiter(dropped.clone()));

    assert!(store
        .set(Rc::new(RefCell::new(MemoryContext {
            host_memory_used: 0,
            wasm_memory_used: 0,
            memory_limit: 1 << 20, // 16 wasm pages is the limit for both wasm + host memory
            limit_exceeded: false
        })))
        .is_ok());

    let linker = Linker::new(&store);
    let instance = linker.instantiate(&module)?;
    let memory = instance.get_memory("m").unwrap();

    // Grow the memory by 640 KiB
    memory.grow(3)?;
    memory.grow(5)?;
    memory.grow(2)?;

    assert!(
        !store
            .get::<Rc<RefCell<MemoryContext>>>()
            .unwrap()
            .borrow()
            .limit_exceeded
    );

    // Grow the host "memory" by 384 KiB
    let f = instance.get_typed_func::<u32, u32>("f")?;

    assert_eq!(f.call(1 * 0x10000).unwrap(), 1);
    assert_eq!(f.call(3 * 0x10000).unwrap(), 1);
    assert_eq!(f.call(2 * 0x10000).unwrap(), 1);

    // Memory is at the maximum, but the limit hasn't been exceeded
    assert!(
        !store
            .get::<Rc<RefCell<MemoryContext>>>()
            .unwrap()
            .borrow()
            .limit_exceeded
    );

    // Try to grow the memory again
    assert_eq!(
        memory.grow(1).map_err(|e| e.to_string()).unwrap_err(),
        "failed to grow memory by `1`"
    );

    assert!(
        store
            .get::<Rc<RefCell<MemoryContext>>>()
            .unwrap()
            .borrow()
            .limit_exceeded
    );

    // Try to grow the host "memory" again
    assert_eq!(f.call(1).unwrap(), 0);

    assert!(
        store
            .get::<Rc<RefCell<MemoryContext>>>()
            .unwrap()
            .borrow()
            .limit_exceeded
    );

    drop(f);
    drop(memory);
    drop(instance);
    drop(linker);
    drop(store);

    assert!(dropped.get());

    Ok(())
}
