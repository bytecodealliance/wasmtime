extern crate cretonne_codegen;
extern crate cretonne_wasm;
extern crate tempdir;

use cretonne_codegen::print_errors::pretty_verifier_error;
use cretonne_codegen::settings::{self, Configurable, Flags};
use cretonne_codegen::verifier;
use cretonne_wasm::{translate_module, DummyEnvironment};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;
use std::str;
use tempdir::TempDir;

#[test]
fn testsuite() {
    let mut paths: Vec<_> = fs::read_dir("../../wasmtests")
        .unwrap()
        .map(|r| r.unwrap())
        .filter(|p| {
            // Ignore files starting with `.`, which could be editor temporary files
            if let Some(stem) = p.path().file_stem() {
                if let Some(stemstr) = stem.to_str() {
                    return !stemstr.starts_with('.');
                }
            }
            false
        })
        .collect();
    paths.sort_by_key(|dir| dir.path());
    let flags = Flags::new(settings::builder());
    for path in paths {
        let path = path.path();
        handle_module(&path, &flags);
    }
}

#[test]
fn return_at_end() {
    let mut flag_builder = settings::builder();
    flag_builder.enable("return_at_end").unwrap();
    let flags = Flags::new(flag_builder);
    handle_module(&PathBuf::from("../../wasmtests/return_at_end.wat"), &flags);
}

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn handle_module(path: &PathBuf, flags: &Flags) {
    let data = match path.extension() {
        None => {
            panic!("the file extension is not wasm or wat");
        }
        Some(ext) => {
            match ext.to_str() {
                Some("wasm") => read_wasm_file(path.clone()).expect("error reading wasm file"),
                Some("wat") => {
                    let tmp_dir = TempDir::new("cretonne-wasm").unwrap();
                    let file_path = tmp_dir.path().join("module.wasm");
                    File::create(file_path.clone()).unwrap();
                    let result_output = Command::new("wat2wasm")
                        .arg(path.clone())
                        .arg("-o")
                        .arg(file_path.to_str().unwrap())
                        .output();
                    match result_output {
                        Err(e) => {
                            if e.kind() == io::ErrorKind::NotFound {
                                println!(
                                    "wat2wasm not found; disabled test {}",
                                    path.to_str().unwrap()
                                );
                                return;
                            }
                            panic!("error convering wat file: {}", e.description());
                        }
                        Ok(output) => {
                            if !output.status.success() {
                                panic!(
                                    "error running wat2wasm: {}",
                                    str::from_utf8(&output.stderr).expect(
                                        "wat2wasm's error message should be valid UTF-8",
                                    )
                                );
                            }
                        }
                    }
                    read_wasm_file(file_path).expect("error reading converted wasm file")
                }
                None | Some(&_) => panic!("the file extension for {:?} is not wasm or wat", path),
            }
        }
    };
    let mut dummy_environ = DummyEnvironment::with_flags(flags.clone());
    translate_module(&data, &mut dummy_environ).unwrap();
    for func in &dummy_environ.info.function_bodies {
        verifier::verify_function(func, flags)
            .map_err(|err| panic!(pretty_verifier_error(func, None, &err)))
            .unwrap();
    }
}
