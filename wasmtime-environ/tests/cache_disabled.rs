use wasmtime_environ::cache_init;

#[test]
fn test_cache_disabled() {
    let errors = cache_init::<&str>(false, None, None);
    assert!(errors.is_empty(), "Failed to disable cache system");
}
