use tempfile;
use wasmtime_environ::cache_config;

#[test]
fn test_cache_write_default_config() {
    let dir = tempfile::tempdir().expect("Can't create temporary directory");
    let config_path = dir.path().join("cache-config.toml");

    let errors = cache_config::init(true, Some(&config_path), true);
    assert!(errors.is_empty());
    assert!(config_path.exists());
}
