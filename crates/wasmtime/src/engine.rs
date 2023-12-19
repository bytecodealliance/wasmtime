use std::sync::{atomic::AtomicU64, Arc};

use anyhow::Result;
use once_cell::sync::OnceCell;
#[cfg(feature = "parallel-compilation")]
use rayon::prelude::*;
use serde_derive::{Deserialize, Serialize};
use wasmtime_environ::{FlagValue, Tunables};
#[cfg(feature = "runtime")]
use wasmtime_runtime::{CompiledModuleIdAllocator, InstanceAllocator};

use crate::Config;
#[cfg(feature = "runtime")]
use crate::{profiling_agent::ProfilingAgent, runtime::signatures::SignatureRegistry};

pub(crate) const VERSION: u8 = 0;

/// An `Engine` which is a global context for compilation and management of wasm
/// modules.
///
/// An engine can be safely shared across threads and is a cheap cloneable
/// handle to the actual engine. The engine itself will be deallocated once all
/// references to it have gone away.
///
/// Engines store global configuration preferences such as compilation settings,
/// enabled features, etc. You'll likely only need at most one of these for a
/// program.
///
/// ## Engines and `Clone`
///
/// Using `clone` on an `Engine` is a cheap operation. It will not create an
/// entirely new engine, but rather just a new reference to the existing engine.
/// In other words it's a shallow copy, not a deep copy.
///
/// ## Engines and `Default`
///
/// You can create an engine with default configuration settings using
/// `Engine::default()`. Be sure to consult the documentation of [`Config`] for
/// default settings.
#[derive(Clone)]
pub struct Engine {
    pub(crate) inner: Arc<EngineInner>,
}

pub(crate) struct EngineInner {
    pub(crate) config: Config,
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub(crate) compiler: Box<dyn wasmtime_environ::Compiler>,
    #[cfg(feature = "runtime")]
    pub(crate) allocator: Box<dyn InstanceAllocator + Send + Sync>,
    #[cfg(feature = "runtime")]
    pub(crate) profiler: Box<dyn ProfilingAgent>,
    #[cfg(feature = "runtime")]
    pub(crate) signatures: SignatureRegistry,
    pub(crate) epoch: AtomicU64,
    #[cfg(feature = "runtime")]
    pub(crate) unique_id_allocator: CompiledModuleIdAllocator,

    /// One-time check of whether the compiler's settings, if present, are
    /// compatible with the native host.
    #[cfg(feature = "runtime")]
    pub(crate) compatible_with_native_host: OnceCell<Result<(), String>>,
}

impl Engine {
    /// Creates a new [`Engine`] with the specified compilation and
    /// configuration settings.
    ///
    /// # Errors
    ///
    /// This method can fail if the `config` is invalid or some
    /// configurations are incompatible.
    ///
    /// For example, feature `reference_types` will need to set
    /// the compiler setting `enable_safepoints` and `unwind_info`
    /// to `true`, but explicitly disable these two compiler settings
    /// will cause errors.
    pub fn new(config: &Config) -> Result<Engine> {
        #[cfg(feature = "runtime")]
        {
            // Ensure that wasmtime_runtime's signal handlers are configured. This
            // is the per-program initialization required for handling traps, such
            // as configuring signals, vectored exception handlers, etc.
            wasmtime_runtime::init_traps(
                crate::module::is_wasm_trap_pc,
                config.macos_use_mach_ports,
            );
            #[cfg(feature = "debug-builtins")]
            wasmtime_runtime::debug_builtins::ensure_exported();
        }

        let config = config.clone();
        config.validate()?;

        #[cfg(any(feature = "cranelift", feature = "winch"))]
        let (config, compiler) = config.build_compiler()?;

        Ok(Engine {
            inner: Arc::new(EngineInner {
                #[cfg(any(feature = "cranelift", feature = "winch"))]
                compiler,
                #[cfg(feature = "runtime")]
                allocator: config.build_allocator()?,
                #[cfg(feature = "runtime")]
                profiler: config.build_profiler()?,
                #[cfg(feature = "runtime")]
                signatures: SignatureRegistry::new(),
                epoch: AtomicU64::new(0),
                #[cfg(feature = "runtime")]
                unique_id_allocator: CompiledModuleIdAllocator::new(),
                #[cfg(feature = "runtime")]
                compatible_with_native_host: OnceCell::new(),
                config,
            }),
        })
    }

