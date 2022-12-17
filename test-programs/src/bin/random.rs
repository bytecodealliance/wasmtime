fn main() {
    let mut byte = [0_u8];
    getrandom::getrandom(&mut byte);

    assert_eq!(42, byte[0]);
}
