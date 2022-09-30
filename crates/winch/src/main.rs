//! Winch CLI tool, meant mostly for testing purposes
//!
//! Reads Wasm in binary/text format and compiles them using Winch

use anyhow::{Context, Result};
use std::{fs, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use target_lexicon::Triple;
use wasmparser::WasmFuncTypeInputs;
use wasmtime_environ::{
    wasmparser::{Parser, Validator},
    DefinedFuncIndex, FunctionBodyData, Module, ModuleEnvironment, ModuleTypes, Tunables,
};
use winch::isa::{self, TargetIsa};

#[derive(StructOpt, Debug)]
struct Options {
    /// The input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// The target architecture
    #[structopt(long = "target")]
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
    let parser = Parser::new(0);
    let mut types = Default::default();
    let tunables = Tunables::default();
    let mut translation = ModuleEnvironment::new(&tunables, &mut validator, &mut types)
        .translate(parser, &bytes)
        .context("Failed to translate WebAssembly module")?;
    let types = types.finish();
    let module = translation.module;

    translation
        .function_body_inputs
        .iter_mut()
        .for_each(|func| compile(&*isa, &module, &types, func));

    Ok(())
}

fn compile(
    isa: &dyn TargetIsa,
    module: &Module,
    types: &ModuleTypes,
    f: (DefinedFuncIndex, &mut FunctionBodyData<'_>),
) {
    let index = module.func_index(f.0);
    let func = &module.functions[index];
    let sig = &types[func.signature];
    let buffer = isa
        .compile_function(sig, f.1)
        .expect("Couldn't compile function");
    for i in buffer {
        println!("{}", i);
    }
}
