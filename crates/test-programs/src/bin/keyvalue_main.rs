use test_programs::keyvalue::wasi::keyvalue::{atomics, batch, store};

fn main() {
    let identifier = std::env::var("IDENTIFIER").unwrap_or("".to_string());
    let bucket = store::open(&identifier).unwrap();

    if identifier != "" {
        // for In-Memory provider, we have preset this data
        assert_eq!(atomics::increment(&bucket, "atomics_key", 5).unwrap(), 5);
    }
    assert_eq!(atomics::increment(&bucket, "atomics_key", 1).unwrap(), 6);

    let resp = bucket.list_keys(None).unwrap();
    assert_eq!(resp.keys, vec!["atomics_key".to_string()]);

    bucket.set("hello", "world".as_bytes()).unwrap();

    let v = bucket.get("hello").unwrap();
    assert_eq!(String::from_utf8(v.unwrap()).unwrap(), "world");

    assert_eq!(bucket.exists("hello").unwrap(), true);
    bucket.delete("hello").unwrap();
    assert_eq!(bucket.exists("hello").unwrap(), false);

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
