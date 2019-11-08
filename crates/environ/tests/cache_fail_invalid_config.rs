use std::fs;
use tempfile;
use wasmtime_environ::cache_init;

#[test]
fn test_cache_fail_invalid_config() {
    let dir = tempfile::tempdir().expect("Can't create temporary directory");
    let baseline_compression_level = -4;

    let config_path = dir.path().join("cache-config.toml");
    let config_content = format!(
        "[cache]\n\
         enabled = true\n\
         directory = {}\n\
         baseline-compression-level = {}\n",
        toml::to_string_pretty(&format!("{}", config_path.display())).unwrap(), // directory is a file -- incorrect!
        baseline_compression_level,
    );
    fs::write(&config_path, config_content).expect("Failed to write test config file");

    let errors = cache_init(true, Some(&config_path), None);
    assert!(!errors.is_empty());
}
