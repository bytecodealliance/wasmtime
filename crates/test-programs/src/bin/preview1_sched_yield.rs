unsafe fn test_sched_yield() {
    wasi::sched_yield().expect("sched_yield");
}

fn main() {
    // Run tests
    unsafe { test_sched_yield() }
}
