use anyhow::Error;
use wasmtime::*;

// Crate a synchronous Func, call it directly:
#[test]
fn call_wrapped_func() -> Result<(), Error> {
    let mut store = Store::<State>::default();
    store.entering_native_code_hook(State::entering_native);
    store.exiting_native_code_hook(State::exiting_native);
    let f = Func::wrap(
        &mut store,
        |caller: Caller<State>, a: i32, b: i64, c: f32, d: f64| {
            assert_eq!(
                caller.data().switches_into_native % 2,
                1,
                "odd number of switches into native while in a Func"
            );
            assert_eq!(a, 1);
            assert_eq!(b, 2);
            assert_eq!(c, 3.0);
            assert_eq!(d, 4.0);
        },
    );

    f.call(
        &mut store,
        &[Val::I32(1), Val::I64(2), 3.0f32.into(), 4.0f64.into()],
    )?;

    // One switch from vm to native to call f, another in return from f.
    assert_eq!(store.data().switches_into_native, 2);

    f.typed::<(i32, i64, f32, f64), (), _>(&store)?
        .call(&mut store, (1, 2, 3.0, 4.0))?;

    assert_eq!(store.data().switches_into_native, 4);

    Ok(())
}

// Create an async Func, call it directly:
#[tokio::test]
async fn call_wrapped_async_func() -> Result<(), Error> {
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, State::default());
    store.entering_native_code_hook(State::entering_native);
    store.exiting_native_code_hook(State::exiting_native);
    let f = Func::wrap4_async(
        &mut store,
        |caller: Caller<State>, a: i32, b: i64, c: f32, d: f64| {
            Box::new(async move {
                assert_eq!(
                    caller.data().switches_into_native % 2,
                    1,
                    "odd number of switches into native while in a Func"
                );
                assert_eq!(a, 1);
                assert_eq!(b, 2);
                assert_eq!(c, 3.0);
                assert_eq!(d, 4.0);
            })
        },
    );

    f.call_async(
        &mut store,
        &[Val::I32(1), Val::I64(2), 3.0f32.into(), 4.0f64.into()],
    )
    .await?;

    // One switch from vm to native to call f, another in return from f.
    assert_eq!(store.data().switches_into_native, 2);

    f.typed::<(i32, i64, f32, f64), (), _>(&store)?
        .call_async(&mut store, (1, 2, 3.0, 4.0))
        .await?;

    assert_eq!(store.data().switches_into_native, 4);

    Ok(())
}

// Use the Linker to define a sync func, call it through WebAssembly:
#[test]
fn call_linked_func() -> Result<(), Error> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, State::default());
    store.entering_native_code_hook(State::entering_native);
    store.exiting_native_code_hook(State::exiting_native);
    let mut linker = Linker::new(&engine);

    linker.func_wrap(
        "host",
        "f",
        |caller: Caller<State>, a: i32, b: i64, c: f32, d: f64| {
            assert_eq!(
                caller.data().switches_into_native % 2,
                1,
                "odd number of switches into native while in a Func"
            );
            assert_eq!(a, 1);
            assert_eq!(b, 2);
            assert_eq!(c, 3.0);
            assert_eq!(d, 4.0);
        },
    )?;

    let wat = r#"
        (module
            (import "host" "f"
                (func $f (param i32) (param i64) (param f32) (param f64)))
            (func (export "export")
                (call $f (i32.const 1) (i64.const 2) (f32.const 3.0) (f64.const 4.0)))
        )
    "#;
    let module = Module::new(&engine, wat)?;

    let inst = linker.instantiate(&mut store, &module)?;
    let export = inst
        .get_export(&mut store, "export")
        .expect("get export")
        .into_func()
        .expect("export is func");

    export.call(&mut store, &[])?;

    // One switch from vm to native to call f, another in return from f.
    assert_eq!(store.data().switches_into_native, 2);

    export.typed::<(), (), _>(&store)?.call(&mut store, ())?;

    assert_eq!(store.data().switches_into_native, 4);

    Ok(())
}

