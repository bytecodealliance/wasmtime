use crate::signatures::SignatureRegistry;
use crate::Config;
use anyhow::Result;
use once_cell::sync::OnceCell;
#[cfg(feature = "parallel-compilation")]
use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
#[cfg(feature = "cache")]
use wasmtime_cache::CacheConfig;
use wasmtime_environ::FlagValue;
use wasmtime_jit::ProfilingAgent;
use wasmtime_runtime::{debug_builtins, CompiledModuleIdAllocator, InstanceAllocator};

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
    inner: Arc<EngineInner>,
}

struct EngineInner {
    config: Config,
    #[cfg(compiler)]
    compiler: Box<dyn wasmtime_environ::Compiler>,
    allocator: Box<dyn InstanceAllocator>,
    profiler: Box<dyn ProfilingAgent>,
    signatures: SignatureRegistry,
    epoch: AtomicU64,
    unique_id_allocator: CompiledModuleIdAllocator,

    // One-time check of whether the compiler's settings, if present, are
    // compatible with the native host.
    compatible_with_native_host: OnceCell<Result<(), String>>,
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
        // Ensure that wasmtime_runtime's signal handlers are configured. This
        // is the per-program initialization required for handling traps, such
        // as configuring signals, vectored exception handlers, etc.
        wasmtime_runtime::init_traps(crate::module::is_wasm_trap_pc);
        debug_builtins::ensure_exported();

        let registry = SignatureRegistry::new();
        let mut config = config.clone();
        config.validate()?;

        #[cfg(compiler)]
        let compiler = config.build_compiler()?;

        let allocator = config.build_allocator()?;
        allocator.adjust_tunables(&mut config.tunables);
        let profiler = config.build_profiler()?;

