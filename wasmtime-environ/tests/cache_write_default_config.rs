use tempfile;
use wasmtime_environ::cache_create_new_config;

#[test]
fn test_cache_write_default_config() {
    let dir = tempfile::tempdir().expect("Can't create temporary directory");
    let config_path = dir.path().join("cache-config.toml");

    let result = cache_create_new_config(Some(&config_path));
    assert!(result.is_ok());
    assert!(config_path.exists());
    assert_eq!(config_path, result.unwrap());
}
