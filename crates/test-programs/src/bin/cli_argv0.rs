fn main() {
    let mut args = std::env::args();
    assert_eq!(args.next(), args.next());
}