        Ok(Engine {
            inner: Arc::new(EngineInner {
                #[cfg(compiler)]
                compiler,
                config,
                allocator,
                profiler,
                signatures: registry,
                epoch: AtomicU64::new(0),
                unique_id_allocator: CompiledModuleIdAllocator::new(),
                compatible_with_native_host: OnceCell::new(),
            }),
        })
    }

    /// Eagerly initialize thread-local functionality shared by all [`Engine`]s.
    ///
    /// Wasmtime's implementation on some platforms may involve per-thread
    /// setup that needs to happen whenever WebAssembly is invoked. This setup
    /// can take on the order of a few hundred microseconds, whereas the
    /// overhead of calling WebAssembly is otherwise on the order of a few
    /// nanoseconds. This setup cost is paid once per-OS-thread. If your
    /// application is sensitive to the latencies of WebAssembly function
    /// calls, even those that happen first on a thread, then this function
    /// can be used to improve the consistency of each call into WebAssembly
    /// by explicitly frontloading the cost of the one-time setup per-thread.
    ///
    /// Note that this function is not required to be called in any embedding.
    /// Wasmtime will automatically initialize thread-local-state as necessary
    /// on calls into WebAssembly. This is provided for use cases where the
    /// latency of WebAssembly calls are extra-important, which is not
    /// necessarily true of all embeddings.
    pub fn tls_eager_initialize() {
        wasmtime_runtime::tls_eager_initialize();
    }

    /// Returns the configuration settings that this engine is using.
    #[inline]
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    #[cfg(compiler)]
    pub(crate) fn compiler(&self) -> &dyn wasmtime_environ::Compiler {
        &*self.inner.compiler
    }

    pub(crate) fn allocator(&self) -> &dyn InstanceAllocator {
        self.inner.allocator.as_ref()
    }

    pub(crate) fn profiler(&self) -> &dyn ProfilingAgent {
        self.inner.profiler.as_ref()
    }

    #[cfg(feature = "cache")]
    pub(crate) fn cache_config(&self) -> &CacheConfig {
        &self.config().cache_config
    }

    /// Returns whether the engine `a` and `b` refer to the same configuration.
    pub fn same(a: &Engine, b: &Engine) -> bool {
        Arc::ptr_eq(&a.inner, &b.inner)
    }

    pub(crate) fn signatures(&self) -> &SignatureRegistry {
        &self.inner.signatures
    }

    pub(crate) fn epoch_counter(&self) -> &AtomicU64 {
        &self.inner.epoch
    }

    pub(crate) fn current_epoch(&self) -> u64 {
        self.epoch_counter().load(Ordering::Relaxed)
    }

    /// Increments the epoch.
    ///
    /// When using epoch-based interruption, currently-executing Wasm
    /// code within this engine will trap or yield "soon" when the
    /// epoch deadline is reached or exceeded. (The configuration, and
    /// the deadline, are set on the `Store`.) The intent of the
    /// design is for this method to be called by the embedder at some
    /// regular cadence, for example by a thread that wakes up at some
    /// interval, or by a signal handler.
    ///
    /// See [`Config::epoch_interruption`](crate::Config::epoch_interruption)
    /// for an introduction to epoch-based interruption and pointers
    /// to the other relevant methods.
    ///
    /// ## Signal Safety
    ///
    /// This method is signal-safe: it does not make any syscalls, and
    /// performs only an atomic increment to the epoch value in
    /// memory.
    pub fn increment_epoch(&self) {
        self.inner.epoch.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn unique_id_allocator(&self) -> &CompiledModuleIdAllocator {
        &self.inner.unique_id_allocator
    }

    /// Ahead-of-time (AOT) compiles a WebAssembly module.
    ///
    /// The `bytes` provided must be in one of two formats:
    ///
    /// * A [binary-encoded][binary] WebAssembly module. This is always supported.
    /// * A [text-encoded][text] instance of the WebAssembly text format.
    ///   This is only supported when the `wat` feature of this crate is enabled.
    ///   If this is supplied then the text format will be parsed before validation.
    ///   Note that the `wat` feature is enabled by default.
    ///
    /// This method may be used to compile a module for use with a different target
    /// host. The output of this method may be used with
    /// [`Module::deserialize`](crate::Module::deserialize) on hosts compatible
    /// with the [`Config`] associated with this [`Engine`].
    ///
    /// The output of this method is safe to send to another host machine for later
    /// execution. As the output is already a compiled module, translation and code
    /// generation will be skipped and this will improve the performance of constructing
    /// a [`Module`](crate::Module) from the output of this method.
    ///
    /// [binary]: https://webassembly.github.io/spec/core/binary/index.html
    /// [text]: https://webassembly.github.io/spec/core/text/index.html
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub fn precompile_module(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        #[cfg(feature = "wat")]
        let bytes = wat::parse_bytes(&bytes)?;
        let (mmap, _, types) = crate::Module::build_artifacts(self, &bytes)?;
        crate::module::SerializedModule::from_artifacts(self, &mmap, &types)
            .to_bytes(&self.config().module_version)
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

    /// Executes `f1` and `f2` in parallel if parallel compilation is enabled at
    /// both runtime and compile time, otherwise runs them synchronously.
    #[allow(dead_code)] // only used for the component-model feature right now
    pub(crate) fn join_maybe_parallel<T, U>(
        &self,
        f1: impl FnOnce() -> T + Send,
        f2: impl FnOnce() -> U + Send,
    ) -> (T, U)
    where
        T: Send,
        U: Send,
    {
        if self.config().parallel_compilation {
            #[cfg(feature = "parallel-compilation")]
            return rayon::join(f1, f2);
        }
        (f1(), f2())
    }

    /// Returns the target triple which this engine is compiling code for
    /// and/or running code for.
    pub(crate) fn target(&self) -> target_lexicon::Triple {
        // If a compiler is configured, use that target.
        #[cfg(compiler)]
        return self.compiler().triple().clone();

        // ... otherwise it's the native target
        #[cfg(not(compiler))]
        return target_lexicon::Triple::host();
    }

    /// Verify that this engine's configuration is compatible with loading
    /// modules onto the native host platform.
    ///
    /// This method is used as part of `Module::new` to ensure that this
    /// engine can indeed load modules for the configured compiler (if any).
    /// Note that if cranelift is disabled this trivially returns `Ok` because
    /// loaded serialized modules are checked separately.
    pub(crate) fn check_compatible_with_native_host(&self) -> Result<()> {
        self.inner
            .compatible_with_native_host
            .get_or_init(|| self._check_compatible_with_native_host())
            .clone()
            .map_err(anyhow::Error::msg)
    }
    fn _check_compatible_with_native_host(&self) -> Result<(), String> {
        #[cfg(compiler)]
        {
            let compiler = self.compiler();

            // Check to see that the config's target matches the host
            let target = compiler.triple();
            if *target != target_lexicon::Triple::host() {
                return Err(format!(
                    "target '{}' specified in the configuration does not match the host",
                    target
                ));
            }

            // Also double-check all compiler settings
            for (key, value) in compiler.flags().iter() {
                self.check_compatible_with_shared_flag(key, value)?;
            }
            for (key, value) in compiler.isa_flags().iter() {
                self.check_compatible_with_isa_flag(key, value)?;
            }
        }
        Ok(())
    }

    /// Checks to see whether the "shared flag", something enabled for
    /// individual compilers, is compatible with the native host platform.
    ///
    /// This is used both when validating an engine's compilation settings are
    /// compatible with the host as well as when deserializing modules from
    /// disk to ensure they're compatible with the current host.
    ///
    /// Note that most of the settings here are not configured by users that
    /// often. While theoretically possible via `Config` methods the more
    /// interesting flags are the ISA ones below. Typically the values here
    /// represent global configuration for wasm features. Settings here
    /// currently rely on the compiler informing us of all settings, including
    /// those disabled. Settings then fall in a few buckets:
    ///
    /// * Some settings must be enabled, such as `avoid_div_traps`.
    /// * Some settings must have a particular value, such as
    ///   `libcall_call_conv`.
    /// * Some settings do not matter as to their value, such as `opt_level`.
    pub(crate) fn check_compatible_with_shared_flag(
        &self,
        flag: &str,
        value: &FlagValue,
    ) -> Result<(), String> {
        let ok = match flag {
            // These settings must all have be enabled, since their value
            // can affect the way the generated code performs or behaves at
            // runtime.
            "avoid_div_traps" => *value == FlagValue::Bool(true),
            "libcall_call_conv" => *value == FlagValue::Enum("isa_default".into()),

            // Features wasmtime doesn't use should all be disabled, since
            // otherwise if they are enabled it could change the behavior of
            // generated code.
            "baldrdash_prologue_words" => *value == FlagValue::Num(0),
            "enable_llvm_abi_extensions" => *value == FlagValue::Bool(false),
            "emit_all_ones_funcaddrs" => *value == FlagValue::Bool(false),
            "enable_pinned_reg" => *value == FlagValue::Bool(false),
            "enable_probestack" => *value == FlagValue::Bool(false),
            "use_colocated_libcalls" => *value == FlagValue::Bool(false),
            "use_pinned_reg_as_heap_base" => *value == FlagValue::Bool(false),

            // If reference types are enabled this must be enabled, otherwise
            // this setting can have any value.
            "enable_safepoints" => {
                if self.config().features.reference_types {
                    *value == FlagValue::Bool(true)
                } else {
                    return Ok(())
                }
            }

            // If reference types or backtraces are enabled, we need unwind info. Otherwise, we
            // don't care.
            "unwind_info" => {
                if self.config().wasm_backtrace || self.config().features.reference_types {
                    *value == FlagValue::Bool(true)
                } else {
                    return Ok(())
                }
            }

            // These settings don't affect the interface or functionality of
            // the module itself, so their configuration values shouldn't
            // matter.
            "enable_heap_access_spectre_mitigation"
            | "enable_table_access_spectre_mitigation"
            | "enable_nan_canonicalization"
            | "enable_jump_tables"
            | "enable_float"
            | "enable_simd"
            | "enable_verifier"
            | "regalloc_checker"
            | "is_pic"
            | "machine_code_cfg_info"
            | "tls_model" // wasmtime doesn't use tls right now
            | "opt_level" // opt level doesn't change semantics
            | "enable_alias_analysis" // alias analysis-based opts don't change semantics
            | "probestack_func_adjusts_sp" // probestack above asserted disabled
            | "probestack_size_log2" // probestack above asserted disabled
            | "regalloc" // shouldn't change semantics
            | "enable_atomics" => return Ok(()),

            // Everything else is unknown and needs to be added somewhere to
            // this list if encountered.
            _ => {
                return Err(format!("unknown shared setting {:?} configured to {:?}", flag, value))
            }
        };

        if !ok {
            return Err(format!(
                "setting {:?} is configured to {:?} which is not supported",
                flag, value,
            ));
        }
        Ok(())
    }

    /// Same as `check_compatible_with_native_host` except used for ISA-specific
    /// flags. This is used to test whether a configured ISA flag is indeed
    /// available on the host platform itself.
    pub(crate) fn check_compatible_with_isa_flag(
        &self,
        flag: &str,
        value: &FlagValue,
    ) -> Result<(), String> {
        match value {
            // ISA flags are used for things like CPU features, so if they're
            // disabled then it's compatible with the native host.
            FlagValue::Bool(false) => return Ok(()),

            // Fall through below where we test at runtime that features are
            // available.
            FlagValue::Bool(true) => {}

            // Only `bool` values are supported right now, other settings would
            // need more support here.
            _ => {
                return Err(format!(
                    "isa-specific feature {:?} configured to unknown value {:?}",
                    flag, value
                ))
            }
        }

        #[allow(unused_assignments)]
        let mut enabled = None;

        #[cfg(target_arch = "aarch64")]
        {
            enabled = match flag {
                "has_lse" => Some(std::arch::is_aarch64_feature_detected!("lse")),
                // fall through to the very bottom to indicate that support is
                // not enabled to test whether this feature is enabled on the
                // host.
                _ => None,
            };
        }

        // There is no is_s390x_feature_detected macro yet, so for now
        // we use getauxval from the libc crate directly.
        #[cfg(all(target_arch = "s390x", target_os = "linux"))]
        {
            let v = unsafe { libc::getauxval(libc::AT_HWCAP) };
            const HWCAP_S390X_VXRS_EXT2: libc::c_ulong = 32768;

            enabled = match flag {
                // There is no separate HWCAP bit for mie2, so assume
                // that any machine with vxrs_ext2 also has mie2.
                "has_vxrs_ext2" | "has_mie2" => Some((v & HWCAP_S390X_VXRS_EXT2) != 0),
                // fall through to the very bottom to indicate that support is
                // not enabled to test whether this feature is enabled on the
                // host.
                _ => None,
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            enabled = match flag {
                "has_sse3" => Some(std::is_x86_feature_detected!("sse3")),
                "has_ssse3" => Some(std::is_x86_feature_detected!("ssse3")),
                "has_sse41" => Some(std::is_x86_feature_detected!("sse4.1")),
                "has_sse42" => Some(std::is_x86_feature_detected!("sse4.2")),
                "has_popcnt" => Some(std::is_x86_feature_detected!("popcnt")),
                "has_avx" => Some(std::is_x86_feature_detected!("avx")),
                "has_avx2" => Some(std::is_x86_feature_detected!("avx2")),
                "has_bmi1" => Some(std::is_x86_feature_detected!("bmi1")),
                "has_bmi2" => Some(std::is_x86_feature_detected!("bmi2")),
                "has_avx512bitalg" => Some(std::is_x86_feature_detected!("avx512bitalg")),
                "has_avx512dq" => Some(std::is_x86_feature_detected!("avx512dq")),
                "has_avx512f" => Some(std::is_x86_feature_detected!("avx512f")),
                "has_avx512vl" => Some(std::is_x86_feature_detected!("avx512vl")),
                "has_avx512vbmi" => Some(std::is_x86_feature_detected!("avx512vbmi")),
                "has_lzcnt" => Some(std::is_x86_feature_detected!("lzcnt")),

                // fall through to the very bottom to indicate that support is
                // not enabled to test whether this feature is enabled on the
                // host.
                _ => None,
            };
        }

        match enabled {
            Some(true) => return Ok(()),
            Some(false) => {
                return Err(format!(
                    "compilation setting {:?} is enabled, but not available on the host",
                    flag
                ))
            }
            // fall through
            None => {}
        }

        Err(format!(
            "cannot test if target-specific flag {:?} is available at runtime",
            flag
        ))
    }
}

impl Default for Engine {
    fn default() -> Engine {
        Engine::new(&Config::default()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Config, Engine, Module, OptLevel};

    use anyhow::Result;
    use tempfile::TempDir;
    use wasmtime_environ::FlagValue;

    #[test]
    fn cache_accounts_for_opt_level() -> Result<()> {
        let td = TempDir::new()?;
        let config_path = td.path().join("config.toml");
        std::fs::write(
            &config_path,
            &format!(
                "
                    [cache]
                    enabled = true
                    directory = '{}'
                ",
                td.path().join("cache").display()
            ),
        )?;
        let mut cfg = Config::new();
        cfg.cranelift_opt_level(OptLevel::None)
            .cache_config_load(&config_path)?;
        let engine = Engine::new(&cfg)?;
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 0);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 1);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);

        let mut cfg = Config::new();
        cfg.cranelift_opt_level(OptLevel::Speed)
            .cache_config_load(&config_path)?;
        let engine = Engine::new(&cfg)?;
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 0);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 1);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);

        let mut cfg = Config::new();
        cfg.cranelift_opt_level(OptLevel::SpeedAndSize)
            .cache_config_load(&config_path)?;
        let engine = Engine::new(&cfg)?;
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 0);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 1);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);

        // FIXME(#1523) need debuginfo on aarch64 before we run this test there
        if !cfg!(target_arch = "aarch64") {
            let mut cfg = Config::new();
            cfg.debug_info(true).cache_config_load(&config_path)?;
            let engine = Engine::new(&cfg)?;
            Module::new(&engine, "(module (func))")?;
            assert_eq!(engine.config().cache_config.cache_hits(), 0);
            assert_eq!(engine.config().cache_config.cache_misses(), 1);
            Module::new(&engine, "(module (func))")?;
            assert_eq!(engine.config().cache_config.cache_hits(), 1);
            assert_eq!(engine.config().cache_config.cache_misses(), 1);
        }

        Ok(())
    }

    #[test]
    #[cfg(compiler)]
    fn test_disable_backtraces() {
        let engine = Engine::new(
            Config::new()
                .wasm_backtrace(false)
                .wasm_reference_types(false),
        )
        .expect("failed to construct engine");
        assert_eq!(
            engine.compiler().flags().get("unwind_info"),
            Some(&FlagValue::Bool(false)),
            "unwind info should be disabled unless needed"
        );
    }
}
