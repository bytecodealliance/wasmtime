//! Module for configuring the cache system.

use super::worker;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use log::{debug, error, trace, warn};
use serde::{de::Deserializer, ser::Serializer, Deserialize, Serialize};
use spin::Once;
use std::fmt::Debug;
use std::fs;
use std::mem;
use std::path::{Path, PathBuf};
use std::string::{String, ToString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::vec::Vec;

// wrapped, so we have named section in config,
// also, for possible future compatibility
#[derive(Serialize, Deserialize, Debug)]
struct Config {
    cache: CacheConfig,
}

// todo: markdown documention of these options
// todo: don't flush default values (create config from simple template + url to docs)
// todo: more user-friendly cache config creation
#[derive(Serialize, Deserialize, Debug)]
struct CacheConfig {
    #[serde(skip)]
    pub errors: Vec<String>,

    pub enabled: bool,
    pub directory: Option<PathBuf>,
    #[serde(rename = "worker-event-queue-size")]
    pub worker_event_queue_size: Option<usize>,
    #[serde(rename = "baseline-compression-level")]
    pub baseline_compression_level: Option<i32>,
    #[serde(rename = "optimized-compression-level")]
    pub optimized_compression_level: Option<i32>,
    #[serde(rename = "optimized-compression-usage-counter-threshold")]
    pub optimized_compression_usage_counter_threshold: Option<u64>,
    #[serde(
        default,
        rename = "cleanup-interval-in-seconds",
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )] // todo unit?
    pub cleanup_interval: Option<Duration>,
    #[serde(
        default,
        rename = "optimizing-compression-task-timeout-in-seconds",
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )] // todo unit?
    pub optimizing_compression_task_timeout: Option<Duration>,
    #[serde(rename = "files-count-soft-limit")]
    pub files_count_soft_limit: Option<u64>,
    #[serde(rename = "files-total-size-soft-limit")]
    pub files_total_size_soft_limit: Option<u64>, // todo unit?
    #[serde(rename = "files-count-limit-percent-if-deleting")]
    pub files_count_limit_percent_if_deleting: Option<u8>, // todo format: integer + %
    #[serde(rename = "files-total-size-limit-percent-if-deleting")]
    pub files_total_size_limit_percent_if_deleting: Option<u8>,
}

// toml-rs fails to serialize Duration ("values must be emitted before tables")
// so we're providing custom functions for it
fn serialize_duration<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    duration.map(|d| d.as_secs()).serialize(serializer)
}

fn deserialize_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<u64>::deserialize(deserializer)?.map(Duration::from_secs))
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
        .expect("All cache system settings must be validated or defaulted")
}

macro_rules! generate_setting_getter {
    ($setting:ident: $setting_type:ty) => {
        /// Returns `$setting`.
        ///
        /// Panics if the cache is disabled.
        pub fn $setting() -> $setting_type {
            CONFIG
                .r#try()
                .expect("Cache system must be initialized")
                .$setting
                .expect("All cache system settings must be validated or defaulted")
        }
    };
}

generate_setting_getter!(worker_event_queue_size: usize);
generate_setting_getter!(baseline_compression_level: i32);
generate_setting_getter!(optimized_compression_level: i32);
generate_setting_getter!(optimized_compression_usage_counter_threshold: u64);
generate_setting_getter!(cleanup_interval: Duration);
generate_setting_getter!(optimizing_compression_task_timeout: Duration);
generate_setting_getter!(files_count_soft_limit: u64);
generate_setting_getter!(files_total_size_soft_limit: u64);
generate_setting_getter!(files_count_limit_percent_if_deleting: u8);
generate_setting_getter!(files_total_size_limit_percent_if_deleting: u8);

/// Initializes the cache system. Should be called exactly once,
/// and before using the cache system. Otherwise it can panic.
/// Returns list of errors. If empty, initialization succeeded.
pub fn init<P: AsRef<Path> + Debug>(
    enabled: bool,
    config_file: Option<P>,
    create_new_config: bool,
    init_file_per_thread_logger: Option<&'static str>,
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
        worker::init(init_file_per_thread_logger);
        debug!("Cache init(\"{}\"): {:#?}", conf_file_str, conf)
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
// TODO: values to be tuned
// TODO: what do we want to warn users about?
const DEFAULT_WORKER_EVENT_QUEUE_SIZE: usize = 0x10;
const WORKER_EVENT_QUEUE_SIZE_WARNING_TRESHOLD: usize = 3;
const DEFAULT_BASELINE_COMPRESSION_LEVEL: i32 = zstd::DEFAULT_COMPRESSION_LEVEL;
const DEFAULT_OPTIMIZED_COMPRESSION_LEVEL: i32 = 20;
const DEFAULT_OPTIMIZED_COMPRESSION_USAGE_COUNTER_THRESHOLD: u64 = 0x100;
const DEFAULT_CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);
const DEFAULT_OPTIMIZING_COMPRESSION_TASK_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const DEFAULT_FILES_COUNT_SOFT_LIMIT: u64 = 0x10_000;
const DEFAULT_FILES_TOTAL_SIZE_SOFT_LIMIT: u64 = 1024 * 1024 * 512;
const DEFAULT_FILES_COUNT_LIMIT_PERCENT_IF_DELETING: u8 = 70;
const DEFAULT_FILES_TOTAL_SIZE_LIMIT_PERCENT_IF_DELETING: u8 = 70;

