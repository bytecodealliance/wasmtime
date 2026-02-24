use std::mem::MaybeUninit;
use test_programs::wasi::random;

fn main() {
    let p1_random_size: usize = std::env::var("TEST_P1_RANDOM_LEN")
        .map(|v| v.parse().expect("TEST_P1_RANDOM_LEN should be a usize"))
        .unwrap_or(256);
    let mut bytes: Box<[MaybeUninit<u8>]> = Box::new_uninit_slice(p1_random_size);
    unsafe {
        wasip1::random_get(bytes.as_mut_ptr() as *mut u8, bytes.len()).unwrap();
    }

    assert!(bytes.iter().any(|x| unsafe { x.assume_init() } != 0));

    let p2_random_size: u64 = std::env::var("TEST_P2_RANDOM_LEN")
        .map(|v| v.parse().expect("TEST_P2_RANDOM_LEN should be a u64"))
        .unwrap_or(256);
    // Acquired random bytes should be of the expected length.
    let array = random::random::get_random_bytes(p2_random_size);
    assert_eq!(array.len(), p2_random_size as usize);

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

    let p2_insecure_random_size: u64 = std::env::var("TEST_P2_INSECURE_RANDOM_LEN")
        .map(|v| {
            v.parse()
                .expect("TEST_P2_INSECURE_RANDOM_LEN should be a u64")
        })
        .unwrap_or(256);
    // Acquired random bytes should be of the expected length.
    let array = random::insecure::get_insecure_random_bytes(p2_insecure_random_size);
    assert_eq!(array.len(), p2_insecure_random_size as usize);

    // It shouldn't take 100+ tries to get a nonzero random integer.
    for i in 0.. {
        if random::insecure::get_insecure_random_u64() == 0 {
            continue;
        }
        assert!(i < 100);
        break;
    }
}
