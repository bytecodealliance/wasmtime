//! Common functionality shared between command implementations.

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use std::net::TcpListener;
use std::{path::Path, time::Duration};
use wasmtime::{Engine, Module, Precompiled, StoreLimits, StoreLimitsBuilder};
use wasmtime_cli_flags::{opt::WasmtimeOptionValue, CommonOptions};
use wasmtime_wasi::WasiCtxBuilder;

#[cfg(feature = "component-model")]
use wasmtime::component::Component;

pub enum RunTarget {
    Core(Module),

    #[cfg(feature = "component-model")]
    Component(Component),
}

impl RunTarget {
    pub fn unwrap_core(&self) -> &Module {
        match self {
            RunTarget::Core(module) => module,
            #[cfg(feature = "component-model")]
            RunTarget::Component(_) => panic!("expected a core wasm module, not a component"),
        }
    }

    #[cfg(feature = "component-model")]
    pub fn unwrap_component(&self) -> &Component {
        match self {
            RunTarget::Component(c) => c,
            RunTarget::Core(_) => panic!("expected a component, not a core wasm module"),
        }
    }
}

/// Common command line arguments for run commands.
#[derive(Parser, PartialEq)]
pub struct RunCommon {
    #[command(flatten)]
    pub common: CommonOptions,

    /// Allow executing precompiled WebAssembly modules as `*.cwasm` files.
    ///
    /// Note that this option is not safe to pass if the module being passed in
    /// is arbitrary user input. Only `wasmtime`-precompiled modules generated
    /// via the `wasmtime compile` command or equivalent should be passed as an
    /// argument with this option specified.
    #[arg(long = "allow-precompiled")]
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
    #[arg(
        long,
        value_name = "STRATEGY",
        value_parser = Profile::parse,
    )]
    pub profile: Option<Profile>,

    /// Grant access of a host directory to a guest.
    ///
    /// If specified as just `HOST_DIR` then the same directory name on the
    /// host is made available within the guest. If specified as `HOST::GUEST`
    /// then the `HOST` directory is opened and made available as the name
    /// `GUEST` in the guest.
    #[arg(long = "dir", value_name = "HOST_DIR[::GUEST_DIR]", value_parser = parse_dirs)]
    pub dirs: Vec<(String, String)>,

    /// Pass an environment variable to the program.
    ///
    /// The `--env FOO=BAR` form will set the environment variable named `FOO`
    /// to the value `BAR` for the guest program using WASI. The `--env FOO`
    /// form will set the environment variable named `FOO` to the same value it
    /// has in the calling process for the guest, or in other words it will
    /// cause the environment variable `FOO` to be inherited.
    #[arg(long = "env", number_of_values = 1, value_name = "NAME[=VAL]", value_parser = parse_env_var)]
    pub vars: Vec<(String, Option<String>)>,
}

fn parse_env_var(s: &str) -> Result<(String, Option<String>)> {
    let mut parts = s.splitn(2, '=');
    Ok((
        parts.next().unwrap().to_string(),
        parts.next().map(|s| s.to_string()),
    ))
}

