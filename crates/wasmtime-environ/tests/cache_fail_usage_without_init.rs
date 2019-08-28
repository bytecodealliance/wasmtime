use wasmtime_environ::cache_conf;

#[test]
#[should_panic]
fn test_fail_usage_without_init() {
    let _ = cache_conf::cache_directory();
}
