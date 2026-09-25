use wasip2::clocks::monotonic_clock;

fn test_clock_time_get() {
    // Test that reading the monotonic clock succeeds. Even in environments
    // where it's not desirable to expose high-precision timers, it should
    // still succeed. `resolution` is where information about precision can be
    // provided.
    let first_time = monotonic_clock::now();
    let time = monotonic_clock::now();
    assert!(first_time <= time, "monotonic clock should be monotonic");
}

fn main() {
    // Run the tests.
    test_clock_time_get()
}
