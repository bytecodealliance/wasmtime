use anyhow::{bail, Context as _, Result};
use object::write::Object;
use target_lexicon::Triple;
use wasmparser::WasmFeatures;
use wasmtime::Strategy;
use wasmtime_environ::{ModuleEnvironment, Tunables};
use wasmtime_jit::Compiler;

/// Creates object file from binary wasm data.
pub fn compile_to_obj(
    wasm: &[u8],
    target: Option<&Triple>,
    strategy: Strategy,
    enable_simd: bool,
    opt_level: wasmtime::OptLevel,
    debug_info: bool,
) -> Result<Object> {
    let strategy = match strategy {
        Strategy::Auto => wasmtime_jit::CompilationStrategy::Auto,
        Strategy::Cranelift => wasmtime_jit::CompilationStrategy::Cranelift,
        #[cfg(feature = "lightbeam")]
        Strategy::Lightbeam => wasmtime_jit::CompilationStrategy::Lightbeam,
        #[cfg(not(feature = "lightbeam"))]
        Strategy::Lightbeam => bail!("lightbeam support not enabled"),
        s => bail!("unknown compilation strategy {:?}", s),
    };
    let mut builder = Compiler::builder(strategy);
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

    let compiler = Compiler::new(&*builder, tunables.clone(), features.clone(), true);
    let environ = ModuleEnvironment::new(&tunables, &features);
    let (_main_module, mut translation, types) = environ
        .translate(wasm)
        .context("failed to translate module")?;
    assert_eq!(translation.len(), 1);
    let compilation = compiler.compile(&mut translation[0], &types)?;
    Ok(compilation.obj)
}
