//! Run the wasi-nn tests in `crates/test-programs`.

use anyhow::Result;
use std::path::Path;
use test_programs_artifacts::*;
use wasi_common::sync::{Dir, WasiCtxBuilder};
use wasi_common::WasiCtx;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi_nn::{backend, testing, Backend, InMemoryRegistry, WasiNnCtx};

const PREOPENED_DIR_NAME: &str = "fixture";

/// Run a wasi-nn test program. This is modeled after
/// `crates/wasi/tests/all/main.rs` but still uses the older preview1 API for
/// file reads.
fn run(path: &str, backend: Backend, preload_model: bool) -> Result<()> {
    wasmtime_wasi_nn::check_test!();
    let path = Path::new(path);
    let engine = Engine::new(&Config::new())?;
    let mut linker = Linker::new(&engine);
    wasmtime_wasi_nn::witx::add_to_linker(&mut linker, |s: &mut Ctx| &mut s.wasi_nn)?;
    wasi_common::sync::add_to_linker(&mut linker, |s: &mut Ctx| &mut s.wasi)?;
    let module = Module::from_file(&engine, path)?;
    let mut store = Store::new(
        &engine,
        Ctx::new(&testing::artifacts_dir(), preload_model, backend)?,
    );
    let instance = linker.instantiate(&mut store, &module)?;
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    start.call(&mut store, ())?;
    Ok(())
}

/// The host state for running wasi-nn  tests.
struct Ctx {
    wasi: WasiCtx,
    wasi_nn: WasiNnCtx,
}

impl Ctx {
    fn new(preopen_dir: &Path, preload_model: bool, mut backend: Backend) -> Result<Self> {
        // Create the WASI context.
        let preopen_dir = Dir::open_ambient_dir(preopen_dir, cap_std::ambient_authority())?;
        let mut builder = WasiCtxBuilder::new();
        builder
            .inherit_stdio()
            .preopened_dir(preopen_dir, PREOPENED_DIR_NAME)?;
        let wasi = builder.build();

        let mut registry = InMemoryRegistry::new();
        let mobilenet_dir = testing::artifacts_dir();
        if preload_model {
            registry.load((backend).as_dir_loadable().unwrap(), &mobilenet_dir)?;
        }
        let wasi_nn = WasiNnCtx::new([backend.into()], registry.into());

        Ok(Self { wasi, wasi_nn })
    }
}

// Check that every wasi-nn test in `crates/test-programs` has its
// manually-added `#[test]` function.
macro_rules! assert_test_exists {
    ($name:ident) => {
        #[allow(unused_imports)]
        use self::$name as _;
    };
}
foreach_nn!(assert_test_exists);

#[cfg_attr(
    not(all(
        target_arch = "x86_64",
        any(target_os = "linux", target_os = "windows")
    )),
    ignore
)]
#[test]
fn nn_image_classification() {
    let backend = Backend::from(backend::openvino::OpenvinoBackend::default());
    run(NN_IMAGE_CLASSIFICATION, backend, false).unwrap()
}

#[cfg_attr(
    not(all(
        target_arch = "x86_64",
        any(target_os = "linux", target_os = "windows")
    )),
    ignore
)]
#[test]
fn nn_image_classification_named() {
    let backend = Backend::from(backend::openvino::OpenvinoBackend::default());
    run(NN_IMAGE_CLASSIFICATION_NAMED, backend, true).unwrap()
}

#[cfg_attr(not(all(feature = "winml", target_os = "windows")), ignore)]
#[test]
fn nn_image_classification_winml() {
    #[cfg(all(feature = "winml", target_os = "windows"))]
    {
        let backend = Backend::from(backend::winml::WinMLBackend::default());
        run(NN_IMAGE_CLASSIFICATION_ONNX, backend, true).unwrap()
    }
}

#[cfg_attr(
    not(all(
        feature = "onnx",
        any(target_arch = "x86_64", target_arch = "aarch64"),
        any(target_os = "linux", target_os = "windows", target_os = "macos")
    )),
    ignore
)]
#[test]
fn nn_image_classification_onnx() {
    #[cfg(feature = "onnx")]
    {
        let backend = Backend::from(backend::onnxruntime::OnnxBackend::default());
        run(NN_IMAGE_CLASSIFICATION_ONNX, backend, false).unwrap()
    }
}
