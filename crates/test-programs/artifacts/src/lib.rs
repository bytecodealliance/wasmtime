include!(concat!(env!("OUT_DIR"), "/gen.rs"));

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::IsTerminal;
use std::sync::{Arc, Mutex};
use wasmtime::{CacheStore, Config, Engine};

/// The wasi-tests binaries use these environment variables to determine their
/// expected behavior.
/// Used by all of the tests/ which execute the wasi-tests binaries.
pub fn wasi_tests_environment() -> &'static [(&'static str, &'static str)] {
    #[cfg(windows)]
    {
        &[
            ("ERRNO_MODE_WINDOWS", "1"),
            // Windows does not support dangling links or symlinks in the filesystem.
            ("NO_DANGLING_FILESYSTEM", "1"),
            // Windows does not support renaming a directory to an empty directory -
            // empty directory must be deleted.
            ("NO_RENAME_DIR_TO_EMPTY_DIR", "1"),
        ]
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        &[("ERRNO_MODE_UNIX", "1")]
    }
    #[cfg(target_os = "macos")]
    {
        &[("ERRNO_MODE_MACOS", "1")]
    }
}

pub fn stdio_is_terminal() -> bool {
    std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
        && std::io::stderr().is_terminal()
}

// Simple incremental cache used during tests to help improve test runtime.
//
// Many tests take a similar module (e.g. a component, a preview1 thing, sync,
// async, etc) and run it in different contexts and this improve cache hit rates
// across usages by sharing one incremental cache across tests.
fn cache_store() -> Arc<dyn CacheStore> {
    #[derive(Debug)]
    struct MyCache;

    static CACHE: Mutex<Option<HashMap<Vec<u8>, Vec<u8>>>> = Mutex::new(None);

    impl CacheStore for MyCache {
        fn get(&self, key: &[u8]) -> Option<Cow<[u8]>> {
            let mut cache = CACHE.lock().unwrap();
            let cache = cache.get_or_insert_with(HashMap::new);
            cache.get(key).map(|s| s.to_vec().into())
        }

        fn insert(&self, key: &[u8], value: Vec<u8>) -> bool {
            let mut cache = CACHE.lock().unwrap();
            let cache = cache.get_or_insert_with(HashMap::new);
            cache.insert(key.to_vec(), value);
            true
        }
    }

    Arc::new(MyCache)
}

/// Helper to create an `Engine` with a pre-configured `Config` that uses a
/// cache for faster building of modules.
pub fn engine(configure: impl FnOnce(&mut Config)) -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config
        .enable_incremental_compilation(cache_store())
        .unwrap();
    configure(&mut config);
    Engine::new(&config).unwrap()
}
