use crate::address_map::{ModuleAddressMap, ValueLabelsRanges};
use crate::compilation::{Compilation, Relocations, Traps};
use crate::module::Module;
use crate::module_environ::FunctionBodyData;
use cranelift_codegen::{ir, isa};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::DefinedFuncIndex;
use log::{debug, trace, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::hash::Hasher;
use std::io::Write;
use std::path::{Path, PathBuf};

#[macro_use] // for tests
mod config;
mod worker;

pub use config::{create_new_config, CacheConfig};
use worker::Worker;

pub struct ModuleCacheEntry<'config>(Option<ModuleCacheEntryInner<'config>>);

struct ModuleCacheEntryInner<'config> {
    mod_cache_path: PathBuf,
    cache_config: &'config CacheConfig,
}

/// Cached compilation data of a Wasm module.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ModuleCacheData {
    compilation: Compilation,
    relocations: Relocations,
    address_transforms: ModuleAddressMap,
    value_ranges: ValueLabelsRanges,
    stack_slots: PrimaryMap<DefinedFuncIndex, ir::StackSlots>,
    traps: Traps,
}

/// A type alias over the module cache data as a tuple.
pub type ModuleCacheDataTupleType = (
    Compilation,
    Relocations,
    ModuleAddressMap,
    ValueLabelsRanges,
    PrimaryMap<DefinedFuncIndex, ir::StackSlots>,
    Traps,
);

struct Sha256Hasher(Sha256);

impl<'config> ModuleCacheEntry<'config> {
    pub fn new<'data>(
        module: &Module,
        function_body_inputs: &PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,
        isa: &dyn isa::TargetIsa,
        compiler_name: &str,
        generate_debug_info: bool,
        cache_config: &'config CacheConfig,
    ) -> Self {
        if cache_config.enabled() {
            Self(Some(ModuleCacheEntryInner::new(
                module,
                function_body_inputs,
                isa,
                compiler_name,
                generate_debug_info,
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

    pub fn get_data(&self) -> Option<ModuleCacheData> {
        if let Some(inner) = &self.0 {
            inner.get_data().map(|val| {
                inner
                    .cache_config
                    .worker()
                    .on_cache_get_async(&inner.mod_cache_path); // call on success
                val
            })
        } else {
            None
        }
    }

    pub fn update_data(&self, data: &ModuleCacheData) {
        if let Some(inner) = &self.0 {
            if inner.update_data(data).is_some() {
                inner
                    .cache_config
                    .worker()
                    .on_cache_update_async(&inner.mod_cache_path); // call on success
            }
        }
    }
}

impl<'config> ModuleCacheEntryInner<'config> {
    fn new<'data>(
        module: &Module,
        function_body_inputs: &PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,
        isa: &dyn isa::TargetIsa,
        compiler_name: &str,
        generate_debug_info: bool,
        cache_config: &'config CacheConfig,
    ) -> Self {
        let hash = Sha256Hasher::digest(module, function_body_inputs);
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
        let mod_filename = format!(
            "mod-{mod_hash}{mod_dbg}",
            mod_hash = base64::encode_config(&hash, base64::URL_SAFE_NO_PAD), // standard encoding uses '/' which can't be used for filename
            mod_dbg = if generate_debug_info { ".d" } else { "" },
        );
        let mod_cache_path = cache_config
            .directory()
            .join(isa.triple().to_string())
            .join(compiler_dir)
            .join(mod_filename);

        Self {
            mod_cache_path,
            cache_config,
        }
    }

    fn get_data(&self) -> Option<ModuleCacheData> {
        trace!("get_data() for path: {}", self.mod_cache_path.display());
        let compressed_cache_bytes = fs::read(&self.mod_cache_path).ok()?;
        let cache_bytes = zstd::decode_all(&compressed_cache_bytes[..])
            .map_err(|err| warn!("Failed to decompress cached code: {}", err))
            .ok()?;
        bincode::deserialize(&cache_bytes[..])
            .map_err(|err| warn!("Failed to deserialize cached code: {}", err))
            .ok()
    }

    fn update_data(&self, data: &ModuleCacheData) -> Option<()> {
        trace!("update_data() for path: {}", self.mod_cache_path.display());
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
        if fs_write_atomic(&self.mod_cache_path, "mod", &compressed_data) {
            return Some(());
        }

        debug!(
            "Attempting to create the cache directory, because \
             failed to write cached code to disk, path: {}",
            self.mod_cache_path.display(),
        );

        let cache_dir = self.mod_cache_path.parent().unwrap();
        fs::create_dir_all(cache_dir)
            .map_err(|err| {
                warn!(
                    "Failed to create cache directory, path: {}, message: {}",
                    cache_dir.display(),
                    err
                )
            })
            .ok()?;

        if fs_write_atomic(&self.mod_cache_path, "mod", &compressed_data) {
            Some(())
        } else {
            None
        }
    }
}

impl ModuleCacheData {
    pub fn from_tuple(data: ModuleCacheDataTupleType) -> Self {
        Self {
            compilation: data.0,
            relocations: data.1,
            address_transforms: data.2,
            value_ranges: data.3,
            stack_slots: data.4,
            traps: data.5,
        }
    }

    pub fn into_tuple(self) -> ModuleCacheDataTupleType {
        (
            self.compilation,
            self.relocations,
            self.address_transforms,
            self.value_ranges,
            self.stack_slots,
            self.traps,
        )
    }
}

impl Sha256Hasher {
    pub fn digest<'data>(
        module: &Module,
        function_body_inputs: &PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,
    ) -> [u8; 32] {
        let mut hasher = Self(Sha256::new());
        module.hash_for_cache(function_body_inputs, &mut hasher);
        hasher.0.result().into()
    }
}

impl Hasher for Sha256Hasher {
    fn finish(&self) -> u64 {
        panic!("Sha256Hasher doesn't support finish!");
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.input(bytes);
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
