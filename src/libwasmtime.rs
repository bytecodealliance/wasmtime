//! Library for high-level WASM functionality to be consumed by the [`wasmtime`](wasmtime.rs) and 3rd party applications.

use cranelift_codegen::settings::{self, Configurable};
use cranelift_native;
use std::error::Error;
use std::io::Read;
use wasmtime_jit::{ActionOutcome, Context};

pub struct ContextBuilder<'a> {
    pub opt_level: Option<&'a str>,
    pub enable_verifier: bool,
    pub set_debug_info: bool,
}

impl<'a> ContextBuilder<'a> {
    pub fn try_build(&self) -> Result<Context, String> {
        let mut flag_builder = settings::builder();

        // Enable verifier passes in debug mode.
        if self.enable_verifier {
            flag_builder
                .enable("enable_verifier")
                .map_err(|e| e.to_string())?;
        }

        if let Some(opt_level) = self.opt_level {
            flag_builder
                .set("opt_level", opt_level)
                .map_err(|e| e.to_string())?;
        }

        let isa_builder = cranelift_native::builder()
            .map_err(|e| format!("host machine is not a supported target: {}", e))?;

        let isa = isa_builder.finish(settings::Flags::new(flag_builder));

        let context = Context::with_isa(isa).set_debug_info(self.set_debug_info);

        Ok(context)
    }
}

pub fn read_wasm<T>(mut module: T) -> Result<Vec<u8>, String>
where
    T: Read,
{
    let data = {
        let mut buf: Vec<u8> = Vec::new();
        module
            .read_to_end(&mut buf)
            .map_err(|err| err.to_string())?;
        buf
    };

    // to a wasm binary with wat2wasm.
    if data.starts_with(&[b'\0', b'a', b's', b'm']) {
        Ok(data)
    } else {
        wabt::wat2wasm(data).map_err(|err| String::from(err.description()))
    }
}

pub fn handle_module<T>(
    context: &mut Context,
    module: T,
    flag_invoke: Option<&String>,
) -> Result<(), String>
where
    T: std::io::Read,
{
    // Read the wasm module binary.
    let data = read_wasm(module)?;

    // Compile and instantiating a wasm module.
    let mut instance = context
        .instantiate_module(None, &data)
        .map_err(|e| e.to_string())?;

    // If a function to invoke was given, invoke it.
    if let Some(ref f) = flag_invoke {
        match context
            .invoke(&mut instance, f, &[])
            .map_err(|e| e.to_string())?
        {
            ActionOutcome::Returned { .. } => {}
            ActionOutcome::Trapped { message } => {
                return Err(format!("Trap from within function {}: {}", f, message));
            }
        }
    }

    Ok(())
}
