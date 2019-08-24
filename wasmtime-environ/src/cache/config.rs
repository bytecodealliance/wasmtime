//! Module for configuring the cache system.

use directories::ProjectDirs;
use lazy_static::lazy_static;
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use spin::Once;
use std::fmt::Debug;
use std::fs;
use std::mem;
use std::path::{Path, PathBuf};
use std::string::{String, ToString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::vec::Vec;

// wrapped, so we have named section in config,
// also, for possible future compatibility
#[derive(Serialize, Deserialize)]
struct Config {
    cache: CacheConfig,
}

#[derive(Serialize, Deserialize)]
struct CacheConfig {
    #[serde(skip)]
    pub errors: Vec<String>,

    pub enabled: bool,
    pub directory: Option<PathBuf>,
    #[serde(rename = "baseline-compression-level")]
    pub baseline_compression_level: Option<i32>,
}

// Private static, so only internal function can access it.
static CONFIG: Once<CacheConfig> = Once::new();
static INIT_CALLED: AtomicBool = AtomicBool::new(false);

/// Returns true if and only if the cache is enabled.
pub fn enabled() -> bool {
    // Not everyone knows about the cache system, i.e. the tests,
    // so the default is cache disabled.
    CONFIG
        .call_once(|| CacheConfig::new_cache_disabled())
        .enabled
}

/// Returns path to the cache directory.
///
/// Panics if the cache is disabled.
pub fn directory() -> &'static PathBuf {
    &CONFIG
        .r#try()
        .expect("Cache system must be initialized")
        .directory
        .as_ref()
        .unwrap()
}

/// Returns cache compression level.
///
/// Panics if the cache is disabled.
pub fn baseline_compression_level() -> i32 {
    CONFIG
        .r#try()
        .expect("Cache system must be initialized")
        .baseline_compression_level
        .unwrap()
}

/// Initializes the cache system. Should be called exactly once,
/// and before using the cache system. Otherwise it can panic.
/// Returns list of errors. If empty, initialization succeeded.
pub fn init<P: AsRef<Path> + Debug>(
    enabled: bool,
    config_file: Option<P>,
    create_new_config: bool,
) -> &'static Vec<String> {
    INIT_CALLED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .expect("Cache system init must be called at most once");
    assert!(
        CONFIG.r#try().is_none(),
        "Cache system init must be called before using the system."
    );
    let conf_file_str = format!("{:?}", config_file);
    let conf = CONFIG.call_once(|| CacheConfig::from_file(enabled, config_file, create_new_config));
    if conf.errors.is_empty() {
        debug!(
            "Cache init(\"{}\"): enabled={}, directory={:?}, baseline-compression-level={:?}",
            conf_file_str, conf.enabled, conf.directory, conf.baseline_compression_level,
        )
    } else {
        error!(
            "Cache init(\"{}\"): errors: {:#?}",
            conf_file_str, conf.errors,
        )
    }
    &conf.errors
}

// permitted levels from: https://docs.rs/zstd/0.4.28+zstd.1.4.3/zstd/stream/write/struct.Encoder.html
const ZSTD_COMPRESSION_LEVELS: std::ops::RangeInclusive<i32> = 0..=21;
lazy_static! {
    static ref PROJECT_DIRS: Option<ProjectDirs> =
        ProjectDirs::from("", "CraneStation", "wasmtime");
}

impl CacheConfig {
    pub fn new_cache_disabled() -> Self {
        Self {
            errors: Vec::new(),
            enabled: false,
            directory: None,
            baseline_compression_level: None,
        }
    }

    fn new_cache_enabled_template() -> Self {
        let mut conf = Self::new_cache_disabled();
        conf.enabled = true;
        conf
    }

    fn new_cache_with_errors(errors: Vec<String>) -> Self {
        let mut conf = Self::new_cache_disabled();
        conf.errors = errors;
        conf
    }

    pub fn from_file<P: AsRef<Path>>(
        enabled: bool,
        config_file: Option<P>,
        create_new_config: bool,
    ) -> Self {
        if !enabled {
            return Self::new_cache_disabled();
        }

        let (mut config, path_if_flush_to_disk) =
            match Self::load_and_parse_file(config_file, create_new_config) {
                Ok(data) => data,
                Err(err) => return Self::new_cache_with_errors(vec![err]),
            };

        // validate values and fill in defaults
        config.validate_cache_directory_or_default();
        config.validate_baseline_compression_level_or_default();

        path_if_flush_to_disk.map(|p| config.flush_to_disk(p));

        config.disable_if_any_error();
        config
    }

