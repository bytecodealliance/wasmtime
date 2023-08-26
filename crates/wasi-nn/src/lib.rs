mod ctx;
mod registry;

pub mod backend;
pub use ctx::{preload, WasiNnCtx};
pub use registry::{GraphRegistry, InMemoryRegistry};
pub mod wit;
pub mod witx;

use std::sync::Arc;

/// A machine learning backend.
pub struct Backend(Box<dyn backend::BackendInner>);
impl std::ops::Deref for Backend {
    type Target = dyn backend::BackendInner;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for Backend {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
impl<T: backend::BackendInner + 'static> From<T> for Backend {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

/// A backend-defined graph (i.e., ML model).
#[derive(Clone)]
pub struct Graph(Arc<dyn backend::BackendGraph>);
impl From<Box<dyn backend::BackendGraph>> for Graph {
    fn from(value: Box<dyn backend::BackendGraph>) -> Self {
        Self(value.into())
    }
}
impl std::ops::Deref for Graph {
    type Target = dyn backend::BackendGraph;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

/// A backend-defined execution context.
pub struct ExecutionContext(Box<dyn backend::BackendExecutionContext>);
impl From<Box<dyn backend::BackendExecutionContext>> for ExecutionContext {
    fn from(value: Box<dyn backend::BackendExecutionContext>) -> Self {
        Self(value)
    }
}
impl std::ops::Deref for ExecutionContext {
    type Target = dyn backend::BackendExecutionContext;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for ExecutionContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

/// A container for graphs.
pub struct Registry(Box<dyn GraphRegistry>);
impl std::ops::Deref for Registry {
    type Target = dyn GraphRegistry;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for Registry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
impl<T> From<T> for Registry
where
    T: GraphRegistry + 'static,
{
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

/// For testing, this module checks:
/// - that OpenVINO can be found in the environment
/// - that some ML model artifacts can be downloaded and cached.
#[cfg(feature = "test-check")]
pub mod test_check {
    use anyhow::{anyhow, Context, Result};
    use std::{env, fs, path::Path, path::PathBuf, process::Command};

    /// Return the directory in which the test artifacts are stored.
    pub fn artifacts_dir() -> PathBuf {
        PathBuf::from(env!("OUT_DIR")).join("mobilenet")
    }

    /// Early-return from a test if the test environment is not met. If the `CI`
    /// or `FORCE_WASINN_TEST_CHECK` environment variables are set, though, this
    /// will return an error instead.
    #[macro_export]
    macro_rules! test_check {
        () => {
            if let Err(e) = $crate::test_check::check() {
                if std::env::var_os("CI").is_some()
                    || std::env::var_os("FORCE_WASINN_TEST_CHECK").is_some()
                {
                    return Err(e);
                } else {
                    println!("> ignoring test: {}", e);
                    return Ok(());
                }
            }
        };
    }

    /// Return `Ok` if all checks pass.
    pub fn check() -> Result<()> {
        check_openvino_is_installed()?;
        check_openvino_artifacts_are_available()?;
        Ok(())
    }

    /// Return `Ok` if we find a working OpenVINO installation.
    fn check_openvino_is_installed() -> Result<()> {
        match std::panic::catch_unwind(|| {
            println!("> found openvino version: {}", openvino::version())
        }) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("unable to find an OpenVINO installation: {:?}", e)),
        }
    }

    /// Return `Ok` if we find the cached MobileNet test artifacts; this will
    /// download the artifacts if necessary.
    fn check_openvino_artifacts_are_available() -> Result<()> {
        const BASE_URL: &str = "https://github.com/intel/openvino-rs/raw/main/crates/openvino/tests/fixtures/mobilenet";
        let artifacts_dir = artifacts_dir();
        if !artifacts_dir.is_dir() {
            fs::create_dir(&artifacts_dir)?;
        }
        for (from, to) in [
            ("mobilenet.bin", "model.bin"),
            ("mobilenet.xml", "model.xml"),
            ("tensor-1x224x224x3-f32.bgr", "tensor.bgr"),
        ] {
            let remote_url = [BASE_URL, from].join("/");
            let local_path = artifacts_dir.join(to);
            if !local_path.is_file() {
                download(&remote_url, &local_path)
                    .with_context(|| "unable to retrieve test artifact")?;
            } else {
                println!("> using cached artifact: {}", local_path.display())
            }
        }
        Ok(())
    }

    /// Retrieve the bytes at the `from` URL and place them in the `to` file.
    fn download(from: &str, to: &Path) -> anyhow::Result<()> {
        let mut curl = Command::new("curl");
        curl.arg("--location").arg(from).arg("--output").arg(to);
        println!("> downloading: {:?}", &curl);
        let result = curl.output().unwrap();
        if !result.status.success() {
            panic!(
                "curl failed: {}\n{}",
                result.status,
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Ok(())
    }

    /// Build the given crate as `wasm32-wasi` and return the path to the built
    /// module.
    pub fn cargo_build(crate_dir: impl AsRef<Path>) -> PathBuf {
        let crate_dir = crate_dir.as_ref();
        let crate_name = crate_dir.file_name().unwrap().to_str().unwrap();
        let cargo_toml = crate_dir.join("Cargo.toml");
        let wasm = crate_dir.join(format!("target/wasm32-wasi/release/{}.wasm", crate_name));
        let result = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("--target=wasm32-wasi")
            .arg("--manifest-path")
            .arg(cargo_toml)
            .output()
            .unwrap();
        if !wasm.is_file() {
            panic!("no file found at: {}", wasm.display());
        }
        if !result.status.success() {
            panic!(
                "cargo build failed: {}\n{}",
                result.status,
                String::from_utf8_lossy(&result.stderr)
            );
        }
        wasm
    }
}

#[cfg(all(test, not(feature = "test-check")))]
compile_error!(
    "to run wasi-nn tests we need to enable a feature: `cargo test --features test-check`"
);
