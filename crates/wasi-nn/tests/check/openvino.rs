use super::{artifacts_dir, download, DOWNLOAD_LOCK};
use anyhow::{bail, Context, Result};
use std::fs;

/// Return `Ok` if we find a working OpenVINO installation.
pub fn is_installed() -> Result<()> {
    match std::panic::catch_unwind(|| println!("> found openvino version: {}", openvino::version()))
    {
        Ok(_) => Ok(()),
        Err(e) => bail!(
            "unable to find an OpenVINO installation: {:?}",
            e.downcast_ref::<String>()
        ),
    }
}

/// Return `Ok` if we find the cached MobileNet test artifacts; this will
/// download the artifacts if necessary.
pub fn are_artifacts_available() -> Result<()> {
    let _exclusively_retrieve_artifacts = DOWNLOAD_LOCK.lock().unwrap();
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
