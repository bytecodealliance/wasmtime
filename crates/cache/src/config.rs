//! Module for configuring the cache system.

use super::Worker;
use anyhow::{anyhow, bail, Context, Result};
use directories_next::ProjectDirs;
use log::{trace, warn};
use serde::{
    de::{self, Deserializer},
    Deserialize,
};
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;
use std::time::Duration;

// wrapped, so we have named section in config,
// also, for possible future compatibility
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct Config {
    cache: CacheConfig,
}

/// Global configuration for how the cache is managed
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct CacheConfig {
    enabled: bool,
    directory: Option<PathBuf>,
    #[serde(
        default,
        rename = "worker-event-queue-size",
        deserialize_with = "deserialize_si_prefix"
    )]
    worker_event_queue_size: Option<u64>,
    #[serde(rename = "baseline-compression-level")]
    baseline_compression_level: Option<i32>,
    #[serde(rename = "optimized-compression-level")]
    optimized_compression_level: Option<i32>,
    #[serde(
        default,
        rename = "optimized-compression-usage-counter-threshold",
        deserialize_with = "deserialize_si_prefix"
    )]
    optimized_compression_usage_counter_threshold: Option<u64>,
    #[serde(
        default,
        rename = "cleanup-interval",
        deserialize_with = "deserialize_duration"
    )]
    cleanup_interval: Option<Duration>,
    #[serde(
        default,
        rename = "optimizing-compression-task-timeout",
        deserialize_with = "deserialize_duration"
    )]
    optimizing_compression_task_timeout: Option<Duration>,
    #[serde(
        default,
        rename = "allowed-clock-drift-for-files-from-future",
        deserialize_with = "deserialize_duration"
    )]
    allowed_clock_drift_for_files_from_future: Option<Duration>,
    #[serde(
        default,
        rename = "file-count-soft-limit",
        deserialize_with = "deserialize_si_prefix"
    )]
    file_count_soft_limit: Option<u64>,
    #[serde(
        default,
        rename = "files-total-size-soft-limit",
        deserialize_with = "deserialize_disk_space"
    )]
    files_total_size_soft_limit: Option<u64>,
    #[serde(
        default,
        rename = "file-count-limit-percent-if-deleting",
        deserialize_with = "deserialize_percent"
    )]
    file_count_limit_percent_if_deleting: Option<u8>,
    #[serde(
        default,
        rename = "files-total-size-limit-percent-if-deleting",
        deserialize_with = "deserialize_percent"
    )]
    files_total_size_limit_percent_if_deleting: Option<u8>,

    #[serde(skip)]
    worker: Option<Worker>,
    #[serde(skip)]
    state: Arc<CacheState>,
}

#[derive(Default, Debug)]
struct CacheState {
    hits: AtomicUsize,
    misses: AtomicUsize,
}

/// Creates a new configuration file at specified path, or default path if None is passed.
/// Fails if file already exists.
pub fn create_new_config<P: AsRef<Path> + Debug>(config_file: Option<P>) -> Result<PathBuf> {
    trace!("Creating new config file, path: {:?}", config_file);

    let config_file = match config_file {
        Some(path) => path.as_ref().to_path_buf(),
        None => default_config_path()?,
    };

    if config_file.exists() {
        bail!(
            "Configuration file '{}' already exists.",
            config_file.display()
        );
    }

    let parent_dir = config_file
        .parent()
        .ok_or_else(|| anyhow!("Invalid cache config path: {}", config_file.display()))?;

    fs::create_dir_all(parent_dir).with_context(|| {
        format!(
            "Failed to create config directory, config path: {}",
            config_file.display(),
        )
    })?;

    let content = "\
# Comment out certain settings to use default values.
# For more settings, please refer to the documentation:
# https://bytecodealliance.github.io/wasmtime/cli-cache.html

[cache]
enabled = true
";

    fs::write(&config_file, &content).with_context(|| {
        format!(
            "Failed to flush config to the disk, path: {}",
            config_file.display(),
        )
    })?;

    Ok(config_file.to_path_buf())
}

