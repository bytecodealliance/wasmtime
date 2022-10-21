//! Winch CLI tool, meant mostly for testing purposes.
//!
//! Reads Wasm in binary/text format and compiles them
//! to any of the supported architectures using Winch.

use anyhow::{Context, Result};
use clap::Parser;
use std::{fs, path::PathBuf, str::FromStr};
use target_lexicon::Triple;
use wasmtime_environ::{
    wasmparser::{FuncType, Parser as WasmParser, ValType, Validator},
    DefinedFuncIndex, FunctionBodyData, Module, ModuleEnvironment, ModuleTypes, Tunables,
};
use winch_codegen::isa::{self, TargetIsa};

#[derive(Parser, Debug)]
struct Options {
    /// The input file
    input: PathBuf,

    /// The target architecture
    #[clap(long = "target")]
    target: String,
}

fn main() -> Result<()> {
    let opt = Options::from_args();
    let bytes = fs::read(&opt.input)
        .with_context(|| format!("Failed to read input file {}", opt.input.display()))?;
    let bytes = wat::parse_bytes(&bytes)?;
    let triple = Triple::from_str(&opt.target)?;
    let isa = isa::lookup(triple)?;
    let mut validator = Validator::new();
    let parser = WasmParser::new(0);
    let mut types = Default::default();
    let tunables = Tunables::default();
    let translation = ModuleEnvironment::new(&tunables, &mut validator, &mut types)
        .translate(parser, &bytes)
        .context("Failed to translate WebAssembly module")?;
    let types = types.finish();

    let module = translation.module;

    translation
        .function_body_inputs
        .into_iter()
        .try_for_each(|func| compile(&*isa, &module, &types, func))?;

    Ok(())
}

fn compile(
    isa: &dyn TargetIsa,
    module: &Module,
    types: &ModuleTypes,
    f: (DefinedFuncIndex, FunctionBodyData<'_>),
) -> Result<()> {
    let index = module.func_index(f.0);
    let func = &module.functions[index];
    let sig = &types[func.signature];
    // The following construction of a wasmparser::FuncType
    // is temporary. This should be replaced by a query to
    // `wasmparser::types::Types::function_at` which will give us
    // an equivalent functionality.
    // There's a change that is needed in wasmparser and wasmtime_environ to
    // enable this. Once I've landed such change, this will be replaced.
    let params: Vec<ValType> = sig
        .params()
        .iter()
        .copied()
        .map(wasmparser::ValType::from)
        .collect();
    let returns: Vec<ValType> = sig
        .returns()
        .iter()
        .copied()
        .map(wasmparser::ValType::from)
        .collect();
    let sig = FuncType::new(params, returns);
    let sig = sig.clone();
    let FunctionBodyData { body, validator } = f.1;
    let validator = validator.into_validator(Default::default());
    let buffer = isa
        .compile_function(&sig, body, validator)
        .expect("Couldn't compile function");
    for i in buffer {
        println!("{}", i);
    }

    Ok(())
}
