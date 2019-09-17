use super::CacheConfig;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::{self, TempDir};

// note: config loading during validation creates cache directory to canonicalize its path,
//       that's why these function and macro always use custom cache directory
// note: tempdir removes directory when being dropped, so we need to return it to the caller,
//       so the paths are valid
pub fn test_prolog() -> (TempDir, PathBuf, PathBuf) {
    let _ = pretty_env_logger::try_init();
    let temp_dir = tempfile::tempdir().expect("Can't create temporary directory");
    let cache_dir = temp_dir.path().join("cache-dir");
    let config_path = temp_dir.path().join("cache-config.toml");
    (temp_dir, cache_dir, config_path)
}

macro_rules! load_config {
    ($config_path:ident, $content_fmt:expr, $cache_dir:ident) => {{
        let config_path = &$config_path;
        let content = format!(
            $content_fmt,
            cache_dir = toml::to_string_pretty(&format!("{}", $cache_dir.display())).unwrap()
        );
        fs::write(config_path, content).expect("Failed to write test config file");
        CacheConfig::from_file(true, Some(config_path))
    }};
}

// test without macros to test being disabled
#[test]
fn test_disabled() {
    let dir = tempfile::tempdir().expect("Can't create temporary directory");
    let config_path = dir.path().join("cache-config.toml");
    let config_content = "[cache]\n\
                          enabled = true\n";
    fs::write(&config_path, config_content).expect("Failed to write test config file");
    let conf = CacheConfig::from_file(false, Some(&config_path));
    assert!(!conf.enabled());
    assert!(conf.errors.is_empty());

    let config_content = "[cache]\n\
                          enabled = false\n";
    fs::write(&config_path, config_content).expect("Failed to write test config file");
    let conf = CacheConfig::from_file(true, Some(&config_path));
    assert!(!conf.enabled());
    assert!(conf.errors.is_empty());
}

#[test]
fn test_unrecognized_settings() {
    let (_td, cd, cp) = test_prolog();
    let conf = load_config!(
        cp,
        "unrecognized-setting = 42\n\
         [cache]\n\
         enabled = true\n\
         directory = {cache_dir}",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         unrecognized-setting = 42",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());
}

#[test]
fn test_all_settings() {
    let (_td, cd, cp) = test_prolog();
    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         worker-event-queue-size = '16'\n\
         baseline-compression-level = 3\n\
         optimized-compression-level = 20\n\
         optimized-compression-usage-counter-threshold = '256'\n\
         cleanup-interval = '1h'\n\
         optimizing-compression-task-timeout = '30m'\n\
         allowed-clock-drift-for-files-from-future = '1d'\n\
         file-count-soft-limit = '65536'\n\
         files-total-size-soft-limit = '512Mi'\n\
         file-count-limit-percent-if-deleting = '70%'\n\
         files-total-size-limit-percent-if-deleting = '70%'",
        cd
    );
    check_conf(&conf, &cd);

    let conf = load_config!(
        cp,
        // added some white spaces
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         worker-event-queue-size  = ' 16\t'\n\
         baseline-compression-level = 3\n\
         optimized-compression-level =\t 20\n\
         optimized-compression-usage-counter-threshold = '256'\n\
         cleanup-interval = ' 1h'\n\
         optimizing-compression-task-timeout = '30  m'\n\
         allowed-clock-drift-for-files-from-future = '1\td'\n\
         file-count-soft-limit = '\t \t65536\t'\n\
         files-total-size-soft-limit = '512\t\t Mi '\n\
         file-count-limit-percent-if-deleting = '70\t%'\n\
         files-total-size-limit-percent-if-deleting = ' 70 %'",
        cd
    );
    check_conf(&conf, &cd);

    fn check_conf(conf: &CacheConfig, cd: &PathBuf) {
        eprintln!("errors: {:#?}", conf.errors);
        assert!(conf.enabled());
        assert!(conf.errors.is_empty());
        assert_eq!(
            conf.directory(),
            &fs::canonicalize(cd).expect("canonicalize failed")
        );
        assert_eq!(conf.worker_event_queue_size(), 0x10);
        assert_eq!(conf.baseline_compression_level(), 3);
        assert_eq!(conf.optimized_compression_level(), 20);
        assert_eq!(conf.optimized_compression_usage_counter_threshold(), 0x100);
        assert_eq!(conf.cleanup_interval(), Duration::from_secs(60 * 60));
        assert_eq!(
            conf.optimizing_compression_task_timeout(),
            Duration::from_secs(30 * 60)
        );
        assert_eq!(
            conf.allowed_clock_drift_for_files_from_future(),
            Duration::from_secs(60 * 60 * 24)
        );
        assert_eq!(conf.file_count_soft_limit(), 0x10_000);
        assert_eq!(conf.files_total_size_soft_limit(), 512 * (1u64 << 20));
        assert_eq!(conf.file_count_limit_percent_if_deleting(), 70);
        assert_eq!(conf.files_total_size_limit_percent_if_deleting(), 70);
    }
}

#[test]
fn test_compression_level_settings() {
    let (_td, cd, cp) = test_prolog();
    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         baseline-compression-level = 1\n\
         optimized-compression-level = 21",
        cd
    );
    assert!(conf.enabled());
    assert!(conf.errors.is_empty());
    assert_eq!(conf.baseline_compression_level(), 1);
    assert_eq!(conf.optimized_compression_level(), 21);

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         baseline-compression-level = -1\n\
         optimized-compression-level = 21",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         baseline-compression-level = 15\n\
         optimized-compression-level = 10",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());
}

