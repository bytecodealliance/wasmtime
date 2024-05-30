//! Run the wasi-nn tests in `crates/test-programs`.

use anyhow::{anyhow, Result};
use std::path::Path;
use test_programs_artifacts::*;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::bindings::sync::Command;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder};
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
    wasmtime_wasi_nn::add_to_linker(&mut linker)?;
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;
    let module = Component::from_file(&engine, path)?;
    let mut store = Store::new(
        &engine,
        Ctx::new(&testing::artifacts_dir(), preload_model, backend)?,
    );
    let (command, _instance) = Command::instantiate(&mut store, &module, &linker)?;
    let result = command.wasi_cli_run().call_run(&mut store)?;
    result.map_err(|_| anyhow!("failed to run command"))
}

/// The host state for running wasi-nn  tests.
struct Ctx {
    wasi: WasiCtx,
    wasi_nn: WasiNnCtx,
    table: ResourceTable,
}

impl Ctx {
    fn new(preopen_dir: &Path, preload_model: bool, mut backend: Backend) -> Result<Self> {
        // Create the WASI context.
        let mut builder = WasiCtxBuilder::new();
        builder.inherit_stdio().preopened_dir(
            preopen_dir,
            PREOPENED_DIR_NAME,
            DirPerms::READ,
            FilePerms::READ,
        )?;
        let wasi = builder.build();

        let mut registry = InMemoryRegistry::new();
        let mobilenet_dir = testing::artifacts_dir();
        if preload_model {
            registry.load((backend).as_dir_loadable().unwrap(), &mobilenet_dir)?;
        }
        let wasi_nn = WasiNnCtx::new([backend.into()], registry.into());

        let table = ResourceTable::new();

        Ok(Self {
            wasi,
            wasi_nn,
            table,
        })
    }
}

impl wasmtime_wasi::WasiView for Ctx {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }

    fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
        &mut self.table
    }
}

impl wasmtime_wasi_nn::WasiNnView for Ctx {
    fn ctx(&mut self) -> &mut WasiNnCtx {
        &mut self.wasi_nn
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
    run(NN_IMAGE_CLASSIFICATION_COMPONENT, backend, false).unwrap()
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
    run(NN_IMAGE_CLASSIFICATION_NAMED_COMPONENT, backend, true).unwrap()
}

#[cfg_attr(not(all(feature = "winml", target_os = "windows")), ignore)]
#[test]
fn nn_image_classification_winml() {
    #[cfg(feature = "winml")]
    {
        let backend = Backend::from(backend::winml::WinMLBackend::default());
        run(NN_IMAGE_CLASSIFICATION_WINML_COMPONENT, backend, true).unwrap()
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
        run(NN_IMAGE_CLASSIFICATION_ONNX_COMPONENT, backend, false).unwrap()
    }
}
