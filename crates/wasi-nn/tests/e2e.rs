//! Embed wasi-nn in Wasmtime and compute an inference by:
//! - downloading any necessary model artifacts (`test_check!`)
//! - setting up a wasi + wasi-nn environment
//! - build an `example` crate into a `*.wasm` file
//! - run the `*.wasm` file.

use anyhow::Result;
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::{ambient_authority, Dir, WasiCtx, WasiCtxBuilder};
use wasmtime_wasi_nn::{backend, InMemoryRegistry, WasiNnCtx};

#[test]
fn image_classification() -> Result<()> {
    wasmtime_wasi_nn::test_check!();

    // Set up a WASI environment that includes wasi-nn and opens the MobileNet
    // artifacts directory as `fixture` in the guest.
    let engine = Engine::default();
    let (mut store, mut linker) = embed_wasi_nn(&engine, WasiNnCtx::default())?;

    // Build and run the example crate.
    let wasm_file = wasmtime_wasi_nn::test_check::cargo_build("examples/image-classification");
    let module = Module::from_file(&engine, wasm_file)?;
    linker.module(&mut store, "", &module)?;
    linker
        .get_default(&mut store, "")?
        .typed::<(), ()>(&store)?
        .call(&mut store, ())?;

    Ok(())
}

#[test]
fn image_classification_with_names() -> Result<()> {
    wasmtime_wasi_nn::test_check!();

    // Set up a WASI environment that includes wasi-nn and uses a registry with
    // the "mobilenet" name populated.
    let engine = Engine::default();
    let mut openvino = backend::openvino::OpenvinoBackend::default();
    let mut registry = InMemoryRegistry::new();
    let mobilenet_dir = wasmtime_wasi_nn::test_check::artifacts_dir();
    registry.load(&mut openvino, &mobilenet_dir)?;
    let wasi_nn = WasiNnCtx::new([openvino.into()], registry.into());
    let (mut store, mut linker) = embed_wasi_nn(&engine, wasi_nn)?;

    // Build and run the example crate.
    let wasm_file =
        wasmtime_wasi_nn::test_check::cargo_build("examples/image-classification-named");
    let module = Module::from_file(&engine, wasm_file)?;
    linker.module(&mut store, "", &module)?;
    linker
        .get_default(&mut store, "")?
        .typed::<(), ()>(&store)?
        .call(&mut store, ())?;

    Ok(())
}

struct Host {
    wasi: WasiCtx,
    wasi_nn: WasiNnCtx,
}

fn embed_wasi_nn(engine: &Engine, wasi_nn: WasiNnCtx) -> Result<(Store<Host>, Linker<Host>)> {
    let mut linker = Linker::new(&engine);
    let host_dir = Dir::open_ambient_dir(
        wasmtime_wasi_nn::test_check::artifacts_dir(),
        ambient_authority(),
    )?;
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .preopened_dir(host_dir, "fixture")?
        .build();
    let store = Store::<Host>::new(&engine, Host { wasi, wasi_nn });
    wasmtime_wasi_nn::witx::add_to_linker(&mut linker, |s: &mut Host| &mut s.wasi_nn)?;
    wasmtime_wasi::add_to_linker(&mut linker, |s: &mut Host| &mut s.wasi)?;
    Ok((store, linker))
}