    /// Returns the configuration settings that this engine is using.
    #[inline]
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    pub(crate) fn run_maybe_parallel<
        A: Send,
        B: Send,
        E: Send,
        F: Fn(A) -> Result<B, E> + Send + Sync,
    >(
        &self,
        input: Vec<A>,
        f: F,
    ) -> Result<Vec<B>, E> {
        if self.config().parallel_compilation {
            #[cfg(feature = "parallel-compilation")]
            return input
                .into_par_iter()
                .map(|a| f(a))
                .collect::<Result<Vec<B>, E>>();
        }

        // In case the parallel-compilation feature is disabled or the parallel_compilation config
        // was turned off dynamically fallback to the non-parallel version.
        input
            .into_iter()
            .map(|a| f(a))
            .collect::<Result<Vec<B>, E>>()
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Metadata<'a> {
    pub(crate) target: String,
    #[serde(borrow)]
    pub(crate) shared_flags: Vec<(&'a str, FlagValue<'a>)>,
    #[serde(borrow)]
    pub(crate) isa_flags: Vec<(&'a str, FlagValue<'a>)>,
    pub(crate) tunables: Tunables,
    pub(crate) features: WasmFeatures,
}

// This exists because `wasmparser::WasmFeatures` isn't serializable
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct WasmFeatures {
    pub(crate) reference_types: bool,
    pub(crate) multi_value: bool,
    pub(crate) bulk_memory: bool,
    pub(crate) component_model: bool,
    pub(crate) simd: bool,
    pub(crate) tail_call: bool,
    pub(crate) threads: bool,
    pub(crate) multi_memory: bool,
    pub(crate) exceptions: bool,
    pub(crate) memory64: bool,
    pub(crate) relaxed_simd: bool,
    pub(crate) extended_const: bool,
    pub(crate) function_references: bool,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Config;

    #[test]
    fn test_architecture_mismatch() -> Result<()> {
        let engine = Engine::default();
        let mut metadata = Metadata::new(&engine);
        metadata.target = "unknown-generic-linux".to_string();

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled for architecture 'unknown'",
            ),
        }

        Ok(())
    }

    #[test]
    fn test_os_mismatch() -> Result<()> {
        let engine = Engine::default();
        let mut metadata = Metadata::new(&engine);

        metadata.target = format!(
            "{}-generic-unknown",
            target_lexicon::Triple::host().architecture
        );

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled for operating system 'unknown'",
            ),
        }

        Ok(())
    }

    #[test]
    fn test_cranelift_flags_mismatch() -> Result<()> {
        let engine = Engine::default();
        let mut metadata = Metadata::new(&engine);

        metadata
            .shared_flags
            .push(("preserve_frame_pointers", FlagValue::Bool(false)));

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert!(format!("{:?}", e).starts_with(
                "\
compilation settings of module incompatible with native host

Caused by:
    setting \"preserve_frame_pointers\" is configured to Bool(false) which is not supported"
            )),
        }

        Ok(())
    }

    #[test]
    fn test_isa_flags_mismatch() -> Result<()> {
        let engine = Engine::default();
        let mut metadata = Metadata::new(&engine);

        metadata
            .isa_flags
            .push(("not_a_flag", FlagValue::Bool(true)));

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert!(format!("{:?}", e).starts_with(
                "\
compilation settings of module incompatible with native host

Caused by:
    cannot test if target-specific flag \"not_a_flag\" is available at runtime",
            )),
        }

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_tunables_int_mismatch() -> Result<()> {
        let engine = Engine::default();
        let mut metadata = Metadata::new(&engine);

        metadata.tunables.static_memory_offset_guard_size = 0;

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(e.to_string(), "Module was compiled with a static memory guard size of '0' but '2147483648' is expected for the host"),
        }

        Ok(())
    }

    #[test]
    fn test_tunables_bool_mismatch() -> Result<()> {
        let mut config = Config::new();
        config.epoch_interruption(true);

        let engine = Engine::new(&config)?;
        let mut metadata = Metadata::new(&engine);
        metadata.tunables.epoch_interruption = false;

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled without epoch interruption but it is enabled for the host"
            ),
        }

        let mut config = Config::new();
        config.epoch_interruption(false);

        let engine = Engine::new(&config)?;
        let mut metadata = Metadata::new(&engine);
        metadata.tunables.epoch_interruption = true;

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled with epoch interruption but it is not enabled for the host"
            ),
        }

        Ok(())
    }

    #[test]
    fn test_feature_mismatch() -> Result<()> {
        let mut config = Config::new();
        config.wasm_threads(true);

        let engine = Engine::new(&config)?;
        let mut metadata = Metadata::new(&engine);
        metadata.features.threads = false;

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(e.to_string(), "Module was compiled without WebAssembly threads support but it is enabled for the host"),
        }

        let mut config = Config::new();
        config.wasm_threads(false);

        let engine = Engine::new(&config)?;
        let mut metadata = Metadata::new(&engine);
        metadata.features.threads = true;

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(e.to_string(), "Module was compiled with WebAssembly threads support but it is not enabled for the host"),
        }

        Ok(())
    }
}