// permitted levels from: https://docs.rs/zstd/0.4.28+zstd.1.4.3/zstd/stream/write/struct.Encoder.html
const ZSTD_COMPRESSION_LEVELS: std::ops::RangeInclusive<i32> = 0..=21;

// Default settings, you're welcome to tune them!
// TODO: what do we want to warn users about?

// At the moment of writing, the modules couldn't depend on anothers,
// so we have at most one module per wasmtime instance
// if changed, update cli-cache.md
const DEFAULT_WORKER_EVENT_QUEUE_SIZE: u64 = 0x10;
const WORKER_EVENT_QUEUE_SIZE_WARNING_TRESHOLD: u64 = 3;
// should be quick and provide good enough compression
// if changed, update cli-cache.md
const DEFAULT_BASELINE_COMPRESSION_LEVEL: i32 = zstd::DEFAULT_COMPRESSION_LEVEL;
// should provide significantly better compression than baseline
// if changed, update cli-cache.md
const DEFAULT_OPTIMIZED_COMPRESSION_LEVEL: i32 = 20;
// shouldn't be to low to avoid recompressing too many files
// if changed, update cli-cache.md
const DEFAULT_OPTIMIZED_COMPRESSION_USAGE_COUNTER_THRESHOLD: u64 = 0x100;
// if changed, update cli-cache.md
const DEFAULT_CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);
// if changed, update cli-cache.md
const DEFAULT_OPTIMIZING_COMPRESSION_TASK_TIMEOUT: Duration = Duration::from_secs(30 * 60);
// the default assumes problems with timezone configuration on network share + some clock drift
// please notice 24 timezones = max 23h difference between some of them
// if changed, update cli-cache.md
const DEFAULT_ALLOWED_CLOCK_DRIFT_FOR_FILES_FROM_FUTURE: Duration =
    Duration::from_secs(60 * 60 * 24);
// if changed, update cli-cache.md
const DEFAULT_FILE_COUNT_SOFT_LIMIT: u64 = 0x10_000;
// if changed, update cli-cache.md
const DEFAULT_FILES_TOTAL_SIZE_SOFT_LIMIT: u64 = 1024 * 1024 * 512;
// if changed, update cli-cache.md
const DEFAULT_FILE_COUNT_LIMIT_PERCENT_IF_DELETING: u8 = 70;
// if changed, update cli-cache.md
const DEFAULT_FILES_TOTAL_SIZE_LIMIT_PERCENT_IF_DELETING: u8 = 70;

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "BytecodeAlliance", "wasmtime")
}

fn default_config_path() -> Result<PathBuf> {
    match project_dirs() {
        Some(dirs) => Ok(dirs.config_dir().join("config.toml")),
        None => bail!("config file not specified and failed to get the default"),
    }
}

// Deserializers of our custom formats
// can be replaced with const generics later
macro_rules! generate_deserializer {
    ($name:ident($numname:ident: $numty:ty, $unitname:ident: &str) -> $retty:ty {$body:expr}) => {
        fn $name<'de, D>(deserializer: D) -> Result<$retty, D::Error>
        where
            D: Deserializer<'de>,
        {
            let text = Option::<String>::deserialize(deserializer)?;
            let text = match text {
                None => return Ok(None),
                Some(text) => text,
            };
            let text = text.trim();
            let split_point = text.find(|c: char| !c.is_numeric());
            let (num, unit) = split_point.map_or_else(|| (text, ""), |p| text.split_at(p));
            let deserialized = (|| {
                let $numname = num.parse::<$numty>().ok()?;
                let $unitname = unit.trim();
                $body
            })();
            if deserialized.is_some() {
                Ok(deserialized)
            } else {
                Err(de::Error::custom(
                    "Invalid value, please refer to the documentation",
                ))
            }
        }
    };
}

