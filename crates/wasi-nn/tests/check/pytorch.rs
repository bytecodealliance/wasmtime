use super::{artifacts_dir, DOWNLOAD_LOCK};
use anyhow::Result;
use std::{env, fs};

pub fn are_artifacts_available() -> Result<()> {
    let _exclusively_retrieve_artifacts = DOWNLOAD_LOCK.lock().unwrap();
    let artifacts_dir = artifacts_dir();
    if !artifacts_dir.is_dir() {
        fs::create_dir(&artifacts_dir)?;
    }

    // Copy preprocessed image tensor from source tree to artifact directory.
    let image_path = env::current_dir()?
        .join("tests")
        .join("fixtures")
        .join("kitten.tensor");
    let dest_path = artifacts_dir.join("kitten.tensor");
    fs::copy(&image_path, &dest_path)?;

    // Copy Resnet18 model from source tree to artifact directory.
    let image_path = env::current_dir()?
        .join("tests")
        .join("fixtures")
        .join("resnet.pt");
    let dest_path = artifacts_dir.join("model.pt");
    fs::copy(&image_path, &dest_path)?;

    Ok(())
}
