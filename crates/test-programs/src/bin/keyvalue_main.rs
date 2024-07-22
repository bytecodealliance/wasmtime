use test_programs::keyvalue::wasi::keyvalue::{atomics, batch, store};

fn main() {
    let bucket = store::open(std::env::var_os("IDENTIFIER").unwrap().to_str().unwrap()).unwrap();
    bucket.set("hello", "world".as_bytes()).unwrap();

    let v = bucket.get("hello").unwrap();
    assert_eq!(String::from_utf8(v.unwrap()).unwrap(), "world");

    bucket.delete("hello").unwrap();
    let exists = bucket.exists("hello").unwrap();
    assert_eq!(exists, false);

    bucket.set("aa", "bb".as_bytes()).unwrap();
    let resp = bucket.list_keys(None).unwrap();
    assert_eq!(resp.keys, vec!["aa".to_string()]);

    assert_eq!(atomics::increment(&bucket, "atomics_key", 5).unwrap(), 5);
    assert_eq!(atomics::increment(&bucket, "atomics_key", 1).unwrap(), 6);

    batch::set_many(
        &bucket,
        &[
            ("a1".to_string(), "v1".as_bytes().to_vec()),
            ("b1".to_string(), "v1".as_bytes().to_vec()),
            ("c1".to_string(), "v1".as_bytes().to_vec()),
        ],
    )
    .unwrap();
    batch::delete_many(&bucket, &["a1".to_string(), "c1".to_string()]).unwrap();
    let values = batch::get_many(
        &bucket,
        &["a1".to_string(), "b1".to_string(), "c1".to_string()],
    )
    .unwrap();
    assert_eq!(
        values,
        vec![
            None,
            Some(("b1".to_string(), "v1".as_bytes().to_vec())),
            None
        ]
    );
}
