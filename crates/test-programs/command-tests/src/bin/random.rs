fn main() {
    let mut bytes = [0_u8; 256];
    getrandom::getrandom(&mut bytes).unwrap();

    assert!(bytes.iter().any(|x| *x != 0));
}