generate_deserializer!(deserialize_duration(num: u64, unit: &str) -> Option<Duration> {
    match unit {
        "s" => Some(Duration::from_secs(num)),
        "m" => Some(Duration::from_secs(num * 60)),
        "h" => Some(Duration::from_secs(num * 60 * 60)),
        "d" => Some(Duration::from_secs(num * 60 * 60 * 24)),
        _ => None,
    }
});

generate_deserializer!(deserialize_si_prefix(num: u64, unit: &str) -> Option<u64> {
    match unit {
        "" => Some(num),
        "K" => num.checked_mul(1_000),
        "M" => num.checked_mul(1_000_000),
        "G" => num.checked_mul(1_000_000_000),
        "T" => num.checked_mul(1_000_000_000_000),
        "P" => num.checked_mul(1_000_000_000_000_000),
        _ => None,
    }
});

generate_deserializer!(deserialize_disk_space(num: u64, unit: &str) -> Option<u64> {
    match unit {
        "" => Some(num),
        "K" => num.checked_mul(1_000),
        "Ki" => num.checked_mul(1u64 << 10),
        "M" => num.checked_mul(1_000_000),
        "Mi" => num.checked_mul(1u64 << 20),
        "G" => num.checked_mul(1_000_000_000),
        "Gi" => num.checked_mul(1u64 << 30),
        "T" => num.checked_mul(1_000_000_000_000),
        "Ti" => num.checked_mul(1u64 << 40),
        "P" => num.checked_mul(1_000_000_000_000_000),
        "Pi" => num.checked_mul(1u64 << 50),
        _ => None,
    }
});

generate_deserializer!(deserialize_percent(num: u8, unit: &str) -> Option<u8> {
    match unit {
        "%" => Some(num),
        _ => None,
    }
});

static CACHE_IMPROPER_CONFIG_ERROR_MSG: &str =
    "Cache system should be enabled and all settings must be validated or defaulted";

macro_rules! generate_setting_getter {
    ($setting:ident: $setting_type:ty) => {
        /// Returns `$setting`.
        ///
        /// Panics if the cache is disabled.
        pub fn $setting(&self) -> $setting_type {
            self.$setting.expect(CACHE_IMPROPER_CONFIG_ERROR_MSG)
        }
    };
}

impl CacheConfig {
    generate_setting_getter!(worker_event_queue_size: u64);
    generate_setting_getter!(baseline_compression_level: i32);
    generate_setting_getter!(optimized_compression_level: i32);
    generate_setting_getter!(optimized_compression_usage_counter_threshold: u64);
    generate_setting_getter!(cleanup_interval: Duration);
    generate_setting_getter!(optimizing_compression_task_timeout: Duration);
    generate_setting_getter!(allowed_clock_drift_for_files_from_future: Duration);
    generate_setting_getter!(file_count_soft_limit: u64);
    generate_setting_getter!(files_total_size_soft_limit: u64);
    generate_setting_getter!(file_count_limit_percent_if_deleting: u8);
    generate_setting_getter!(files_total_size_limit_percent_if_deleting: u8);

    /// Returns true if and only if the cache is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns path to the cache directory.
    ///
    /// Panics if the cache is disabled.
    pub fn directory(&self) -> &PathBuf {
        self.directory
            .as_ref()
            .expect(CACHE_IMPROPER_CONFIG_ERROR_MSG)
    }

    /// Creates a new set of configuration which represents a disabled cache
    pub fn new_cache_disabled() -> Self {
        Self {
            enabled: false,
            directory: None,
            worker_event_queue_size: None,
            baseline_compression_level: None,
            optimized_compression_level: None,
            optimized_compression_usage_counter_threshold: None,
            cleanup_interval: None,
            optimizing_compression_task_timeout: None,
            allowed_clock_drift_for_files_from_future: None,
            file_count_soft_limit: None,
            files_total_size_soft_limit: None,
            file_count_limit_percent_if_deleting: None,
            files_total_size_limit_percent_if_deleting: None,
            worker: None,
            state: Arc::new(CacheState::default()),
        }
    }

    fn new_cache_enabled_template() -> Self {
        let mut conf = Self::new_cache_disabled();
        conf.enabled = true;
        conf
    }

