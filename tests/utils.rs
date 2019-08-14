use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::{Component, Path};
use std::time::SystemTime;

pub fn read_wasm<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, String> {
    let data = fs::read(path).map_err(|err| err.to_string())?;
    if data.starts_with(&[b'\0', b'a', b's', b'm']) {
        Ok(data)
    } else {
        Err("Invalid Wasm file encountered".to_owned())
    }
}

pub fn prepare_workspace<S: AsRef<str>>(exe_name: S) -> Result<String, String> {
    let mut workspace = env::temp_dir();
    let time_now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|err| err.to_string())?;
    let subdir = format!(
        "wasi_common_tests_{}_{}",
        exe_name.as_ref(),
        time_now.as_secs()
    );
    workspace.push(subdir);
    fs::create_dir(workspace.as_path()).map_err(|err| err.to_string())?;

    Ok(workspace
        .as_os_str()
        .to_str()
        .ok_or("couldn't convert to str".to_owned())?
        .to_string())
}

pub fn extract_exec_name_from_path<P: AsRef<Path>>(path: P) -> Result<String, String> {
    Ok(path
        .as_ref()
        .components()
        .next_back()
        .map(Component::as_os_str)
        .and_then(OsStr::to_str)
        .ok_or("couldn't convert to str".to_owned())?
        .to_owned())
}
