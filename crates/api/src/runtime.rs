use anyhow::Result;
use std::cell::RefCell;
use std::fmt;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wasmparser::{OperatorValidatorConfig, ValidatingParserConfig};
use wasmtime_environ::settings::{self, Configurable};
use wasmtime_environ::CacheConfig;
use wasmtime_jit::{native, CompilationStrategy, Compiler};
use wasmtime_profiling::{JitDumpAgent, ProfilingAgent, ProfilingStrategy};

// Runtime Environment

// Configuration

/// Global configuration options used to create an [`Engine`] and customize its
/// behavior.
///
/// This structure exposed a builder-like interface and is primarily consumed by
/// [`Engine::new()`]
#[derive(Clone)]
pub struct Config {
    pub(crate) flags: settings::Builder,
    pub(crate) validating_config: ValidatingParserConfig,
    pub(crate) debug_info: bool,
    pub(crate) strategy: CompilationStrategy,
    pub(crate) cache_config: CacheConfig,
    pub(crate) profiler: Option<Arc<Mutex<Box<dyn ProfilingAgent + Send>>>>,
}

impl Config {
    /// Creates a new configuration object with the default configuration
    /// specified.
    pub fn new() -> Config {
        let mut flags = settings::builder();

        // There are two possible traps for division, and this way
        // we get the proper one if code traps.
        flags
            .enable("avoid_div_traps")
            .expect("should be valid flag");

        // Invert cranelift's default-on verification to instead default off.
        flags
            .set("enable_verifier", "false")
            .expect("should be valid flag");

        // Turn on cranelift speed optimizations by default
        flags
            .set("opt_level", "speed")
            .expect("should be valid flag");

        Config {
            debug_info: false,
            validating_config: ValidatingParserConfig {
                operator_config: OperatorValidatorConfig {
                    enable_threads: false,
                    enable_reference_types: false,
                    enable_bulk_memory: false,
                    enable_simd: false,
                    enable_multi_value: false,
                },
            },
            flags,
            strategy: CompilationStrategy::Auto,
            cache_config: CacheConfig::new_cache_disabled(),
            profiler: None,
        }
    }

    /// Configures whether DWARF debug information will be emitted during
    /// compilation.
    ///
    /// By default this option is `false`.
    pub fn debug_info(&mut self, enable: bool) -> &mut Self {
        self.debug_info = enable;
        self
    }

    /// Configures whether the WebAssembly threads proposal will be enabled for
    /// compilation.
    ///
    /// The [WebAssembly threads proposal][threads] is not currently fully
    /// standardized and is undergoing development. Additionally the support in
    /// wasmtime itself is still being worked on. Support for this feature can
    /// be enabled through this method for appropriate wasm modules.
    ///
    /// This feature gates items such as shared memories and atomic
    /// instructions. Note that enabling the threads feature will
    /// also enable the bulk memory feature.
    ///
    /// This is `false` by default.
    ///
    /// [threads]: https://github.com/webassembly/threads
    pub fn wasm_threads(&mut self, enable: bool) -> &mut Self {
        self.validating_config.operator_config.enable_threads = enable;
        // The threads proposal depends on the bulk memory proposal
        if enable {
            self.wasm_bulk_memory(true);
        }
        self
    }

    /// Configures whether the WebAssembly reference types proposal will be
    /// enabled for compilation.
    ///
    /// The [WebAssembly reference types proposal][proposal] is not currently
    /// fully standardized and is undergoing development. Additionally the
    /// support in wasmtime itself is still being worked on. Support for this
    /// feature can be enabled through this method for appropriate wasm
    /// modules.
    ///
    /// This feature gates items such as the `anyref` type and multiple tables
    /// being in a module. Note that enabling the reference types feature will
    /// also enable the bulk memory feature.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/reference-types
    pub fn wasm_reference_types(&mut self, enable: bool) -> &mut Self {
        self.validating_config
            .operator_config
            .enable_reference_types = enable;
        // The reference types proposal depends on the bulk memory proposal
        if enable {
            self.wasm_bulk_memory(true);
        }
        self
    }

    /// Configures whether the WebAssembly SIMD proposal will be
    /// enabled for compilation.
    ///
    /// The [WebAssembly SIMD proposal][proposal] is not currently
    /// fully standardized and is undergoing development. Additionally the
    /// support in wasmtime itself is still being worked on. Support for this
    /// feature can be enabled through this method for appropriate wasm
    /// modules.
    ///
    /// This feature gates items such as the `v128` type and all of its
    /// operators being in a module.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/simd
    pub fn wasm_simd(&mut self, enable: bool) -> &mut Self {
        self.validating_config.operator_config.enable_simd = enable;
        let val = if enable { "true" } else { "false" };
        self.flags
            .set("enable_simd", val)
            .expect("should be valid flag");
        self
    }

