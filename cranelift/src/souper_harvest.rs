use crate::utils::parse_sets_and_triple;
use cranelift_codegen::Context;
use cranelift_wasm::{DummyEnvironment, ReturnMode};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{fs, io};

static WASM_MAGIC: &[u8] = &[0x00, 0x61, 0x73, 0x6D];

pub fn run(target: &str, input: &str, output: &str, flag_set: &[String]) -> Result<(), String> {
    let parsed = parse_sets_and_triple(flag_set, target)?;
    let fisa = parsed.as_fisa();
    if fisa.isa.is_none() {
        return Err("`souper-harvest` requires a target isa".into());
    }

    let stdin = io::stdin();
    let mut input: Box<dyn io::BufRead> = match input {
        "-" => Box::new(stdin.lock()),
        _ => Box::new(io::BufReader::new(
            fs::File::open(input).map_err(|e| format!("failed to open input file: {}", e))?,
        )),
    };

    let mut output: Box<dyn io::Write + Send> = match output {
        "-" => Box::new(io::stdout()),
        _ => Box::new(io::BufWriter::new(
            fs::File::create(output).map_err(|e| format!("failed to create output file: {}", e))?,
        )),
    };

    let mut contents = vec![];
    input
        .read_to_end(&mut contents)
        .map_err(|e| format!("failed to read from input file: {}", e))?;

    let funcs = if &contents[..WASM_MAGIC.len()] == WASM_MAGIC {
        let mut dummy_environ = DummyEnvironment::new(
            fisa.isa.unwrap().frontend_config(),
            ReturnMode::NormalReturns,
            false,
        );
        cranelift_wasm::translate_module(&contents, &mut dummy_environ)
            .map_err(|e| format!("failed to translate Wasm module to clif: {}", e))?;
        dummy_environ
            .info
            .function_bodies
            .iter()
            .map(|(_, f)| f.clone())
            .collect()
    } else {
        let contents = String::from_utf8(contents)
            .map_err(|e| format!("input is not a UTF-8 string: {}", e))?;
        cranelift_reader::parse_functions(&contents)
            .map_err(|e| format!("failed to parse clif: {}", e))?
    };

    let (send, recv) = std::sync::mpsc::channel::<String>();

    let writing_thread = std::thread::spawn(move || -> Result<(), String> {
        for lhs in recv {
            output
                .write_all(lhs.as_bytes())
                .map_err(|e| format!("failed to write to output file: {}", e))?;
        }
        Ok(())
    });

    funcs
        .into_par_iter()
        .map_with(send, move |send, func| {
            let mut ctx = Context::new();
            ctx.func = func;

            ctx.compute_cfg();
            ctx.preopt(fisa.isa.unwrap())
                .map_err(|e| format!("failed to run preopt: {}", e))?;

            ctx.souper_harvest(send)
                .map_err(|e| format!("failed to run souper harvester: {}", e))?;

            Ok(())
        })
        .collect::<Result<(), String>>()?;

    match writing_thread.join() {
        Ok(result) => result?,
        Err(e) => std::panic::resume_unwind(e),
    }

    Ok(())
}
