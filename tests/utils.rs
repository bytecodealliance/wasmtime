use std::fs;
use std::path::Path;
use tempfile::{Builder, TempDir};

pub fn read_wasm(path: &Path) -> Result<Vec<u8>, String> {
    let data = fs::read(path).map_err(|err| err.to_string())?;
    if data.starts_with(&[b'\0', b'a', b's', b'm']) {
        Ok(data)
    } else {
        Err("Invalid Wasm file encountered".to_owned())
    }
}

pub fn prepare_workspace(exe_name: &str) -> Result<TempDir, String> {
    let prefix = format!("wasi_common_{}", exe_name);
    Builder::new()
        .prefix(&prefix)
        .tempdir()
        .map_err(|e| format!("couldn't create workspace in temp files: {}", e))
}

pub fn extract_exec_name_from_path(path: &Path) -> Result<String, String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(String::from)
        .ok_or(format!(
            "couldn't extract the file stem from path {}",
            path.display()
        ))
}