    /// Parses cache configuration from the file specified
    pub fn from_file(config_file: Option<&Path>) -> Result<Self> {
        let mut config = Self::load_and_parse_file(config_file)?;

        // validate values and fill in defaults
        config.validate_directory_or_default()?;
        config.validate_worker_event_queue_size_or_default();
        config.validate_baseline_compression_level_or_default()?;
        config.validate_optimized_compression_level_or_default()?;
        config.validate_optimized_compression_usage_counter_threshold_or_default();
        config.validate_cleanup_interval_or_default();
        config.validate_optimizing_compression_task_timeout_or_default();
        config.validate_allowed_clock_drift_for_files_from_future_or_default();
        config.validate_file_count_soft_limit_or_default();
        config.validate_files_total_size_soft_limit_or_default();
        config.validate_file_count_limit_percent_if_deleting_or_default()?;
        config.validate_files_total_size_limit_percent_if_deleting_or_default()?;
        config.spawn_worker();

        Ok(config)
    }

    fn spawn_worker(&mut self) {
        if self.enabled {
            self.worker = Some(Worker::start_new(self, None));
        }
    }

    pub(super) fn worker(&self) -> &Worker {
        assert!(self.enabled);
        self.worker.as_ref().unwrap()
    }

    /// Returns the number of cache hits seen so far
    pub fn cache_hits(&self) -> usize {
        self.state.hits.load(SeqCst)
    }

    /// Returns the number of cache misses seen so far
    pub fn cache_misses(&self) -> usize {
        self.state.misses.load(SeqCst)
    }

    pub(crate) fn on_cache_get_async(&self, path: impl AsRef<Path>) {
        self.state.hits.fetch_add(1, SeqCst);
        self.worker().on_cache_get_async(path)
    }

    pub(crate) fn on_cache_update_async(&self, path: impl AsRef<Path>) {
        self.state.misses.fetch_add(1, SeqCst);
        self.worker().on_cache_update_async(path)
    }

    fn load_and_parse_file(config_file: Option<&Path>) -> Result<Self> {
        // get config file path
        let (config_file, user_custom_file) = match config_file {
            Some(path) => (path.to_path_buf(), true),
            None => (default_config_path()?, false),
        };

        // read config, or use default one
        let entity_exists = config_file.exists();
        match (entity_exists, user_custom_file) {
            (false, false) => Ok(Self::new_cache_enabled_template()),
            _ => {
                let bytes = fs::read(&config_file).context(format!(
                    "failed to read config file: {}",
                    config_file.display()
                ))?;
                let config = toml::from_slice::<Config>(&bytes[..]).context(format!(
                    "failed to parse config file: {}",
                    config_file.display()
                ))?;
                Ok(config.cache)
            }
        }
    }

    fn validate_directory_or_default(&mut self) -> Result<()> {
        if self.directory.is_none() {
            match project_dirs() {
                Some(proj_dirs) => self.directory = Some(proj_dirs.cache_dir().to_path_buf()),
                None => {
                    bail!("Cache directory not specified and failed to get the default");
                }
            }
        }

        // On Windows, if we want long paths, we need '\\?\' prefix, but it doesn't work
        // with relative paths. One way to get absolute path (the only one?) is to use
        // fs::canonicalize, but it requires that given path exists. The extra advantage
        // of this method is fact that the method prepends '\\?\' on Windows.
        let cache_dir = self.directory.as_ref().unwrap();

        if !cache_dir.is_absolute() {
            bail!(
                "Cache directory path has to be absolute, path: {}",
                cache_dir.display(),
            );
        }

        fs::create_dir_all(cache_dir).context(format!(
            "failed to create cache directory: {}",
            cache_dir.display()
        ))?;
        let canonical = fs::canonicalize(cache_dir).context(format!(
            "failed to canonicalize cache directory: {}",
            cache_dir.display()
        ))?;
        self.directory = Some(canonical);
        Ok(())
    }