fn parse_dirs(s: &str) -> Result<(String, String)> {
    let mut parts = s.split("::");
    let host = parts.next().unwrap();
    let guest = match parts.next() {
        Some(guest) => guest,
        None => host,
    };
    Ok((host.into(), guest.into()))
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

    pub fn ensure_allow_precompiled(&self) -> Result<()> {
        if self.allow_precompiled {
            Ok(())
        } else {
            bail!("running a precompiled module requires the `--allow-precompiled` flag")
        }
    }

    #[cfg(feature = "component-model")]
    fn ensure_allow_components(&self) -> Result<()> {
        if self.common.wasm.component_model == Some(false) {
            bail!("cannot execute a component without `--wasm component-model`");
        }

        Ok(())
    }

    pub fn load_module(&self, engine: &Engine, path: &Path) -> Result<RunTarget> {
        let path = match path.to_str() {
            #[cfg(unix)]
            Some("-") => "/dev/stdin".as_ref(),
            _ => path,
        };

        // First attempt to load the module as an mmap. If this succeeds then
        // detection can be done with the contents of the mmap and if a
        // precompiled module is detected then `deserialize_file` can be used
        // which is a slightly more optimal version than `deserialize` since we
        // can leave most of the bytes on disk until they're referenced.
        //
        // If the mmap fails, for example if stdin is a pipe, then fall back to
        // `std::fs::read` to load the contents. At that point precompiled
        // modules must go through the `deserialize` functions.
        //
        // Note that this has the unfortunate side effect for precompiled
        // modules on disk that they're opened once to detect what they are and
        // then again internally in Wasmtime as part of the `deserialize_file`
        // API. Currently there's no way to pass the `MmapVec` here through to
        // Wasmtime itself (that'd require making `MmapVec` a public type, both
        // which isn't ready to happen at this time). It's hoped though that
        // opening a file twice isn't too bad in the grand scheme of things with
        // respect to the CLI.
        match wasmtime::_internal::MmapVec::from_file(path) {
            Ok(map) => self.load_module_contents(
                engine,
                path,
                &map,
                || unsafe { Module::deserialize_file(engine, path) },
                #[cfg(feature = "component-model")]
                || unsafe { Component::deserialize_file(engine, path) },
            ),
            Err(_) => {
                let bytes = std::fs::read(path)
                    .with_context(|| format!("failed to read file: {}", path.display()))?;
                self.load_module_contents(
                    engine,
                    path,
                    &bytes,
                    || unsafe { Module::deserialize(engine, &bytes) },
                    #[cfg(feature = "component-model")]
                    || unsafe { Component::deserialize(engine, &bytes) },
                )
            }
        }
    }

    pub fn load_module_contents(
        &self,
        engine: &Engine,
        path: &Path,
        bytes: &[u8],
        deserialize_module: impl FnOnce() -> Result<Module>,
        #[cfg(feature = "component-model")] deserialize_component: impl FnOnce() -> Result<Component>,
    ) -> Result<RunTarget> {
        Ok(match engine.detect_precompiled(bytes) {
            Some(Precompiled::Module) => {
                self.ensure_allow_precompiled()?;
                RunTarget::Core(deserialize_module()?)
            }
            #[cfg(feature = "component-model")]
            Some(Precompiled::Component) => {
                self.ensure_allow_precompiled()?;
                self.ensure_allow_components()?;
                RunTarget::Component(deserialize_component()?)
            }
            #[cfg(not(feature = "component-model"))]
            Some(Precompiled::Component) => {
                bail!("support for components was not enabled at compile time");
            }
            #[cfg(any(feature = "cranelift", feature = "winch"))]
            None => {
                // Parse the text format here specifically to add the `path` to
                // the error message if there's a syntax error.
                #[cfg(feature = "wat")]
                let bytes = wat::parse_bytes(bytes).map_err(|mut e| {
                    e.set_path(path);
                    e
                })?;
                if wasmparser::Parser::is_component(&bytes) {
                    #[cfg(feature = "component-model")]
                    {
                        self.ensure_allow_components()?;
                        let component = wasmtime::CodeBuilder::new(engine)
                            .wasm(&bytes, Some(path))?
                            .compile_component()?;
                        RunTarget::Component(component)
                    }
                    #[cfg(not(feature = "component-model"))]
                    {
                        bail!("support for components was not enabled at compile time");
                    }
                } else {
                    let module = wasmtime::CodeBuilder::new(engine)
                        .wasm(&bytes, Some(path))?
                        .compile_module()?;
                    RunTarget::Core(module)
                }
            }
            #[cfg(not(any(feature = "cranelift", feature = "winch")))]
            None => {
                let _ = path;
                bail!("support for compiling modules was disabled at compile time");
            }
        })
    }

    pub fn configure_wasip2(&self, builder: &mut WasiCtxBuilder) -> Result<()> {
        // It's ok to block the current thread since we're the only thread in
        // the program as the CLI. This helps improve the performance of some
        // blocking operations in WASI, for example, by skipping the
        // back-and-forth between sync and async.
        builder.allow_blocking_current_thread(true);

        if self.common.wasi.inherit_env == Some(true) {
            for (k, v) in std::env::vars() {
                builder.env(&k, &v);
            }
        }
        for (key, value) in self.vars.iter() {
            let value = match value {
                Some(value) => value.clone(),
                None => match std::env::var_os(key) {
                    Some(val) => val
                        .into_string()
                        .map_err(|_| anyhow!("environment variable `{key}` not valid utf-8"))?,
                    None => {
                        // leave the env var un-set in the guest
                        continue;
                    }
                },
            };
            builder.env(key, &value);
        }

        for (host, guest) in self.dirs.iter() {
            builder.preopened_dir(
                host,
                guest,
                wasmtime_wasi::DirPerms::all(),
                wasmtime_wasi::FilePerms::all(),
            )?;
        }

        if self.common.wasi.listenfd == Some(true) {
            bail!("components do not support --listenfd");
        }
        for _ in self.compute_preopen_sockets()? {
            bail!("components do not support --tcplisten");
        }

        if self.common.wasi.inherit_network == Some(true) {
            builder.inherit_network();
        }
        if let Some(enable) = self.common.wasi.allow_ip_name_lookup {
            builder.allow_ip_name_lookup(enable);
        }
        if let Some(enable) = self.common.wasi.tcp {
            builder.allow_tcp(enable);
        }
        if let Some(enable) = self.common.wasi.udp {
            builder.allow_udp(enable);
        }

        Ok(())
    }

    pub fn compute_preopen_sockets(&self) -> Result<Vec<TcpListener>> {
        let mut listeners = vec![];

        for address in &self.common.wasi.tcplisten {
            let stdlistener = std::net::TcpListener::bind(address)
                .with_context(|| format!("failed to bind to address '{}'", address))?;

            let _ = stdlistener.set_nonblocking(true)?;

            listeners.push(stdlistener)
        }
        Ok(listeners)
    }
}

#[derive(Clone, PartialEq)]
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
