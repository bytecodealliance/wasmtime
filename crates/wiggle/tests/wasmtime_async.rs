use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use wasmtime::{Config, Engine, Linker, Module, Store};

wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/tests/atoms.witx"],
    async: {
        atoms::{double_int_return_float}
    }
});

pub struct Ctx;
impl wiggle::GuestErrorType for types::Errno {
    fn success() -> Self {
        types::Errno::Ok
    }
}

#[wiggle::async_trait]
impl atoms::Atoms for Ctx {
    fn int_float_args(&mut self, an_int: u32, an_float: f32) -> Result<(), types::Errno> {
        println!("INT FLOAT ARGS: {} {}", an_int, an_float);
        Ok(())
    }
    async fn double_int_return_float(
        &mut self,
        an_int: u32,
    ) -> Result<types::AliasToFloat, types::Errno> {
        Ok((an_int as f32) * 2.0)
    }
}

#[test]
fn test_sync_host_func() {
    let mut store = async_store();
    let mut linker = Linker::new(store.engine());
    atoms::add_to_linker(&mut linker, |cx| cx).unwrap();
    let shim_mod = shim_module(linker.engine());
    let shim_inst = run(linker.instantiate_async(&mut store, &shim_mod)).unwrap();

    let results = run(shim_inst
        .get_func(&mut store, "int_float_args_shim")
        .unwrap()
        .call_async(&mut store, &[0i32.into(), 123.45f32.into()]))
    .unwrap();

    assert_eq!(results.len(), 1, "one return value");
    assert_eq!(
        results[0].unwrap_i32(),
        types::Errno::Ok as i32,
        "int_float_args errno"
    );
}

#[test]
fn test_async_host_func() {
    let mut store = async_store();
    let mut linker = Linker::new(store.engine());
    atoms::add_to_linker(&mut linker, |cx| cx).unwrap();

    let shim_mod = shim_module(linker.engine());
    let shim_inst = run(linker.instantiate_async(&mut store, &shim_mod)).unwrap();

    let input: i32 = 123;
    let result_location: i32 = 0;

    let results = run(shim_inst
        .get_func(&mut store, "double_int_return_float_shim")
        .unwrap()
        .call_async(&mut store, &[input.into(), result_location.into()]))
    .unwrap();

    assert_eq!(results.len(), 1, "one return value");
    assert_eq!(
        results[0].unwrap_i32(),
        types::Errno::Ok as i32,
        "double_int_return_float errno"
    );

    // The actual result is in memory:
    let mem = shim_inst.get_memory(&mut store, "memory").unwrap();
    let mut result_bytes: [u8; 4] = [0, 0, 0, 0];
    mem.read(&store, result_location as usize, &mut result_bytes)
        .unwrap();
    let result = f32::from_le_bytes(result_bytes);
    assert_eq!((input * 2) as f32, result);
}

fn run<F: Future>(future: F) -> F::Output {
    let mut f = Pin::from(Box::new(future));
    let waker = dummy_waker();
    let mut cx = Context::from_waker(&waker);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(val) => break val,
            Poll::Pending => {}
        }
    }
}

fn dummy_waker() -> Waker {
    return unsafe { Waker::from_raw(clone(5 as *const _)) };

    unsafe fn clone(ptr: *const ()) -> RawWaker {
        assert_eq!(ptr as usize, 5);
        const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
        RawWaker::new(ptr, &VTABLE)
    }

    unsafe fn wake(ptr: *const ()) {
        assert_eq!(ptr as usize, 5);
    }

    unsafe fn wake_by_ref(ptr: *const ()) {
        assert_eq!(ptr as usize, 5);
    }

    unsafe fn drop(ptr: *const ()) {
        assert_eq!(ptr as usize, 5);
    }
}

fn async_store() -> Store<Ctx> {
    Store::new(
        &Engine::new(Config::new().async_support(true)).unwrap(),
        Ctx,
    )
}

// Wiggle expects the caller to have an exported memory. Wasmtime can only
// provide this if the caller is a WebAssembly module, so we need to write
// a shim module:
fn shim_module(engine: &Engine) -> Module {
    Module::new(
        engine,
        r#"
        (module
            (import "atoms" "int_float_args" (func $int_float_args (param i32 f32) (result i32)))
            (import "atoms" "double_int_return_float" (func $double_int_return_float (param i32 i32) (result i32)))

            (memory 1)
            (export "memory" (memory 0))

            (func $int_float_args_shim (param i32 f32) (result i32)
                local.get 0
                local.get 1
                call $int_float_args
            )
            (func $double_int_return_float_shim (param i32 i32) (result i32)
                local.get 0
                local.get 1
                call $double_int_return_float
            )
            (export "int_float_args_shim" (func $int_float_args_shim))
            (export "double_int_return_float_shim" (func $double_int_return_float_shim))
        )
        "#,
    )
    .unwrap()
}