    /// Configures whether the WebAssembly bulk memory operations proposal will
    /// be enabled for compilation.
    ///
    /// The [WebAssembly bulk memory operations proposal][proposal] is not
    /// currently fully standardized and is undergoing development.
    /// Additionally the support in wasmtime itself is still being worked on.
    /// Support for this feature can be enabled through this method for
    /// appropriate wasm modules.
    ///
    /// This feature gates items such as the `memory.copy` instruction, passive
    /// data/table segments, etc, being in a module.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/bulk-memory-operations
    pub fn wasm_bulk_memory(&mut self, enable: bool) -> &mut Self {
        self.validating_config.operator_config.enable_bulk_memory = enable;
        self
    }

    /// Configures whether the WebAssembly multi-value proposal will
    /// be enabled for compilation.
    ///
    /// The [WebAssembly multi-value proposal][proposal] is not
    /// currently fully standardized and is undergoing development.
    /// Additionally the support in wasmtime itself is still being worked on.
    /// Support for this feature can be enabled through this method for
    /// appropriate wasm modules.
    ///
    /// This feature gates functions and blocks returning multiple values in a
    /// module, for example.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/multi-value
    pub fn wasm_multi_value(&mut self, enable: bool) -> &mut Self {
        self.validating_config.operator_config.enable_multi_value = enable;
        self
    }

    /// Configures which compilation strategy will be used for wasm modules.
    ///
    /// This method can be used to configure which compiler is used for wasm
    /// modules, and for more documentation consult the [`Strategy`] enumeration
    /// and its documentation.
    ///
    /// The default value for this is `Strategy::Auto`.
    ///
    /// # Errors
    ///
    /// Some compilation strategies require compile-time options of `wasmtime`
    /// itself to be set, but if they're not set and the strategy is specified
    /// here then an error will be returned.
    pub fn strategy(&mut self, strategy: Strategy) -> Result<&mut Self> {
        self.strategy = match strategy {
            Strategy::Auto => CompilationStrategy::Auto,
            Strategy::Cranelift => CompilationStrategy::Cranelift,
            #[cfg(feature = "lightbeam")]
            Strategy::Lightbeam => CompilationStrategy::Lightbeam,
            #[cfg(not(feature = "lightbeam"))]
            Strategy::Lightbeam => {
                anyhow::bail!("lightbeam compilation strategy wasn't enabled at compile time");
            }
        };
        Ok(self)
    }

    /// Creates a default profiler based on the profiling strategy choosen
    ///
    /// Profiler creation calls the type's default initializer where the purpose is
    /// really just to put in place the type used for profiling.
    pub fn profiler(&mut self, profile: ProfilingStrategy) -> Result<&mut Self> {
        match profile {
            ProfilingStrategy::JitDumpProfiler => {
                self.profiler = { Some(Arc::new(Mutex::new(Box::new(JitDumpAgent::default())))) }
            }
            _ => self.profiler = { None },
        };
        Ok(self)
    }

    /// Configures whether the debug verifier of Cranelift is enabled or not.
    ///
    /// When Cranelift is used as a code generation backend this will configure
    /// it to have the `enable_verifier` flag which will enable a number of debug
    /// checks inside of Cranelift. This is largely only useful for the
    /// developers of wasmtime itself.
    ///
    /// The default value for this is `false`
    pub fn cranelift_debug_verifier(&mut self, enable: bool) -> &mut Self {
        let val = if enable { "true" } else { "false" };
        self.flags
            .set("enable_verifier", val)
            .expect("should be valid flag");
        self
    }

    /// Configures the Cranelift code generator optimization level.
    ///
    /// When the Cranelift code generator is used you can configure the
    /// optimization level used for generated code in a few various ways. For
    /// more information see the documentation of [`OptLevel`].
    ///
    /// The default value for this is `OptLevel::None`.
    pub fn cranelift_opt_level(&mut self, level: OptLevel) -> &mut Self {
        let val = match level {
            OptLevel::None => "none",
            OptLevel::Speed => "speed",
            OptLevel::SpeedAndSize => "speed_and_size",
        };
        self.flags
            .set("opt_level", val)
            .expect("should be valid flag");
        self
    }

