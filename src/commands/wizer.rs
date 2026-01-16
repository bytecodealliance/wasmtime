use crate::commands::run::{CliInstance, Preloads, RunCommand};
use crate::common::{RunCommon, RunTarget};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use wasmtime::{Module, Result, error::Context as _};
use wasmtime_wizer::Wizer;

#[derive(clap::Parser)]
#[expect(missing_docs, reason = "inheriting wizer's docs")]
pub struct WizerCommand {
    #[command(flatten)]
    run: RunCommon,

    #[command(flatten)]
    wizer: Wizer,

    /// The input Wasm module's file path.
    input: PathBuf,

    #[command(flatten)]
    preloads: Preloads,

    /// The file path to write the output Wasm module to.
    ///
    /// If not specified, then `stdout` is used.
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,
}

enum WizerInfo<'a> {
    Core(wasmtime_wizer::ModuleContext<'a>),
    #[cfg(feature = "component-model")]
    Component(wasmtime_wizer::ComponentContext<'a>),
}

impl WizerCommand {
    /// Runs the command.
    pub fn execute(mut self) -> Result<()> {
        self.run.common.init_logging()?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .enable_io()
            .build()?;
        runtime.block_on(self.execute_async())
    }

    async fn execute_async(mut self) -> Result<()> {
        // By default use deterministic relaxed simd operations to guarantee
        // that if relaxed simd operations are used in a module that they always
        // produce the same result.
        if self.run.common.wasm.relaxed_simd_deterministic.is_none() {
            self.run.common.wasm.relaxed_simd_deterministic = Some(true);
        }

        // Don't provide any WASI imports by default to wizened components. The
        // `run` command provides the "cli" world as a default so turn that off
        // here if the command line flags don't otherwise say what to do.
        if self.run.common.wasi.cli.is_none() {
            self.run.common.wasi.cli = Some(false);
        }

        // Read the input wasm, possibly from stdin.
        let mut wasm = Vec::new();
        if self.input.to_str() == Some("-") {
            io::stdin()
                .read_to_end(&mut wasm)
                .context("failed to read input Wasm module from stdin")?;
        } else {
            wasm = fs::read(&self.input).context("failed to read input Wasm module")?;
        }

        #[cfg(feature = "wat")]
        let wasm = wat::parse_bytes(&wasm)?;
        let is_component = wasmparser::Parser::is_component(&wasm);

        let mut run = RunCommand {
            run: self.run,
            argv0: None,
            invoke: Some(if is_component {
                format!("{}()", self.wizer.get_init_func())
            } else {
                self.wizer.get_init_func().to_string()
            }),
            module_and_args: vec![self.input.clone().into()],
            preloads: self.preloads.clone(),
        };
        let engine = run.new_engine()?;

        // Instrument the input wasm with wizer.
        let (cx, main) = if is_component {
            #[cfg(feature = "component-model")]
            {
                let (cx, wasm) = self.wizer.instrument_component(&wasm)?;
                (
                    WizerInfo::Component(cx),
                    RunTarget::Component(wasmtime::component::Component::new(&engine, &wasm)?),
                )
            }
            #[cfg(not(feature = "component-model"))]
            unreachable!();
        } else {
            let (cx, wasm) = self.wizer.instrument(&wasm)?;
            (
                WizerInfo::Core(cx),
                RunTarget::Core(Module::new(&engine, &wasm)?),
            )
        };

        // Execute a rough equivalent of
        // `wasmtime run --invoke <..> <instrumented-wasm>`
        let (mut store, mut linker) = run.new_store_and_linker(&engine, &main)?;
        let instance = run
            .instantiate_and_run(&engine, &mut linker, &main, &mut store)
            .await?;

        // Use our state to capture a snapshot with Wizer and then serialize
        // that.
        let final_wasm = match (cx, instance) {
            (WizerInfo::Core(cx), CliInstance::Core(instance)) => {
                self.wizer
                    .snapshot(
                        cx,
                        &mut wasmtime_wizer::WasmtimeWizer {
                            store: &mut store,
                            instance,
                        },
                    )
                    .await?
            }

            #[cfg(feature = "component-model")]
            (WizerInfo::Component(cx), CliInstance::Component(instance)) => {
                self.wizer
                    .snapshot_component(
                        cx,
                        &mut wasmtime_wizer::WasmtimeWizerComponent {
                            store: &mut store,
                            instance,
                        },
                    )
                    .await?
            }

            #[cfg(feature = "component-model")]
            (WizerInfo::Core(_) | WizerInfo::Component(_), _) => unreachable!(),
        };

        match &self.output {
            Some(file) => fs::write(file, &final_wasm).context("failed to write output file")?,
            None => std::io::stdout()
                .write_all(&final_wasm)
                .context("failed to write output to stdout")?,
        }
        Ok(())
    }
}
