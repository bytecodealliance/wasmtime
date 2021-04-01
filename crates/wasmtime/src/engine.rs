use crate::Config;
use anyhow::Result;
use std::sync::Arc;
#[cfg(feature = "cache")]
use wasmtime_cache::CacheConfig;
use wasmtime_jit::Compiler;
use wasmtime_runtime::{debug_builtins, InstanceAllocator};

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
#[derive(Clone)]
pub struct Engine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    config: Config,
    compiler: Compiler,
    allocator: Box<dyn InstanceAllocator>,
}

impl Engine {
    /// Creates a new [`Engine`] with the specified compilation and
    /// configuration settings.
    pub fn new(config: &Config) -> Result<Engine> {
        debug_builtins::ensure_exported();
        config.validate()?;
        let allocator = config.build_allocator()?;
        Ok(Engine {
            inner: Arc::new(EngineInner {
                config: config.clone(),
                compiler: config.build_compiler(allocator.as_ref()),
                allocator,
            }),
        })
    }

    /// Returns the configuration settings that this engine is using.
    #[inline]
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    pub(crate) fn compiler(&self) -> &Compiler {
        &self.inner.compiler
    }

    pub(crate) fn allocator(&self) -> &dyn InstanceAllocator {
        self.inner.allocator.as_ref()
    }

    #[cfg(feature = "cache")]
    pub(crate) fn cache_config(&self) -> &CacheConfig {
        &self.config().cache_config
    }

    /// Returns whether the engine `a` and `b` refer to the same configuration.
    pub fn same(a: &Engine, b: &Engine) -> bool {
        Arc::ptr_eq(&a.inner, &b.inner)
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
    /// host. The output of this method may be used with [`Module::new`](crate::Module::new)
    /// on hosts compatible with the [`Config`] associated with this [`Engine`].
    ///
    /// The output of this method is safe to send to another host machine for later
    /// execution. As the output is already a compiled module, translation and code
    /// generation will be skipped and this will improve the performance of constructing
    /// a [`Module`](crate::Module) from the output of this method.
    ///
    /// [binary]: https://webassembly.github.io/spec/core/binary/index.html
    /// [text]: https://webassembly.github.io/spec/core/text/index.html
    pub fn precompile_module(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        const USE_PAGED_MEM_INIT: bool = cfg!(all(feature = "uffd", target_os = "linux"));

        #[cfg(feature = "wat")]
        let bytes = wat::parse_bytes(&bytes)?;

        let (_, artifacts, types) = wasmtime_jit::CompilationArtifacts::build(
            &self.inner.compiler,
            &bytes,
            USE_PAGED_MEM_INIT,
        )?;

        crate::module::SerializedModule::from_artifacts(&self.inner.compiler, &artifacts, &types)
            .to_bytes()
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
}
