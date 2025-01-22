use test_programs::wasi::random0_3_0 as random;

fn main() {
    let mut bytes = [0_u8; 256];
    getrandom::getrandom(&mut bytes).unwrap();

    assert!(bytes.iter().any(|x| *x != 0));

    // Acquired random bytes should be of the expected length.
    let array = random::random::get_random_bytes(100);
    assert_eq!(array.len(), 100);

    // It shouldn't take 100+ tries to get a nonzero random integer.
    for i in 0.. {
        if random::random::get_random_u64() == 0 {
            continue;
        }
        assert!(i < 100);
        break;
    }

    // The `insecure_seed` API should return the same result each time.
    let (a1, b1) = random::insecure_seed::insecure_seed();
    let (a2, b2) = random::insecure_seed::insecure_seed();
    assert_eq!(a1, a2);
    assert_eq!(b1, b2);

    // Acquired random bytes should be of the expected length.
    let array = random::insecure::get_insecure_random_bytes(100);
    assert_eq!(array.len(), 100);

    // It shouldn't take 100+ tries to get a nonzero random integer.
    for i in 0.. {
        if random::insecure::get_insecure_random_u64() == 0 {
            continue;
        }
        assert!(i < 100);
        break;
    }
}
