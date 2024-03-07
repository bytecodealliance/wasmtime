use anyhow::Result;
use wasmtime::{Engine, Instance, Linker, Module, Store};

mod allocator;

#[no_mangle]
pub unsafe extern "C" fn run(buf: *mut u8, size: usize) -> usize {
    let buf = std::slice::from_raw_parts_mut(buf, size);
    match run_result() {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("{e:?}");
            let len = buf.len().min(msg.len());
            buf[..len].copy_from_slice(&msg.as_bytes()[..len]);
            len
        }
    }
}

fn run_result() -> Result<()> {
    smoke()?;
    simple_add()?;
    simple_host_fn()?;
    Ok(())
}

fn smoke() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(&engine, "(module)")?;
    Instance::new(&mut Store::new(&engine, ()), &module, &[])?;
    Ok(())
}

fn simple_add() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"
            (module
                (func (export "add") (param i32 i32) (result i32)
                    (i32.add (local.get 0) (local.get 1)))
            )
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_typed_func::<(u32, u32), u32>(&mut store, "add")?;
    assert_eq!(func.call(&mut store, (2, 3))?, 5);
    Ok(())
}

fn simple_host_fn() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"
            (module
                (import "host" "multiply" (func $multiply (param i32 i32) (result i32)))
                (func (export "add_and_mul") (param i32 i32 i32) (result i32)
                    (i32.add (call $multiply (local.get 0) (local.get 1)) (local.get 2)))
            )
        "#,
    )?;
    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("host", "multiply", |a: u32, b: u32| a.saturating_mul(b))?;
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &module)?;
    let func = instance.get_typed_func::<(u32, u32, u32), u32>(&mut store, "add_and_mul")?;
    assert_eq!(func.call(&mut store, (2, 3, 4))?, 10);
    Ok(())
}
