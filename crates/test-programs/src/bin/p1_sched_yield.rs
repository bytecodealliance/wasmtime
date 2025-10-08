#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

unsafe fn test_sched_yield() {
    wasip1::sched_yield().expect("sched_yield");
}

fn main() {
    // Run tests
    unsafe { test_sched_yield() }
}
