use wasmtime_environ::cache_config;

#[test]
fn test_cache_default_config_in_memory() {
    let errors = cache_config::init::<&str>(true, None, false, None);
    assert!(
        errors.is_empty(),
        "This test loads config from the default location, if there's one. Make sure it's correct!"
    );
}
