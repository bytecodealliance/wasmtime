fn main() {
    let mut byte = [0_u8];
    getrandom::getrandom(&mut byte).unwrap();

    assert_eq!(42, byte[0]);
}