impl CacheConfig {
    pub fn new_cache_disabled() -> Self {
        Self {
            errors: Vec::new(),
            enabled: false,
            directory: None,
            worker_event_queue_size: None,
            baseline_compression_level: None,
            optimized_compression_level: None,
            optimized_compression_usage_counter_threshold: None,
            cleanup_interval: None,
            optimizing_compression_task_timeout: None,
            files_count_soft_limit: None,
            files_total_size_soft_limit: None,
            files_count_limit_percent_if_deleting: None,
            files_total_size_limit_percent_if_deleting: None,
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
        config.validate_directory_or_default();
        config.validate_worker_event_queue_size_or_default();
        config.validate_baseline_compression_level_or_default();
        config.validate_optimized_compression_level_or_default();
        config.validate_optimized_compression_usage_counter_threshold_or_default();
        config.validate_cleanup_interval_or_default();
        config.validate_optimizing_compression_task_timeout_or_default();
        config.validate_files_count_soft_limit_or_default();
        config.validate_files_total_size_soft_limit_or_default();
        config.validate_files_count_limit_percent_if_deleting_or_default();
        config.validate_files_total_size_limit_percent_if_deleting_or_default();

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

    fn validate_directory_or_default(&mut self) {
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

    fn validate_worker_event_queue_size_or_default(&mut self) {
        if self.worker_event_queue_size.is_none() {
            self.worker_event_queue_size = Some(DEFAULT_WORKER_EVENT_QUEUE_SIZE);
        }

        if self.worker_event_queue_size.unwrap() < WORKER_EVENT_QUEUE_SIZE_WARNING_TRESHOLD {
            warn!("Detected small worker event queue size. Some messages might be lost.");
        }
    }

    fn validate_baseline_compression_level_or_default(&mut self) {
        if self.baseline_compression_level.is_none() {
            self.baseline_compression_level = Some(DEFAULT_BASELINE_COMPRESSION_LEVEL);
        }

        if !ZSTD_COMPRESSION_LEVELS.contains(&self.baseline_compression_level.unwrap()) {
            self.errors.push(format!(
                "Invalid baseline compression level: {} not in {:#?}",
                self.baseline_compression_level.unwrap(),
                ZSTD_COMPRESSION_LEVELS
            ));
        }
    }

    // assumption: baseline compression level has been verified
    fn validate_optimized_compression_level_or_default(&mut self) {
        if self.optimized_compression_level.is_none() {
            self.optimized_compression_level = Some(DEFAULT_OPTIMIZED_COMPRESSION_LEVEL);
        }

        let opt_lvl = self.optimized_compression_level.unwrap();
        let base_lvl = self.baseline_compression_level.unwrap();

        if !ZSTD_COMPRESSION_LEVELS.contains(&opt_lvl) {
            self.errors.push(format!(
                "Invalid optimized compression level: {} not in {:#?}",
                opt_lvl, ZSTD_COMPRESSION_LEVELS
            ));
        }

        if opt_lvl < base_lvl {
            self.errors.push(format!(
                "Invalid optimized compression level is lower than baseline: {} < {}",
                opt_lvl, base_lvl
            ));
        }
    }

    fn validate_optimized_compression_usage_counter_threshold_or_default(&mut self) {
        if self.optimized_compression_usage_counter_threshold.is_none() {
            self.optimized_compression_usage_counter_threshold =
                Some(DEFAULT_OPTIMIZED_COMPRESSION_USAGE_COUNTER_THRESHOLD);
        }
    }

    fn validate_cleanup_interval_or_default(&mut self) {
        if self.cleanup_interval.is_none() {
            self.cleanup_interval = Some(DEFAULT_CLEANUP_INTERVAL);
        }
    }

    fn validate_optimizing_compression_task_timeout_or_default(&mut self) {
        if self.optimizing_compression_task_timeout.is_none() {
            self.optimizing_compression_task_timeout =
                Some(DEFAULT_OPTIMIZING_COMPRESSION_TASK_TIMEOUT);
        }
    }

    fn validate_files_count_soft_limit_or_default(&mut self) {
        if self.files_count_soft_limit.is_none() {
            self.files_count_soft_limit = Some(DEFAULT_FILES_COUNT_SOFT_LIMIT);
        }
    }

    fn validate_files_total_size_soft_limit_or_default(&mut self) {
        if self.files_total_size_soft_limit.is_none() {
            self.files_total_size_soft_limit = Some(DEFAULT_FILES_TOTAL_SIZE_SOFT_LIMIT);
        }
    }

    fn validate_files_count_limit_percent_if_deleting_or_default(&mut self) {
        if self.files_count_limit_percent_if_deleting.is_none() {
            self.files_count_limit_percent_if_deleting =
                Some(DEFAULT_FILES_COUNT_LIMIT_PERCENT_IF_DELETING);
        }

        let percent = self.files_count_limit_percent_if_deleting.unwrap();
        if percent > 100 {
            self.errors.push(format!(
                "Invalid files count limit percent if deleting: {} not in range 0-100%",
                percent
            ));
        }
    }

    fn validate_files_total_size_limit_percent_if_deleting_or_default(&mut self) {
        if self.files_total_size_limit_percent_if_deleting.is_none() {
            self.files_total_size_limit_percent_if_deleting =
                Some(DEFAULT_FILES_TOTAL_SIZE_LIMIT_PERCENT_IF_DELETING);
        }

        let percent = self.files_total_size_limit_percent_if_deleting.unwrap();
        if percent > 100 {
            self.errors.push(format!(
                "Invalid files total size limit percent if deleting: {} not in range 0-100%",
                percent
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
