//! This is testing-specific code--it is public only so that it can be
//! accessible both in unit and integration tests.
//!
//! This module checks:
//! - that OpenVINO can be found in the environment
//! - that WinML is available
//! - that some ML model artifacts can be downloaded and cached.

#[allow(unused_imports)]
use anyhow::{anyhow, Context, Result};
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(any(feature = "onnx", feature = "winml"))]
pub mod onnx;
#[cfg(feature = "openvino")]
pub mod openvino;
#[cfg(all(feature = "winml", target_os = "windows"))]
pub mod winml;

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
