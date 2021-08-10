use log::{debug, trace, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::hash::Hash;
use std::hash::Hasher;
use std::io::Write;
use std::path::{Path, PathBuf};

#[macro_use] // for tests
mod config;
mod worker;

pub use config::{create_new_config, CacheConfig};
use worker::Worker;

/// Module level cache entry.
pub struct ModuleCacheEntry<'config>(Option<ModuleCacheEntryInner<'config>>);

struct ModuleCacheEntryInner<'config> {
    root_path: PathBuf,
    cache_config: &'config CacheConfig,
}

struct Sha256Hasher(Sha256);

impl<'config> ModuleCacheEntry<'config> {
    /// Create the cache entry.
    pub fn new<'data>(compiler_name: &str, cache_config: &'config CacheConfig) -> Self {
        if cache_config.enabled() {
            Self(Some(ModuleCacheEntryInner::new(
                compiler_name,
                cache_config,
            )))
        } else {
            Self(None)
        }
    }

    #[cfg(test)]
    fn from_inner(inner: ModuleCacheEntryInner<'config>) -> Self {
        Self(Some(inner))
    }

    /// Gets cached data if state matches, otherwise calls the `compute`.
    // NOTE: This takes a function pointer instead of a closure so that it doesn't accidentally
    // close over something not accounted in the cache.
    pub fn get_data<T, U, E>(&self, state: T, compute: fn(T) -> Result<U, E>) -> Result<U, E>
    where
        T: Hash,
        U: Serialize + for<'a> Deserialize<'a>,
    {
        let inner = match &self.0 {
            Some(inner) => inner,
            None => return compute(state),
        };

        let mut hasher = Sha256Hasher(Sha256::new());
        state.hash(&mut hasher);
        let hash: [u8; 32] = hasher.0.finalize().into();
        // standard encoding uses '/' which can't be used for filename
        let hash = base64::encode_config(&hash, base64::URL_SAFE_NO_PAD);

        if let Some(cached_val) = inner.get_data(&hash) {
            let mod_cache_path = inner.root_path.join(&hash);
            inner.cache_config.on_cache_get_async(&mod_cache_path); // call on success
            return Ok(cached_val);
        }
        let val_to_cache = compute(state)?;
        if inner.update_data(&hash, &val_to_cache).is_some() {
            let mod_cache_path = inner.root_path.join(&hash);
            inner.cache_config.on_cache_update_async(&mod_cache_path); // call on success
        }
        Ok(val_to_cache)
    }
}

impl<'config> ModuleCacheEntryInner<'config> {
    fn new<'data>(compiler_name: &str, cache_config: &'config CacheConfig) -> Self {
        // If debug assertions are enabled then assume that we're some sort of
        // local build. We don't want local builds to stomp over caches between
        // builds, so just use a separate cache directory based on the mtime of
        // our executable, which should roughly correlate with "you changed the
        // source code so you get a different directory".
        //
        // Otherwise if this is a release build we use the `GIT_REV` env var
        // which is either the git rev if installed from git or the crate
        // version if installed from crates.io.
        let compiler_dir = if cfg!(debug_assertions) {
            fn self_mtime() -> Option<String> {
                let path = std::env::current_exe().ok()?;
                let metadata = path.metadata().ok()?;
                let mtime = metadata.modified().ok()?;
                Some(match mtime.duration_since(std::time::UNIX_EPOCH) {
                    Ok(dur) => format!("{}", dur.as_millis()),
                    Err(err) => format!("m{}", err.duration().as_millis()),
                })
            }
            let self_mtime = self_mtime().unwrap_or("no-mtime".to_string());
            format!(
                "{comp_name}-{comp_ver}-{comp_mtime}",
                comp_name = compiler_name,
                comp_ver = env!("GIT_REV"),
                comp_mtime = self_mtime,
            )
        } else {
            format!(
                "{comp_name}-{comp_ver}",
                comp_name = compiler_name,
                comp_ver = env!("GIT_REV"),
            )
        };
        let root_path = cache_config.directory().join("modules").join(compiler_dir);

        Self {
            root_path,
            cache_config,
        }
    }

    fn get_data<T>(&self, hash: &str) -> Option<T>
    where
        T: for<'a> Deserialize<'a>,
    {
        let mod_cache_path = self.root_path.join(hash);
        trace!("get_data() for path: {}", mod_cache_path.display());
        let compressed_cache_bytes = fs::read(&mod_cache_path).ok()?;
        let cache_bytes = zstd::decode_all(&compressed_cache_bytes[..])
            .map_err(|err| warn!("Failed to decompress cached code: {}", err))
            .ok()?;
        bincode::deserialize(&cache_bytes[..])
            .map_err(|err| warn!("Failed to deserialize cached code: {}", err))
            .ok()
    }

    fn update_data<T: Serialize>(&self, hash: &str, data: &T) -> Option<()> {
        let mod_cache_path = self.root_path.join(hash);
        trace!("update_data() for path: {}", mod_cache_path.display());
        let serialized_data = bincode::serialize(&data)
            .map_err(|err| warn!("Failed to serialize cached code: {}", err))
            .ok()?;
        let compressed_data = zstd::encode_all(
            &serialized_data[..],
            self.cache_config.baseline_compression_level(),
        )
        .map_err(|err| warn!("Failed to compress cached code: {}", err))
        .ok()?;

        // Optimize syscalls: first, try writing to disk. It should succeed in most cases.
        // Otherwise, try creating the cache directory and retry writing to the file.
        if fs_write_atomic(&mod_cache_path, "mod", &compressed_data) {
            return Some(());
        }

        debug!(
            "Attempting to create the cache directory, because \
             failed to write cached code to disk, path: {}",
            mod_cache_path.display(),
        );

        let cache_dir = mod_cache_path.parent().unwrap();
        fs::create_dir_all(cache_dir)
            .map_err(|err| {
                warn!(
                    "Failed to create cache directory, path: {}, message: {}",
                    cache_dir.display(),
                    err
                )
            })
            .ok()?;

        if fs_write_atomic(&mod_cache_path, "mod", &compressed_data) {
            Some(())
        } else {
            None
        }
    }
}

impl Hasher for Sha256Hasher {
    fn finish(&self) -> u64 {
        panic!("Sha256Hasher doesn't support finish!");
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }
}

// Assumption: path inside cache directory.
// Then, we don't have to use sound OS-specific exclusive file access.
// Note: there's no need to remove temporary file here - cleanup task will do it later.
fn fs_write_atomic(path: &Path, reason: &str, contents: &[u8]) -> bool {
    let lock_path = path.with_extension(format!("wip-atomic-write-{}", reason));
    fs::OpenOptions::new()
        .create_new(true) // atomic file creation (assumption: no one will open it without this flag)
        .write(true)
        .open(&lock_path)
        .and_then(|mut file| file.write_all(contents))
        // file should go out of scope and be closed at this point
        .and_then(|()| fs::rename(&lock_path, &path)) // atomic file rename
        .map_err(|err| {
            warn!(
                "Failed to write file with rename, lock path: {}, target path: {}, err: {}",
                lock_path.display(),
                path.display(),
                err
            )
        })
        .is_ok()
}

#[cfg(test)]
mod tests;
