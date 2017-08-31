extern crate cretonne_wasm;
extern crate cretonne;

use cretonne_wasm::{translate_module, FunctionTranslation, DummyRuntime, WasmRuntime};
use std::path::PathBuf;
use std::fs::File;
use std::error::Error;
use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::fs;
use cretonne::ir;
use cretonne::ir::entities::AnyEntity;
use cretonne::isa::TargetIsa;
use cretonne::verifier;

#[test]
fn testsuite() {
    let mut paths: Vec<_> = fs::read_dir("../../wasmtests")
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    paths.sort_by_key(|dir| dir.path());
    for path in paths {
        let path = path.path();
        match handle_module(path) {
            Ok(()) => (),
            Err(message) => println!("{}", message),
        };
    }
}

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    buf_reader.read_to_end(&mut buf)?;
    Ok(buf)
}

fn handle_module(path: PathBuf) -> Result<(), String> {
    let data = match path.extension() {
        None => {
            return Err(String::from("the file extension is not wasm or wast"));
        }
        Some(ext) => {
            match ext.to_str() {
                Some("wasm") => {
                    match read_wasm_file(path.clone()) {
                        Ok(data) => data,
                        Err(err) => {
                            return Err(String::from(err.description()));
                        }
                    }
                }
                None | Some(&_) => {
                    return Err(String::from("the file extension is not wasm or wast"));
                }
            }
        }
    };
    let mut dummy_runtime = DummyRuntime::new();
    let translation = {
        let runtime: &mut WasmRuntime = &mut dummy_runtime;
        match translate_module(&data, runtime) {
            Ok(x) => x,
            Err(string) => {
                return Err(string);
            }
        }
    };
    for func in translation.functions {
        let il = match func {
            FunctionTranslation::Import() => continue,
            FunctionTranslation::Code { ref il, .. } => il.clone(),
        };
        match verifier::verify_function(&il, None) {
            Ok(()) => (),
            Err(err) => return Err(pretty_verifier_error(&il, None, err)),
        }
    }
    Ok(())
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
