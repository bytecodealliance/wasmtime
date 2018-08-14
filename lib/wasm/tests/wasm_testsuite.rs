extern crate cranelift_codegen;
extern crate cranelift_wasm;
#[macro_use]
extern crate target_lexicon;
extern crate wabt;

use cranelift_codegen::print_errors::pretty_verifier_error;
use cranelift_codegen::settings::{self, Configurable, Flags};
use cranelift_codegen::verifier;
use cranelift_wasm::{translate_module, DummyEnvironment};
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::str::FromStr;
use wabt::wat2wasm;

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
    handle_module(Path::new("../../wasmtests/return_at_end.wat"), &flags);
}

fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn handle_module(path: &Path, flags: &Flags) {
    let data = match path.extension() {
        None => {
            panic!("the file extension is not wasm or wat");
        }
        Some(ext) => match ext.to_str() {
            Some("wasm") => read_file(path).expect("error reading wasm file"),
            Some("wat") => {
                let wat = read_file(path).expect("error reading wat file");
                match wat2wasm(&wat) {
                    Ok(wasm) => wasm,
                    Err(e) => {
                        panic!("error converting wat to wasm: {:?}", e);
                    }
                }
            }
            None | Some(&_) => panic!("the file extension for {:?} is not wasm or wat", path),
        },
    };
    let mut dummy_environ = DummyEnvironment::with_triple_flags(triple!("riscv64"), flags.clone());
    translate_module(&data, &mut dummy_environ).unwrap();
    for func in dummy_environ.info.function_bodies.values() {
        verifier::verify_function(func, flags)
            .map_err(|errors| panic!(pretty_verifier_error(func, None, None, errors)))
            .unwrap();
    }
}
