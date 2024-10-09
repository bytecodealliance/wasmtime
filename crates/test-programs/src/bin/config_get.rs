use test_programs::wasi::config::store;

fn main() {
    let v = store::get("hello").unwrap().unwrap();
    assert_eq!(v, "world");
    let config = store::get_all().unwrap();
    assert_eq!(config, [("hello".to_string(), "world".to_string())])
}
