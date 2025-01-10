fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    assert_eq!(args, [
        "hello",
        "this",
        "",
        "is an argument",
        "with 🚩 emoji"
    ]);
}
