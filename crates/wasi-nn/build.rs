//! This build script:
//!  - has the configuration necessary for the wiggle and witx macros.
use std::fs;
use std::path::Path;



fn main() {
    // Check if there's a bak file, if so move it back.
    if Path::new("spec/phases/ephemeral/witx/wasi_ephemeral_nn.witx.bak").exists() {
        fs::rename("spec/phases/ephemeral/witx/wasi_ephemeral_nn.witx.bak", "spec/phases/ephemeral/witx/wasi_ephemeral_nn.witx").unwrap();
    }

    // If we are building with image2tensor, save the original .witx and copy the new one.
    #[cfg(feature = "i2t")]
    fs::copy("spec/phases/ephemeral/witx/wasi_ephemeral_nn.witx", "spec/phases/ephemeral/witx/wasi_ephemeral_nn.witx.bak").unwrap();
    #[cfg(feature = "i2t")]
    fs::copy("wasi_ephemeral_nn.witx", "spec/phases/ephemeral/witx/wasi_ephemeral_nn.witx").unwrap();

    // This is necessary for Wiggle/Witx macros.
    let cwd = std::env::current_dir().unwrap();
    let wasi_root = cwd.join("spec");
    println!("cargo:rustc-env=WASI_ROOT={}", wasi_root.display());

    // Also automatically rebuild if the Witx files change
    for entry in walkdir::WalkDir::new(wasi_root) {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }
}
