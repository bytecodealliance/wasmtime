use tempfile;
use wasmtime_environ::cache_init;

#[test]
fn test_cache_fail_invalid_path_to_config() {
    let dir = tempfile::tempdir().expect("Can't create temporary directory");
    let config_path = dir.path().join("cache-config.toml"); // doesn't exist
    let errors = cache_init(true, Some(&config_path), None);
    assert!(!errors.is_empty());
}
