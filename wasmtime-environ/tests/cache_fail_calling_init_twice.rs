use tempfile;
use wasmtime_environ::cache_conf;

#[test]
#[should_panic]
fn test_fail_calling_init_twice() {
    let dir = tempfile::tempdir().expect("Can't create temporary directory");
    cache_conf::init(true, Some(dir.path()));
    cache_conf::init(true, Some(dir.path()));
}
