use test_programs::wasi::clocks0_2_3::monotonic_clock;

fn main() {
    sleep_10ms();
    sleep_0ms();
    sleep_backwards_in_time();
}

fn sleep_10ms() {
    let dur = 10_000_000;
    let p = monotonic_clock::subscribe_instant(monotonic_clock::now() + dur);
    p.block();
    let p = monotonic_clock::subscribe_duration(dur);
    p.block();
}

fn sleep_0ms() {
    let p = monotonic_clock::subscribe_instant(monotonic_clock::now());
    p.block();
    let p = monotonic_clock::subscribe_duration(0);
    assert!(
        p.ready(),
        "timer subscription with duration 0 is ready immediately"
    );
}

fn sleep_backwards_in_time() {
    let p = monotonic_clock::subscribe_instant(monotonic_clock::now() - 1);
    assert!(
        p.ready(),
        "timer subscription for instant which has passed is ready immediately"
    );
}
