use anyhow::{bail, Context, Result};
use clap::Parser;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use wasmparser::{Validator, WasmFeatures};
use wasmtime_environ::component::*;
use wasmtime_environ::fact::Module;

/// A small helper utility to explore generated adapter modules from Wasmtime's
/// adapter fusion compiler.
///
/// This utility takes a `*.wat` file as input which is expected to be a valid
/// WebAssembly component. The component is parsed and any type definition for a
/// component function gets a generated adapter for it as if the caller/callee
/// used that type as the adapter.
///
/// For example with an input that looks like:
///
///     (component
///         (type (func (param u32) (result (list u8))))
///     )
///
/// This tool can be used to generate an adapter for that signature.
#[derive(Parser)]
struct Factc {
    /// Whether or not debug code is inserted into the generated adapter.
    #[arg(long)]
    debug: bool,

    /// Whether or not the lifting options (the callee of the exported adapter)
    /// uses a 64-bit memory as opposed to a 32-bit memory.
    #[arg(long)]
    lift64: bool,

    /// Whether or not the lowering options (the caller of the exported adapter)
    /// uses a 64-bit memory as opposed to a 32-bit memory.
    #[arg(long)]
    lower64: bool,

    /// Whether or not a call to a `post-return` configured function is enabled
    /// or not.
    #[arg(long)]
    post_return: bool,

    /// Whether or not to skip validation of the generated adapter module.
    #[arg(long)]
    skip_validate: bool,

    /// Where to place the generated adapter module. Standard output is used if
    /// this is not specified.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output the text format for WebAssembly instead of the binary format.
    #[arg(short, long)]
    text: bool,

    #[arg(long, value_parser = parse_string_encoding, default_value = "utf8")]
    lift_str: StringEncoding,

    #[arg(long, value_parser = parse_string_encoding, default_value = "utf8")]
    lower_str: StringEncoding,

    /// TODO
    input: PathBuf,
}

fn parse_string_encoding(name: &str) -> Result<StringEncoding> {
    Ok(match name {
        "utf8" => StringEncoding::Utf8,
        "utf16" => StringEncoding::Utf16,
        "compact-utf16" => StringEncoding::CompactUtf16,
        other => bail!("invalid string encoding: `{other}`"),
    })
}

fn main() -> Result<()> {
    Factc::parse().execute()
}

impl Factc {
    fn execute(self) -> Result<()> {
        env_logger::init();

        // Manufactures a unique `CoreDef` so all function imports get unique
        // function imports.
        let mut next_def = 0;
        let mut dummy_def = || {
            next_def += 1;
            dfg::CoreDef::Adapter(dfg::AdapterId::from_u32(next_def))
        };

        // Manufactures a `CoreExport` for a memory with the shape specified. Note
        // that we can't import as many memories as functions so these are
        // intentionally limited. Once a handful of memories are generated of each
        // type then they start getting reused.
        let mut next_memory = 0;
        let mut memories32 = Vec::new();
        let mut memories64 = Vec::new();
        let mut dummy_memory = |memory64: bool| {
            let dst = if memory64 {
                &mut memories64
            } else {
                &mut memories32
            };
            let idx = if dst.len() < 5 {
                next_memory += 1;
                dst.push(next_memory - 1);
                next_memory - 1
            } else {
                dst[0]
            };
            dfg::CoreExport {
                instance: dfg::InstanceId::from_u32(idx),
                item: ExportItem::Name(String::new()),
            }
        };

        let mut validator = Validator::new();
        let mut types = ComponentTypesBuilder::new(&validator);

        let mut adapters = Vec::new();
        let input = wat::parse_file(&self.input)?;
        let wasm_types = validator
            .validate_all(&input)
            .context("failed to validate input wasm")?;
        let wasm_types = wasm_types.as_ref();
        for i in 0..wasm_types.component_type_count() {
            let ty = match wasm_types.component_any_type_at(i) {
                wasmparser::component_types::ComponentAnyTypeId::Func(id) => id,
                _ => continue,
            };
            let ty = types.convert_component_func_type(wasm_types, ty)?;
            adapters.push(Adapter {
                lift_ty: ty,
                lower_ty: ty,
                lower_options: AdapterOptions {
                    instance: RuntimeComponentInstanceIndex::from_u32(0),
                    string_encoding: self.lower_str,
                    memory64: self.lower64,
                    // Pessimistically assume that memory/realloc are going to be
                    // required for this trampoline and provide it. Avoids doing
                    // calculations to figure out whether they're necessary and
                    // simplifies the fuzzer here without reducing coverage within FACT
                    // itself.
                    memory: Some(dummy_memory(self.lower64)),
                    realloc: Some(dummy_def()),
                    // Lowering never allows `post-return`
                    post_return: None,
                    async_: false,
                    callback: None,
                },
                lift_options: AdapterOptions {
                    instance: RuntimeComponentInstanceIndex::from_u32(1),
                    string_encoding: self.lift_str,
                    memory64: self.lift64,
                    memory: Some(dummy_memory(self.lift64)),
                    realloc: Some(dummy_def()),
                    post_return: if self.post_return {
                        Some(dummy_def())
                    } else {
                        None
                    },
                    async_: false,
                    callback: None,
                },
                func: dummy_def(),
            });
        }

        let mut fact_module = Module::new(&types, self.debug);
        for (i, adapter) in adapters.iter().enumerate() {
            fact_module.adapt(&format!("adapter{i}"), adapter);
        }
        let wasm = fact_module.encode();

        let output = if self.text {
            wasmprinter::print_bytes(&wasm)
                .context("failed to convert binary wasm to text")?
                .into_bytes()
        } else if self.output.is_none() && std::io::stdout().is_terminal() {
            bail!("cannot print binary wasm output to a terminal unless `-t` flag is passed")
        } else {
            wasm.clone()
        };

        match &self.output {
            Some(file) => std::fs::write(file, output).context("failed to write output file")?,
            None => std::io::stdout()
                .write_all(&output)
                .context("failed to write to stdout")?,
        }

        if !self.skip_validate {
            Validator::new_with_features(WasmFeatures::default() | WasmFeatures::MEMORY64)
                .validate_all(&wasm)
                .context("failed to validate generated module")?;
        }

        Ok(())
    }
}
