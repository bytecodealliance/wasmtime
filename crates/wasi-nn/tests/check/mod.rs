//! Check that the environment is set up correctly for running tests.
//!
//! This module checks:
//! - that various backends can be located on the system (see sub-modules)
//! - that certain ML model artifacts can be downloaded and cached.

use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

#[cfg(any(feature = "onnx", all(feature = "winml", target_os = "windows")))]
pub mod onnx;
#[cfg(feature = "openvino")]
pub mod openvino;
#[cfg(feature = "pytorch")]
pub mod pytorch;
#[cfg(all(feature = "winml", target_os = "windows"))]
pub mod winml;

/// Protect `are_artifacts_available` from concurrent access; when running tests
/// in parallel, we want to avoid two threads attempting to create the same
/// directory or download the same file.
pub static DOWNLOAD_LOCK: Mutex<()> = Mutex::new(());

/// Return the directory in which the test artifacts are stored.
pub fn artifacts_dir() -> PathBuf {
    PathBuf::from(env!("OUT_DIR")).join("fixtures")
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
