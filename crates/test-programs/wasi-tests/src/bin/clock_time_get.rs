use more_asserts::assert_le;
use wasi::wasi_unstable;
use wasi_tests::wasi_wrappers::wasi_clock_time_get;

unsafe fn test_clock_time_get() {
    // Test that clock_time_get succeeds. Even in environments where it's not
    // desirable to expose high-precision timers, it should still succeed.
    // clock_res_get is where information about precision can be provided.
    let mut time: wasi_unstable::Timestamp = 0;
    let status = wasi_clock_time_get(wasi_unstable::CLOCK_MONOTONIC, 1, &mut time);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "clock_time_get with a precision of 1"
    );

    let status = wasi_clock_time_get(wasi_unstable::CLOCK_MONOTONIC, 0, &mut time);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "clock_time_get with a precision of 0"
    );
    let first_time = time;

    let status = wasi_clock_time_get(wasi_unstable::CLOCK_MONOTONIC, 0, &mut time);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "clock_time_get with a precision of 0"
    );
    assert_le!(first_time, time, "CLOCK_MONOTONIC should be monotonic");
}

fn main() {
    // Run the tests.
    unsafe { test_clock_time_get() }
}
