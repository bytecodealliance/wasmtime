use std::fs;
use tempfile;
use wasmtime_environ::cache_init;

#[test]
#[should_panic]
fn test_cache_fail_calling_init_twice() {
    let dir = tempfile::tempdir().expect("Can't create temporary directory");
    let cache_dir = dir.path().join("cache-dir");
    let baseline_compression_level = 5;

    let config_path = dir.path().join("cache-config.toml");
    let config_content = format!(
        "[cache]\n\
         enabled = true\n\
         directory = {}\n\
         baseline-compression-level = {}\n",
        toml::to_string_pretty(&format!("{}", cache_dir.display())).unwrap(),
        baseline_compression_level,
    );
    fs::write(&config_path, config_content).expect("Failed to write test config file");

    let errors = cache_init(true, Some(&config_path), None);
    assert!(errors.is_empty());
    let _errors = cache_init(true, Some(&config_path), None);
}
