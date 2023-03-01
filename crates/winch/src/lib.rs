use anyhow::bail;
use wasmtime_environ::CompilerBuilder;
use winch_codegen::isa;

mod compiler;

pub fn builder() -> Box<dyn CompilerBuilder> {
    wasmtime_cranelift_shared::builder(
        |triple| isa::lookup(triple).map_err(|e| e.into()),
        |isa, opts| {
            if opts.cache_store.is_some() {
                bail!("incremental compilation isn't supported with winch");
            }
            Ok(Box::new(compiler::Compiler::new(
                isa?,
                opts.linkopts.clone(),
            )))
        },
    )
}
