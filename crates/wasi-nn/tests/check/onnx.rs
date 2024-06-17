#![allow(unused)]

use super::{artifacts_dir, download};
use anyhow::{Context, Result};
use std::sync::Mutex;
use std::{env, fs};

/// Protect `are_artifacts_available` from concurrent access; when running tests
/// in parallel, we want to avoid two threads attempting to create the same
/// directory or download the same file.
static DOWNLOAD_LOCK: Mutex<()> = Mutex::new(());

/// Return `Ok` if we find the cached MobileNet test artifacts; this will
/// download the artifacts if necessary.
pub fn are_artifacts_available() -> Result<()> {
    let _exclusively_retrieve_artifacts = DOWNLOAD_LOCK.lock().unwrap();

    const ONNX_BASE_URL: &str =
        "https://github.com/onnx/models/raw/bec48b6a70e5e9042c0badbaafefe4454e072d08/validated/vision/classification/mobilenet/model/mobilenetv2-10.onnx?download=";

    let artifacts_dir = artifacts_dir();
    if !artifacts_dir.is_dir() {
        fs::create_dir(&artifacts_dir)?;
    }

    for (from, to) in [(ONNX_BASE_URL.to_string(), "model.onnx")] {
        let local_path = artifacts_dir.join(to);
        if !local_path.is_file() {
            download(&from, &local_path).with_context(|| "unable to retrieve test artifact")?;
        } else {
            println!("> using cached artifact: {}", local_path.display())
        }
    }

    // Copy image from source tree to artifact directory.
    let image_path = env::current_dir()?
        .join("tests")
        .join("fixtures")
        .join("000000062808.rgb");
    let dest_path = artifacts_dir.join("000000062808.rgb");
    fs::copy(&image_path, &dest_path)?;
    Ok(())
}