    /// Loads cache configuration specified at `path`.
    ///
    /// This method will read the file specified by `path` on the filesystem and
    /// attempt to load cache configuration from it. This method can also fail
    /// due to I/O errors, misconfiguration, syntax errors, etc. For expected
    /// syntax in the configuration file see the [documentation online][docs].
    ///
    /// By default cache configuration is not enabled or loaded.
    ///
    /// # Errors
    ///
    /// This method can fail due to any error that happens when loading the file
    /// pointed to by `path` and attempting to load the cache configuration.
    ///
    /// [docs]: https://bytecodealliance.github.io/wasmtime/cli-cache.html
    pub fn cache_config_load(&mut self, path: impl AsRef<Path>) -> Result<&mut Self> {
        self.cache_config = wasmtime_environ::CacheConfig::from_file(Some(path.as_ref()))?;
        Ok(self)
    }

    /// Loads cache configuration from the system default path.
    ///
    /// This commit is the same as [`Config::cache_config_load`] except that it
    /// does not take a path argument and instead loads the default
    /// configuration present on the system. This is located, for example, on
    /// Unix at `$HOME/.config/wasmtime/config.toml` and is typically created
    /// with the `wasmtime config new` command.
    ///
    /// By default cache configuration is not enabled or loaded.
    ///
    /// # Errors
    ///
    /// This method can fail due to any error that happens when loading the
    /// default system configuration. Note that it is not an error if the
    /// default config file does not exist, in which case the default settings
    /// for an enabled cache are applied.
    ///
    /// [docs]: https://bytecodealliance.github.io/wasmtime/cli-cache.html
    pub fn cache_config_load_default(&mut self) -> Result<&mut Self> {
        self.cache_config = wasmtime_environ::CacheConfig::from_file(None)?;
        Ok(self)
    }
}

impl Default for Config {
    fn default() -> Config {
        Config::new()
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let features = &self.validating_config.operator_config;
        f.debug_struct("Config")
            .field("debug_info", &self.debug_info)
            .field("strategy", &self.strategy)
            .field("wasm_threads", &features.enable_threads)
            .field("wasm_reference_types", &features.enable_reference_types)
            .field("wasm_bulk_memory", &features.enable_bulk_memory)
            .field("wasm_simd", &features.enable_simd)
            .field("wasm_multi_value", &features.enable_multi_value)
            .field(
                "flags",
                &settings::Flags::new(self.flags.clone()).to_string(),
            )
            .finish()
    }
}

/// Possible Compilation strategies for a wasm module.
///
/// This is used as an argument to the [`Config::strategy`] method.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum Strategy {
    /// An indicator that the compilation strategy should be automatically
    /// selected.
    ///
    /// This is generally what you want for most projects and indicates that the
    /// `wasmtime` crate itself should make the decision about what the best
    /// code generator for a wasm module is.
    ///
    /// Currently this always defaults to Cranelift, but the default value will
    /// change over time.
    Auto,

    /// Currently the default backend, Cranelift aims to be a reasonably fast
    /// code generator which generates high quality machine code.
    Cranelift,

    /// A single-pass code generator that is faster than Cranelift but doesn't
    /// produce as high-quality code.
    ///
    /// To successfully pass this argument to [`Config::strategy`] the
    /// `lightbeam` feature of this crate must be enabled.
    Lightbeam,
}

/// Possible optimization levels for the Cranelift codegen backend.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum OptLevel {
    /// No optimizations performed, minimizes compilation time by disabling most
    /// optimizations.
    None,
    /// Generates the fastest possible code, but may take longer.
    Speed,
    /// Similar to `speed`, but also performs transformations aimed at reducing
    /// code size.
    SpeedAndSize,
}

// Engine

/// An `Engine` which is a global context for compilation and management of wasm
/// modules.
///
/// An engine can be safely shared across threads and is a cheap cloneable
/// handle to the actual engine. The engine itself will be deallocate once all
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
#[derive(Default, Clone)]
pub struct Engine {
    config: Arc<Config>,
}

impl Engine {
    /// Creates a new [`Engine`] with the specified compilation and
    /// configuration settings.
    pub fn new(config: &Config) -> Engine {
        Engine {
            config: Arc::new(config.clone()),
        }
    }

    /// Returns the configuration settings that this engine is using.
    pub fn config(&self) -> &Config {
        &self.config
    }
}

// Store

