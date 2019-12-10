use more_asserts::assert_le;

unsafe fn test_clock_time_get() {
    // Test that clock_time_get succeeds. Even in environments where it's not
    // desirable to expose high-precision timers, it should still succeed.
    // clock_res_get is where information about precision can be provided.
    wasi::clock_time_get(wasi::CLOCKID_MONOTONIC, 1).expect("precision 1 should work");

    let first_time =
        wasi::clock_time_get(wasi::CLOCKID_MONOTONIC, 0).expect("precision 0 should work");

    let time = wasi::clock_time_get(wasi::CLOCKID_MONOTONIC, 0).expect("re-fetch time should work");
    assert_le!(first_time, time, "CLOCK_MONOTONIC should be monotonic");
}

fn main() {
    // Run the tests.
    unsafe { test_clock_time_get() }
}
