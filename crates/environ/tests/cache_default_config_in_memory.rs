use wasmtime_environ::cache_init;

#[test]
fn test_cache_default_config_in_memory() {
    let errors = cache_init::<&str>(true, None, None);
    assert!(
        errors.is_empty(),
        "This test loads config from the default location, if there's one. Make sure it's correct!"
    );
}