// Use the Linker to define an async func, call it through WebAssembly:
#[tokio::test]
async fn call_linked_func_async() -> Result<(), Error> {
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, State::default());
    store.entering_native_code_hook(State::entering_native);
    store.exiting_native_code_hook(State::exiting_native);

    let f = Func::wrap4_async(
        &mut store,
        |caller: Caller<State>, a: i32, b: i64, c: f32, d: f64| {
            Box::new(async move {
                assert_eq!(
                    caller.data().switches_into_native % 2,
                    1,
                    "odd number of switches into native while in a Func"
                );
                assert_eq!(a, 1);
                assert_eq!(b, 2);
                assert_eq!(c, 3.0);
                assert_eq!(d, 4.0);
            })
        },
    );

    let mut linker = Linker::new(&engine);

    linker.define("host", "f", f)?;

    let wat = r#"
        (module
            (import "host" "f"
                (func $f (param i32) (param i64) (param f32) (param f64)))
            (func (export "export")
                (call $f (i32.const 1) (i64.const 2) (f32.const 3.0) (f64.const 4.0)))
        )
    "#;
    let module = Module::new(&engine, wat)?;

    let inst = linker.instantiate_async(&mut store, &module).await?;
    let export = inst
        .get_export(&mut store, "export")
        .expect("get export")
        .into_func()
        .expect("export is func");

    export.call_async(&mut store, &[]).await?;

    // One switch from vm to native to call f, another in return from export.
    assert_eq!(store.data().switches_into_native, 2);

    export
        .typed::<(), (), _>(&store)?
        .call_async(&mut store, ())
        .await?;

    // 2 more switches.
    assert_eq!(store.data().switches_into_native, 4);

    Ok(())
}

#[test]
fn instantiate() -> Result<(), Error> {
    let mut store = Store::<State>::default();
    store.entering_native_code_hook(State::entering_native);
    store.exiting_native_code_hook(State::exiting_native);

    let m = Module::new(store.engine(), "(module)")?;
    Instance::new(&mut store, &m, &[])?;
    assert_eq!(store.data().switches_into_native, 0);

    let m = Module::new(store.engine(), "(module (func) (start 0))")?;
    Instance::new(&mut store, &m, &[])?;
    assert_eq!(store.data().switches_into_native, 1);

    Ok(())
}

#[tokio::test]
async fn instantiate_async() -> Result<(), Error> {
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, State::default());
    store.entering_native_code_hook(State::entering_native);
    store.exiting_native_code_hook(State::exiting_native);

    let m = Module::new(store.engine(), "(module)")?;
    Instance::new_async(&mut store, &m, &[]).await?;
    assert_eq!(store.data().switches_into_native, 0);

    let m = Module::new(store.engine(), "(module (func) (start 0))")?;
    Instance::new_async(&mut store, &m, &[]).await?;
    assert_eq!(store.data().switches_into_native, 1);

    Ok(())
}

enum Context {
    Native,
    Vm,
}

struct State {
    context: Context,
    switches_into_native: usize,
}

impl Default for State {
    fn default() -> Self {
        State {
            context: Context::Native,
            switches_into_native: 0,
        }
    }
}

impl State {
    fn entering_native(&mut self) -> Result<(), Trap> {
        match self.context {
            Context::Vm => {
                println!("entering native");
                self.context = Context::Native;
                self.switches_into_native += 1;
                Ok(())
            }
            Context::Native => Err(Trap::new("illegal state: exiting vm when in native")),
        }
    }
    fn exiting_native(&mut self) -> Result<(), Trap> {
        match self.context {
            Context::Native => {
                println!("entering vm");
                self.context = Context::Vm;
                Ok(())
            }
            Context::Vm => Err(Trap::new("illegal state: exiting native when in vm")),
        }
    }
}
