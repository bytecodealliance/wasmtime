use wasmtime::{Engine, Linker, Module, Store, Val};

// from_witx invocation says the func is async. This context doesn't support async!
wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/tests/atoms.witx"],
    async: {
        atoms::{double_int_return_float}
    }
});

pub mod integration {
    //  The integration invocation says the func is blocking, so it will still work.
    wiggle::wasmtime_integration!({
        target: crate,
        witx: ["$CARGO_MANIFEST_DIR/tests/atoms.witx"],
        block_on: {
            atoms::{double_int_return_float}
        }
    });
}

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
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    integration::add_atoms_to_linker(&mut linker, |cx| cx).unwrap();
    let mut store = store(&engine);
    let shim_mod = shim_module(&engine);
    let shim_inst = linker.instantiate(&mut store, &shim_mod).unwrap();

    let mut results = [Val::I32(0)];
    shim_inst
        .get_func(&mut store, "int_float_args_shim")
        .unwrap()
        .call(&mut store, &[0i32.into(), 123.45f32.into()], &mut results)
        .unwrap();

    assert_eq!(
        results[0].unwrap_i32(),
        types::Errno::Ok as i32,
        "int_float_args errno"
    );
}

#[test]
fn test_async_host_func() {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    integration::add_atoms_to_linker(&mut linker, |cx| cx).unwrap();
    let mut store = store(&engine);

    let shim_mod = shim_module(&engine);
    let shim_inst = linker.instantiate(&mut store, &shim_mod).unwrap();

    let input: i32 = 123;
    let result_location: i32 = 0;

    let mut results = [Val::I32(0)];
    shim_inst
        .get_func(&mut store, "double_int_return_float_shim")
        .unwrap()
        .call(
            &mut store,
            &[input.into(), result_location.into()],
            &mut results,
        )
        .unwrap();

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

fn store(engine: &Engine) -> Store<Ctx> {
    Store::new(engine, Ctx)
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