    fn load_and_parse_file<P: AsRef<Path>>(
        config_file: Option<P>,
        create_new_config: bool,
    ) -> Result<(Self, Option<PathBuf>), String> {
        // get config file path
        let (config_file, user_custom_file) = match config_file {
            Some(p) => (PathBuf::from(p.as_ref()), true),
            None => match &*PROJECT_DIRS {
                Some(proj_dirs) => (
                    proj_dirs.config_dir().join("wasmtime-cache-config.toml"),
                    false,
                ),
                None => Err("Config file not specified and failed to get the default".to_string())?,
            },
        };

        // read config, or create an empty one
        let entity_exists = config_file.exists();
        match (create_new_config, entity_exists, user_custom_file) {
            (true, true, _) => Err(format!(
                "Tried to create a new config, but given entity already exists, path: {}",
                config_file.display()
            )),
            (true, false, _) => Ok((Self::new_cache_enabled_template(), Some(config_file))),
            (false, false, false) => Ok((Self::new_cache_enabled_template(), None)),
            (false, _, _) => match fs::read(&config_file) {
                Ok(bytes) => match toml::from_slice::<Config>(&bytes[..]) {
                    Ok(config) => Ok((config.cache, None)),
                    Err(err) => Err(format!(
                        "Failed to parse config file, path: {}, error: {}",
                        config_file.display(),
                        err
                    )),
                },
                Err(err) => Err(format!(
                    "Failed to read config file, path: {}, error: {}",
                    config_file.display(),
                    err
                )),
            },
        }
    }

    fn validate_cache_directory_or_default(&mut self) {
        if self.directory.is_none() {
            match &*PROJECT_DIRS {
                Some(proj_dirs) => self.directory = Some(proj_dirs.cache_dir().to_path_buf()),
                None => {
                    self.errors.push(
                        "Cache directory not specified and failed to get the default".to_string(),
                    );
                    return;
                }
            }
        }

        // On Windows, if we want long paths, we need '\\?\' prefix, but it doesn't work
        // with relative paths. One way to get absolute path (the only one?) is to use
        // fs::canonicalize, but it requires that given path exists. The extra advantage
        // of this method is fact that the method prepends '\\?\' on Windows.
        let cache_dir = self.directory.as_ref().unwrap();

        if !cache_dir.is_absolute() {
            self.errors.push(format!(
                "Cache directory path has to be absolute, path: {}",
                cache_dir.display(),
            ));
            return;
        }

        match fs::create_dir_all(cache_dir) {
            Ok(()) => (),
            Err(err) => {
                self.errors.push(format!(
                    "Failed to create the cache directory, path: {}, error: {}",
                    cache_dir.display(),
                    err
                ));
                return;
            }
        };

        match fs::canonicalize(cache_dir) {
            Ok(p) => self.directory = Some(p),
            Err(err) => {
                self.errors.push(format!(
                    "Failed to canonicalize the cache directory, path: {}, error: {}",
                    cache_dir.display(),
                    err
                ));
            }
        }
    }

    fn validate_baseline_compression_level_or_default(&mut self) {
        if self.baseline_compression_level.is_none() {
            self.baseline_compression_level = Some(zstd::DEFAULT_COMPRESSION_LEVEL);
        }

        if !ZSTD_COMPRESSION_LEVELS.contains(&self.baseline_compression_level.unwrap()) {
            self.errors.push(format!(
                "Invalid baseline compression level: {} not in {:#?}",
                self.baseline_compression_level.unwrap(),
                ZSTD_COMPRESSION_LEVELS
            ));
        }
    }

    fn flush_to_disk(&mut self, path: PathBuf) {
        if !self.errors.is_empty() {
            return;
        }

        trace!(
            "Flushing cache config file to the disk, path: {}",
            path.display()
        );

        let parent_dir = match path.parent() {
            Some(p) => p,
            None => {
                self.errors
                    .push(format!("Invalid cache config path: {}", path.display()));
                return;
            }
        };

        match fs::create_dir_all(parent_dir) {
            Ok(()) => (),
            Err(err) => {
                self.errors.push(format!(
                    "Failed to create config directory, config path: {}, error: {}",
                    path.display(),
                    err
                ));
                return;
            }
        };

        let serialized = match self.exec_as_config(|config| toml::to_string_pretty(&config)) {
            Ok(data) => data,
            Err(err) => {
                self.errors.push(format!(
                    "Failed to serialize config, (unused) path: {}, msg: {}",
                    path.display(),
                    err
                ));
                return;
            }
        };

        let header = "# Automatically generated with defaults.\n\
                      # Comment out certain fields to use default values.\n\n";

        let content = format!("{}{}", header, serialized);
        match fs::write(&path, &content) {
            Ok(()) => (),
            Err(err) => {
                self.errors.push(format!(
                    "Failed to flush config to the disk, path: {}, msg: {}",
                    path.display(),
                    err
                ));
                return;
            }
        }
    }

    fn disable_if_any_error(&mut self) {
        if !self.errors.is_empty() {
            let mut conf = Self::new_cache_disabled();
            mem::swap(self, &mut conf);
            mem::swap(&mut self.errors, &mut conf.errors);
        }
    }

    fn exec_as_config<T>(&mut self, closure: impl FnOnce(&mut Config) -> T) -> T {
        let mut config = Config {
            cache: CacheConfig::new_cache_disabled(),
        };
        mem::swap(self, &mut config.cache);
        let ret = closure(&mut config);
        mem::swap(self, &mut config.cache);
        ret
    }
}
