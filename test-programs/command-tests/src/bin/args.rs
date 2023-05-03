fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(
        args,
        ["hello", "this", "", "is an argument", "with 🚩 emoji"]
    );
}
