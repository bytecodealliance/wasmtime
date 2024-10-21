use super::{artifacts_dir, download, DOWNLOAD_LOCK};
use anyhow::{bail, Context, Result};
use std::{env, fs};

/// Return `Ok` if we find the cached MobileNet test artifacts; this will
/// download the artifacts if necessary.
pub fn are_artifacts_available() -> Result<()> {
    let _exclusively_retrieve_artifacts = DOWNLOAD_LOCK.lock().unwrap();
    const PYTORCH_BASE_URL: &str =
        "https://github.com/rahulchaphalkar/libtorch-models/releases/download/v0.1/squeezenet1_1.pt";
    let artifacts_dir = artifacts_dir();
    if !artifacts_dir.is_dir() {
        fs::create_dir(&artifacts_dir)?;
    }

    let local_path = artifacts_dir.join("model.pt");
    let remote_url = PYTORCH_BASE_URL;
    if !local_path.is_file() {
        download(&remote_url, &local_path).with_context(|| "unable to retrieve test artifact")?;
    } else {
        println!("> using cached artifact: {}", local_path.display())
    }

    // Copy image from source tree to artifact directory.
    let image_path = env::current_dir()?
        .join("tests")
        .join("fixtures")
        .join("kitten.tensor");
    let dest_path = artifacts_dir.join("kitten.tensor");
    fs::copy(&image_path, &dest_path)?;
    Ok(())
}
