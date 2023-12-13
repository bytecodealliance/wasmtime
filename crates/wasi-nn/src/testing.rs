//! This is testing-specific code--it is public only so that it can be
//! accessible both in unit and integration tests.
//!
//! This module checks:
//! - that OpenVINO can be found in the environment
//! - that some ML model artifacts can be downloaded and cached.

use anyhow::{anyhow, Context, Result};
use std::{env, fs, path::Path, path::PathBuf, process::Command, sync::Mutex};

/// Return the directory in which the test artifacts are stored.
pub fn artifacts_dir() -> PathBuf {
    PathBuf::from(env!("OUT_DIR")).join("mobilenet")
}

/// Early-return from a test if the test environment is not met. If the `CI`
/// or `FORCE_WASINN_TEST_CHECK` environment variables are set, though, this
/// will return an error instead.
#[macro_export]
macro_rules! check_test {
    () => {
        if let Err(e) = $crate::testing::check() {
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
    match std::panic::catch_unwind(|| println!("> found openvino version: {}", openvino::version()))
    {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow!("unable to find an OpenVINO installation: {:?}", e)),
    }
}

/// Protect `check_openvino_artifacts_are_available` from concurrent access;
/// when running tests in parallel, we want to avoid two threads attempting to
/// create the same directory or download the same file.
static ARTIFACTS: Mutex<()> = Mutex::new(());

/// Return `Ok` if we find the cached MobileNet test artifacts; this will
/// download the artifacts if necessary.
fn check_openvino_artifacts_are_available() -> Result<()> {
    let _exclusively_retrieve_artifacts = ARTIFACTS.lock().unwrap();
    const BASE_URL: &str =
        "https://github.com/intel/openvino-rs/raw/main/crates/openvino/tests/fixtures/mobilenet";
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
