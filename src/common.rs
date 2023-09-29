//! Common functionality shared between command implementations.

use anyhow::{bail, Result};
use clap::Parser;
use std::time::Duration;
use wasmtime::{StoreLimits, StoreLimitsBuilder};
use wasmtime_cli_flags::{opt::WasmtimeOptionValue, CommonOptions};

/// Common command line arguments for run commands.
#[derive(Parser)]
#[structopt(name = "run")]
pub struct RunCommon {
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Allow executing precompiled WebAssembly modules as `*.cwasm` files.
    ///
    /// Note that this option is not safe to pass if the module being passed in
    /// is arbitrary user input. Only `wasmtime`-precompiled modules generated
    /// via the `wasmtime compile` command or equivalent should be passed as an
    /// argument with this option specified.
    #[clap(long = "allow-precompiled")]
    pub allow_precompiled: bool,

    /// Profiling strategy (valid options are: perfmap, jitdump, vtune, guest)
    ///
    /// The perfmap, jitdump, and vtune profiling strategies integrate Wasmtime
    /// with external profilers such as `perf`. The guest profiling strategy
    /// enables in-process sampling and will write the captured profile to
    /// `wasmtime-guest-profile.json` by default which can be viewed at
    /// https://profiler.firefox.com/.
    ///
    /// The `guest` option can be additionally configured as:
    ///
    ///     --profile=guest[,path[,interval]]
    ///
    /// where `path` is where to write the profile and `interval` is the
    /// duration between samples. When used with `--wasm-timeout` the timeout
    /// will be rounded up to the nearest multiple of this interval.
    #[clap(
        long,
        value_name = "STRATEGY",
        value_parser = Profile::parse,
    )]
    pub profile: Option<Profile>,
}

impl RunCommon {
    pub fn store_limits(&self) -> StoreLimits {
        let mut limits = StoreLimitsBuilder::new();
        if let Some(max) = self.common.wasm.max_memory_size {
            limits = limits.memory_size(max);
        }
        if let Some(max) = self.common.wasm.max_table_elements {
            limits = limits.table_elements(max);
        }
        if let Some(max) = self.common.wasm.max_instances {
            limits = limits.instances(max);
        }
        if let Some(max) = self.common.wasm.max_tables {
            limits = limits.tables(max);
        }
        if let Some(max) = self.common.wasm.max_memories {
            limits = limits.memories(max);
        }
        if let Some(enable) = self.common.wasm.trap_on_grow_failure {
            limits = limits.trap_on_grow_failure(enable);
        }

        limits.build()
    }
}

#[derive(Clone)]
pub enum Profile {
    Native(wasmtime::ProfilingStrategy),
    Guest { path: String, interval: Duration },
}

impl Profile {
    /// Parse the `profile` argument to either the `run` or `serve` commands.
    pub fn parse(s: &str) -> Result<Profile> {
        let parts = s.split(',').collect::<Vec<_>>();
        match &parts[..] {
            ["perfmap"] => Ok(Profile::Native(wasmtime::ProfilingStrategy::PerfMap)),
            ["jitdump"] => Ok(Profile::Native(wasmtime::ProfilingStrategy::JitDump)),
            ["vtune"] => Ok(Profile::Native(wasmtime::ProfilingStrategy::VTune)),
            ["guest"] => Ok(Profile::Guest {
                path: "wasmtime-guest-profile.json".to_string(),
                interval: Duration::from_millis(10),
            }),
            ["guest", path] => Ok(Profile::Guest {
                path: path.to_string(),
                interval: Duration::from_millis(10),
            }),
            ["guest", path, dur] => Ok(Profile::Guest {
                path: path.to_string(),
                interval: WasmtimeOptionValue::parse(Some(dur))?,
            }),
            _ => bail!("unknown profiling strategy: {s}"),
        }
    }
}
