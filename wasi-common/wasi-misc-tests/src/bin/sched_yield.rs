use wasi::wasi_unstable;

fn test_sched_yield() {
    assert!(wasi_unstable::sched_yield().is_ok(), "sched_yield");
}

fn main() {
    // Run tests
    test_sched_yield()
}