#[test]
fn test_si_prefix_settings() {
    let (_td, cd, cp) = test_prolog();
    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         worker-event-queue-size = '42'\n\
         optimized-compression-usage-counter-threshold = '4K'\n\
         file-count-soft-limit = '3M'",
        cd
    );
    assert!(conf.enabled());
    assert!(conf.errors.is_empty());
    assert_eq!(conf.worker_event_queue_size(), 42);
    assert_eq!(conf.optimized_compression_usage_counter_threshold(), 4_000);
    assert_eq!(conf.file_count_soft_limit(), 3_000_000);

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         worker-event-queue-size = '2G'\n\
         optimized-compression-usage-counter-threshold = '4444T'\n\
         file-count-soft-limit = '1P'",
        cd
    );
    assert!(conf.enabled());
    assert!(conf.errors.is_empty());
    assert_eq!(conf.worker_event_queue_size(), 2_000_000_000);
    assert_eq!(
        conf.optimized_compression_usage_counter_threshold(),
        4_444_000_000_000_000
    );
    assert_eq!(conf.file_count_soft_limit(), 1_000_000_000_000_000);

    // different errors
    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         worker-event-queue-size = '2g'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         file-count-soft-limit = 1",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         file-count-soft-limit = '-31337'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         file-count-soft-limit = '3.14M'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());
}

#[test]
fn test_disk_space_settings() {
    let (_td, cd, cp) = test_prolog();
    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-soft-limit = '76'",
        cd
    );
    assert!(conf.enabled());
    assert!(conf.errors.is_empty());
    assert_eq!(conf.files_total_size_soft_limit(), 76);

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-soft-limit = '42 Mi'",
        cd
    );
    assert!(conf.enabled());
    assert!(conf.errors.is_empty());
    assert_eq!(conf.files_total_size_soft_limit(), 42 * (1u64 << 20));

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-soft-limit = '2 Gi'",
        cd
    );
    assert!(conf.enabled());
    assert!(conf.errors.is_empty());
    assert_eq!(conf.files_total_size_soft_limit(), 2 * (1u64 << 30));

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-soft-limit = '31337 Ti'",
        cd
    );
    assert!(conf.enabled());
    assert!(conf.errors.is_empty());
    assert_eq!(conf.files_total_size_soft_limit(), 31337 * (1u64 << 40));

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-soft-limit = '7 Pi'",
        cd
    );
    assert!(conf.enabled());
    assert!(conf.errors.is_empty());
    assert_eq!(conf.files_total_size_soft_limit(), 7 * (1u64 << 50));

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-soft-limit = '7M'",
        cd
    );
    assert!(conf.enabled());
    assert!(conf.errors.is_empty());
    assert_eq!(conf.files_total_size_soft_limit(), 7_000_000);

    // different errors
    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-soft-limit = '7 mi'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-soft-limit = 1",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-soft-limit = '-31337'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-soft-limit = '3.14Ki'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());
}

#[test]
fn test_duration_settings() {
    let (_td, cd, cp) = test_prolog();
    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         cleanup-interval = '100s'\n\
         optimizing-compression-task-timeout = '3m'\n\
         allowed-clock-drift-for-files-from-future = '4h'",
        cd
    );
    assert!(conf.enabled());
    assert!(conf.errors.is_empty());
    assert_eq!(conf.cleanup_interval(), Duration::from_secs(100));
    assert_eq!(
        conf.optimizing_compression_task_timeout(),
        Duration::from_secs(3 * 60)
    );
    assert_eq!(
        conf.allowed_clock_drift_for_files_from_future(),
        Duration::from_secs(4 * 60 * 60)
    );

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         cleanup-interval = '2d'\n\
         optimizing-compression-task-timeout = '333 m'",
        cd
    );
    assert!(conf.enabled());
    assert!(conf.errors.is_empty());
    assert_eq!(
        conf.cleanup_interval(),
        Duration::from_secs(2 * 24 * 60 * 60)
    );
    assert_eq!(
        conf.optimizing_compression_task_timeout(),
        Duration::from_secs(333 * 60)
    );

    // different errors
    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         optimizing-compression-task-timeout = '333'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         optimizing-compression-task-timeout = 333",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         optimizing-compression-task-timeout = '10 M'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         optimizing-compression-task-timeout = '10 min'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         optimizing-compression-task-timeout = '-10s'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         optimizing-compression-task-timeout = '1.5m'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());
}

#[test]
fn test_percent_settings() {
    let (_td, cd, cp) = test_prolog();
    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         file-count-limit-percent-if-deleting = '62%'\n\
         files-total-size-limit-percent-if-deleting = '23 %'",
        cd
    );
    assert!(conf.enabled());
    assert!(conf.errors.is_empty());
    assert_eq!(conf.file_count_limit_percent_if_deleting(), 62);
    assert_eq!(conf.files_total_size_limit_percent_if_deleting(), 23);

    // different errors
    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-limit-percent-if-deleting = '23'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-limit-percent-if-deleting = '22.5%'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-limit-percent-if-deleting = '0.5'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-limit-percent-if-deleting = '-1%'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());

    let conf = load_config!(
        cp,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         files-total-size-limit-percent-if-deleting = '101%'",
        cd
    );
    assert!(!conf.enabled());
    assert!(!conf.errors.is_empty());
}
