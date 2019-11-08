use std::fs;
use std::path::Path;
use tempfile::{Builder, TempDir};

pub fn read_wasm(path: &Path) -> anyhow::Result<Vec<u8>> {
    let data = fs::read(path)?;
    if data.starts_with(&[b'\0', b'a', b's', b'm']) {
        Ok(data)
    } else {
        anyhow::bail!("Invalid Wasm file encountered")
    }
}

pub fn prepare_workspace(exe_name: &str) -> anyhow::Result<TempDir> {
    let prefix = format!("wasi_common_{}", exe_name);
    let tempdir = Builder::new().prefix(&prefix).tempdir()?;
    Ok(tempdir)
}

pub fn extract_exec_name_from_path(path: &Path) -> anyhow::Result<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(String::from)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "couldn't extract the file stem from path {}",
                path.display()
            )
        })
}
