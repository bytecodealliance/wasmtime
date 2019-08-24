// These tests doesn't call init(), so we can test a multiple certain things here

use wasmtime_environ::cache_config;

#[test]
#[should_panic]
fn test_cache_fail_usage_without_init_directory() {
    let _ = cache_config::directory();
}

#[test]
#[should_panic]
fn test_cache_fail_usage_without_init_baseline_compression_level() {
    let _ = cache_config::baseline_compression_level();
}
