mod wasi;

use cranelift_codegen::settings;
use cranelift_native;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::{Component, Path, PathBuf};
use std::process::exit;
use std::time::SystemTime;
use wasmtime_jit::Context;

fn read_to_end(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn read_wasm(path: PathBuf) -> Result<Vec<u8>, String> {
    let data = read_to_end(path).map_err(|err| err.to_string())?;
    if data.starts_with(&[b'\0', b'a', b's', b'm']) {
        Ok(data)
    } else {
        Err("Invalid Wasm file encountered".to_owned())
    }
}

fn handle_module(context: &mut Context, path: &Path) -> Result<(), String> {
    // Read the wasm module binary.
    let data = read_wasm(path.to_path_buf())?;

    // Compile and instantiating a wasm module.
    context
        .instantiate_module(None, &data)
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn prepare_workspace(exe_name: &str) -> Result<String, String> {
    let mut workspace = env::temp_dir();
    let time_now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|err| err.to_string())?;
    let subdir = format!("wasi_common_tests_{}_{}", exe_name, time_now.as_secs());
    workspace.push(subdir);
    fs::create_dir(workspace.as_path()).map_err(|err| err.to_string())?;

    Ok(workspace
        .as_os_str()
        .to_str()
        .expect("could convert to string")
        .to_string())
}

fn preopen_workspace(workspace: String) -> (String, File) {
    let preopen_dir = wasi_common::preopen_dir(&workspace).unwrap_or_else(|err| {
        println!("error while preopening directory {}: {}", workspace, err);
        exit(1);
    });
    (".".to_owned(), preopen_dir)
}

pub fn run_wasm<P: AsRef<Path>>(path: P) {
    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let flag_builder = settings::builder();
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut context = Context::with_isa(isa);

    let global_exports = context.get_global_exports();

    // extract exe name from path
    let exe_name = Path::new(path.as_ref())
        .components()
        .next_back()
        .map(Component::as_os_str)
        .and_then(OsStr::to_str)
        .unwrap_or("unknown")
        .to_owned();

    let workspace = match prepare_workspace(&exe_name) {
        Ok(workspace) => workspace,
        Err(message) => {
            println!("error while processing preopen dirs: {}", message);
            exit(1);
        }
    };
    let preopen_dirs = &[preopen_workspace(workspace)];
    let argv = vec![exe_name, ".".to_owned()];

    context.name_instance(
        "wasi_unstable".to_owned(),
        wasi::instantiate_wasi("", global_exports, preopen_dirs, &argv, &[])
            .expect("instantiating wasi"),
    );

    // Load the main wasm module.
    match handle_module(&mut context, path.as_ref()) {
        Ok(()) => {}
        Err(message) => {
            let name = path.as_ref().as_os_str().to_string_lossy();
            println!("error while processing main module {}: {}", name, message);
            exit(1);
        }
    }
}
