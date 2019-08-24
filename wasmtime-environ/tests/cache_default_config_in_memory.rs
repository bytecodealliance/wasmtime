use wasmtime_environ::cache_config;

#[test]
fn test_cache_default_config_in_memory() {
    let errors = cache_config::init::<&str>(true, None, false);
    assert!(errors.is_empty());
}
