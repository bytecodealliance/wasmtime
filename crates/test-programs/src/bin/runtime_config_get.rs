use test_programs::wasi::config::runtime;

fn main() {
    let v = runtime::get("hello").unwrap().unwrap();
    assert_eq!(v, "world");
    let config = runtime::get_all().unwrap();
    assert_eq!(config, [("hello".to_string(), "world".to_string())])
}