/// A `Store` is a shared cache of information between WebAssembly modules.
///
/// Each `Module` is compiled into a `Store` and a `Store` is associated with an
/// [`Engine`]. You'll use a `Store` to attach to a number of global items in
/// the production of various items for wasm modules.
///
/// # Stores and `Clone`
///
/// Using `clone` on a `Store` is a cheap operation. It will not create an
/// entirely new store, but rather just a new reference to the existing object.
/// In other words it's a shallow copy, not a deep copy.
///
/// ## Stores and `Default`
///
/// You can create a store with default configuration settings using
/// `Store::default()`. This will create a brand new [`Engine`] with default
/// ocnfiguration (see [`Config`] for more information).
#[derive(Clone)]
pub struct Store {
    // FIXME(#777) should be `Arc` and this type should be thread-safe
    inner: Rc<StoreInner>,
}

struct StoreInner {
    engine: Engine,
    compiler: RefCell<Compiler>,
}

impl Store {
    /// Creates a new store to be associated with the given [`Engine`].
    pub fn new(engine: &Engine) -> Store {
        let isa = native::builder().finish(settings::Flags::new(engine.config.flags.clone()));
        let compiler = Compiler::new(
            isa,
            engine.config.strategy,
            engine.config.cache_config.clone(),
        );
        Store {
            inner: Rc::new(StoreInner {
                engine: engine.clone(),
                compiler: RefCell::new(compiler),
            }),
        }
    }

    /// Returns the [`Engine`] that this store is associated with.
    pub fn engine(&self) -> &Engine {
        &self.inner.engine
    }

    pub(crate) fn compiler(&self) -> std::cell::Ref<'_, Compiler> {
        self.inner.compiler.borrow()
    }

    pub(crate) fn compiler_mut(&self) -> std::cell::RefMut<'_, Compiler> {
        self.inner.compiler.borrow_mut()
    }

    /// Returns whether the stores `a` and `b` refer to the same underlying
    /// `Store`.
    ///
    /// Because the `Store` type is reference counted multiple clones may point
    /// to the same underlying storage, and this method can be used to determine
    /// whether two stores are indeed the same.
    pub fn same(a: &Store, b: &Store) -> bool {
        Rc::ptr_eq(&a.inner, &b.inner)
    }
}

impl Default for Store {
    fn default() -> Store {
        Store::new(&Engine::default())
    }
}

fn _assert_send_sync() {
    fn _assert<T: Send + Sync>() {}
    _assert::<Engine>();
    _assert::<Config>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Module;
    use tempfile::TempDir;

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
        let store = Store::new(&Engine::new(&cfg));
        Module::new(&store, "(module (func))")?;
        assert_eq!(store.engine().config.cache_config.cache_hits(), 0);
        assert_eq!(store.engine().config.cache_config.cache_misses(), 1);
        Module::new(&store, "(module (func))")?;
        assert_eq!(store.engine().config.cache_config.cache_hits(), 1);
        assert_eq!(store.engine().config.cache_config.cache_misses(), 1);

        let mut cfg = Config::new();
        cfg.cranelift_opt_level(OptLevel::Speed)
            .cache_config_load(&config_path)?;
        let store = Store::new(&Engine::new(&cfg));
        Module::new(&store, "(module (func))")?;
        assert_eq!(store.engine().config.cache_config.cache_hits(), 0);
        assert_eq!(store.engine().config.cache_config.cache_misses(), 1);
        Module::new(&store, "(module (func))")?;
        assert_eq!(store.engine().config.cache_config.cache_hits(), 1);
        assert_eq!(store.engine().config.cache_config.cache_misses(), 1);

        let mut cfg = Config::new();
        cfg.cranelift_opt_level(OptLevel::SpeedAndSize)
            .cache_config_load(&config_path)?;
        let store = Store::new(&Engine::new(&cfg));
        Module::new(&store, "(module (func))")?;
        assert_eq!(store.engine().config.cache_config.cache_hits(), 0);
        assert_eq!(store.engine().config.cache_config.cache_misses(), 1);
        Module::new(&store, "(module (func))")?;
        assert_eq!(store.engine().config.cache_config.cache_hits(), 1);
        assert_eq!(store.engine().config.cache_config.cache_misses(), 1);

        let mut cfg = Config::new();
        cfg.debug_info(true).cache_config_load(&config_path)?;
        let store = Store::new(&Engine::new(&cfg));
        Module::new(&store, "(module (func))")?;
        assert_eq!(store.engine().config.cache_config.cache_hits(), 0);
        assert_eq!(store.engine().config.cache_config.cache_misses(), 1);
        Module::new(&store, "(module (func))")?;
        assert_eq!(store.engine().config.cache_config.cache_hits(), 1);
        assert_eq!(store.engine().config.cache_config.cache_misses(), 1);

        Ok(())
    }
}
