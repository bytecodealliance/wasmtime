fn test_big_random_buf() {
    let buf = wasip2::random::random::get_random_bytes(1024);
    assert_eq!(buf.len(), 1024);
    // Chances are pretty good that at least *one* byte will be non-zero in
    // any meaningful random function producing 1024 u8 values.
    assert!(
        buf.iter().any(|x| *x != 0),
        "get_random_bytes returned all zeros"
    );
}

fn main() {
    // Run the tests.
    test_big_random_buf()
}
