extern crate cton_wasm;
extern crate cretonne;
extern crate tempdir;

use cton_wasm::{translate_module, DummyRuntime, WasmRuntime};
use std::path::PathBuf;
use std::borrow::Borrow;
use std::fs::File;
use std::error::Error;
use std::io;
use std::str;
use std::io::BufReader;
use std::io::prelude::*;
use std::process::Command;
use std::fs;
use cretonne::ir;
use cretonne::ir::entities::AnyEntity;
use cretonne::isa::{self, TargetIsa};
use cretonne::settings::{self, Configurable};
use cretonne::verifier;
use tempdir::TempDir;

#[test]
fn testsuite() {
    let mut paths: Vec<_> = fs::read_dir("../../wasmtests")
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    paths.sort_by_key(|dir| dir.path());
    for path in paths {
        let path = path.path();
        handle_module(path, None);
    }
}

#[test]
fn return_at_end() {
    let mut flag_builder = settings::builder();
    flag_builder.enable("return_at_end").unwrap();
    let flags = settings::Flags::new(&flag_builder);
    // We don't care about the target itself here, so just pick one arbitrarily.
    let isa = match isa::lookup("riscv") {
        Err(_) => {
            println!("riscv target not found; disabled test return_at_end.wat");
            return;
        }
        Ok(isa_builder) => isa_builder.finish(flags),
    };
    handle_module(
        PathBuf::from("../../wasmtests/return_at_end.wat"),
        Some(isa.borrow()),
    );
}

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    buf_reader.read_to_end(&mut buf)?;
    Ok(buf)
}

fn handle_module(path: PathBuf, isa: Option<&TargetIsa>) {
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
                None | Some(&_) => panic!("the file extension is not wasm or wat"),
            }
        }
    };
    let mut dummy_runtime = match isa {
        Some(isa) => DummyRuntime::with_flags(isa.flags().clone()),
        None => DummyRuntime::default(),
    };
    let translation = {
        let runtime: &mut WasmRuntime = &mut dummy_runtime;
        translate_module(&data, runtime).unwrap()
    };
    for func in &translation.functions {
        verifier::verify_function(func, isa)
            .map_err(|err| panic!(pretty_verifier_error(func, isa, err)))
            .unwrap();
    }
}


/// Pretty-print a verifier error.
pub fn pretty_verifier_error(
    func: &ir::Function,
    isa: Option<&TargetIsa>,
    err: verifier::Error,
) -> String {
    let msg = err.to_string();
    let str1 = match err.location {
        AnyEntity::Inst(inst) => {
            format!(
                "{}\n{}: {}\n\n",
                msg,
                inst,
                func.dfg.display_inst(inst, isa)
            )
        }
        _ => String::from(format!("{}\n", msg)),
    };
    format!("{}{}", str1, func.display(isa))
}