    fn validate_worker_event_queue_size_or_default(&mut self) {
        if self.worker_event_queue_size.is_none() {
            self.worker_event_queue_size = Some(DEFAULT_WORKER_EVENT_QUEUE_SIZE);
        }

        if self.worker_event_queue_size.unwrap() < WORKER_EVENT_QUEUE_SIZE_WARNING_TRESHOLD {
            warn!("Detected small worker event queue size. Some messages might be lost.");
        }
    }

    fn validate_baseline_compression_level_or_default(&mut self) -> Result<()> {
        if self.baseline_compression_level.is_none() {
            self.baseline_compression_level = Some(DEFAULT_BASELINE_COMPRESSION_LEVEL);
        }

        if !ZSTD_COMPRESSION_LEVELS.contains(&self.baseline_compression_level.unwrap()) {
            bail!(
                "Invalid baseline compression level: {} not in {:#?}",
                self.baseline_compression_level.unwrap(),
                ZSTD_COMPRESSION_LEVELS
            );
        }
        Ok(())
    }

    // assumption: baseline compression level has been verified
    fn validate_optimized_compression_level_or_default(&mut self) -> Result<()> {
        if self.optimized_compression_level.is_none() {
            self.optimized_compression_level = Some(DEFAULT_OPTIMIZED_COMPRESSION_LEVEL);
        }

        let opt_lvl = self.optimized_compression_level.unwrap();
        let base_lvl = self.baseline_compression_level.unwrap();

        if !ZSTD_COMPRESSION_LEVELS.contains(&opt_lvl) {
            bail!(
                "Invalid optimized compression level: {} not in {:#?}",
                opt_lvl,
                ZSTD_COMPRESSION_LEVELS
            );
        }

        if opt_lvl < base_lvl {
            bail!(
                "Invalid optimized compression level is lower than baseline: {} < {}",
                opt_lvl,
                base_lvl
            );
        }
        Ok(())
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

    fn validate_allowed_clock_drift_for_files_from_future_or_default(&mut self) {
        if self.allowed_clock_drift_for_files_from_future.is_none() {
            self.allowed_clock_drift_for_files_from_future =
                Some(DEFAULT_ALLOWED_CLOCK_DRIFT_FOR_FILES_FROM_FUTURE);
        }
    }

    fn validate_file_count_soft_limit_or_default(&mut self) {
        if self.file_count_soft_limit.is_none() {
            self.file_count_soft_limit = Some(DEFAULT_FILE_COUNT_SOFT_LIMIT);
        }
    }

    fn validate_files_total_size_soft_limit_or_default(&mut self) {
        if self.files_total_size_soft_limit.is_none() {
            self.files_total_size_soft_limit = Some(DEFAULT_FILES_TOTAL_SIZE_SOFT_LIMIT);
        }
    }

    fn validate_file_count_limit_percent_if_deleting_or_default(&mut self) -> Result<()> {
        if self.file_count_limit_percent_if_deleting.is_none() {
            self.file_count_limit_percent_if_deleting =
                Some(DEFAULT_FILE_COUNT_LIMIT_PERCENT_IF_DELETING);
        }

        let percent = self.file_count_limit_percent_if_deleting.unwrap();
        if percent > 100 {
            bail!(
                "Invalid files count limit percent if deleting: {} not in range 0-100%",
                percent
            );
        }
        Ok(())
    }

    fn validate_files_total_size_limit_percent_if_deleting_or_default(&mut self) -> Result<()> {
        if self.files_total_size_limit_percent_if_deleting.is_none() {
            self.files_total_size_limit_percent_if_deleting =
                Some(DEFAULT_FILES_TOTAL_SIZE_LIMIT_PERCENT_IF_DELETING);
        }

        let percent = self.files_total_size_limit_percent_if_deleting.unwrap();
        if percent > 100 {
            bail!(
                "Invalid files total size limit percent if deleting: {} not in range 0-100%",
                percent
            );
        }
        Ok(())
    }
}

#[cfg(test)]
#[macro_use]
pub mod tests;
