use anyhow::{bail, Context as _, Result};
use std::mem;
use target_lexicon::Triple;
use wasmparser::WasmFeatures;
use wasmtime::Strategy;
use wasmtime_environ::{ModuleEnvironment, PrimaryMap, Tunables};

/// Creates object file from binary wasm data.
pub fn compile_to_obj(
    wasm: &[u8],
    target: Option<&Triple>,
    strategy: Strategy,
    enable_simd: bool,
    opt_level: wasmtime::OptLevel,
    debug_info: bool,
) -> Result<Vec<u8>> {
    match strategy {
        Strategy::Cranelift | Strategy::Auto => {}
        other => panic!("unsupported strategy {:?}", other),
    }
    let mut builder = wasmtime_cranelift::builder();
    if let Some(target) = target {
        builder.target(target.clone())?;
    }
    let mut features = WasmFeatures::default();

    if enable_simd {
        builder.enable("enable_simd").unwrap();
        features.simd = true;
    }

    match opt_level {
        wasmtime::OptLevel::None => {}
        wasmtime::OptLevel::Speed => {
            builder.set("opt_level", "speed").unwrap();
        }
        wasmtime::OptLevel::SpeedAndSize => {
            builder.set("opt_level", "speed_and_size").unwrap();
        }
        other => bail!("unknown optimization level {:?}", other),
    }

    // TODO: Expose the tunables as command-line flags.
    let mut tunables = Tunables::default();
    tunables.generate_native_debuginfo = debug_info;
    tunables.parse_wasm_debuginfo = debug_info;

    let compiler = builder.build();
    let environ = ModuleEnvironment::new(&tunables, &features);
    let (_main_module, mut translation, types) = environ
        .translate(wasm)
        .context("failed to translate module")?;
    assert_eq!(translation.len(), 1);
    let mut funcs = PrimaryMap::default();
    for (index, func) in mem::take(&mut translation[0].function_body_inputs) {
        funcs.push(compiler.compile_function(&translation[0], index, func, &tunables, &types)?);
    }
    let mut obj = compiler.object()?;
    compiler.emit_obj(
        &translation[0],
        &types,
        funcs,
        tunables.generate_native_debuginfo,
        &mut obj,
    )?;
    Ok(obj.write()?)
}
