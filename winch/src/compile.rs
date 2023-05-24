use anyhow::{Context, Result};
use clap::Parser;
use cranelift_codegen::settings;
use std::{fs, path::PathBuf, str::FromStr};
use target_lexicon::Triple;
use wasmtime_environ::{
    wasmparser::{Parser as WasmParser, Validator},
    DefinedFuncIndex, FunctionBodyData, ModuleEnvironment, Tunables, TypeConvert, VMOffsets,
};
use winch_codegen::{lookup, TargetIsa};
use winch_environ::FuncEnv;
use winch_filetests::disasm::disasm;

#[derive(Parser, Debug)]
pub struct Options {
    /// The input file.
    input: PathBuf,

    /// The target architecture.
    #[clap(long = "target")]
    target: String,
}

pub fn run(opt: &Options) -> Result<()> {
    let bytes = fs::read(&opt.input)
        .with_context(|| format!("Failed to read input file {}", opt.input.display()))?;
    let bytes = wat::parse_bytes(&bytes)?;
    let triple = Triple::from_str(&opt.target)?;
    let shared_flags = settings::Flags::new(settings::builder());
    let isa_builder = lookup(triple)?;
    let isa = isa_builder.finish(shared_flags)?;
    let mut validator = Validator::new();
    let parser = WasmParser::new(0);
    let mut types = Default::default();
    let tunables = Tunables::default();
    let mut translation = ModuleEnvironment::new(&tunables, &mut validator, &mut types)
        .translate(parser, &bytes)
        .context("Failed to translate WebAssembly module")?;

    let vmoffsets = VMOffsets::new(isa.pointer_bytes(), &translation.module);
    let _ = types.finish();

    let body_inputs = std::mem::take(&mut translation.function_body_inputs);
    let module = &translation.module;
    let types = translation.get_types();
    let env = FuncEnv::new(module, &types);

    body_inputs
        .into_iter()
        .try_for_each(|func| compile(&env, &isa, &vmoffsets, func))?;

    Ok(())
}

fn compile(
    env: &FuncEnv,
    isa: &Box<dyn TargetIsa>,
    vmoffsets: &VMOffsets<u8>,
    f: (DefinedFuncIndex, FunctionBodyData<'_>),
) -> Result<()> {
    let index = env.module.func_index(f.0);
    let sig = env
        .types
        .function_at(index.as_u32())
        .expect(&format!("function type at index {:?}", index.as_u32()));
    let sig = env.convert_func_type(sig);
    let FunctionBodyData { body, validator } = f.1;
    let mut validator = validator.into_validator(Default::default());
    let buffer = isa
        .compile_function(&sig, &body, vmoffsets, env, &mut validator)
        .expect("Couldn't compile function");

    println!("Disassembly for function: {}", index.as_u32());
    disasm(buffer.data(), isa)?
        .iter()
        .for_each(|s| println!("{}", s));

    Ok(())
}
