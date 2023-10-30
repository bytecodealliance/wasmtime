use test_programs::wasi::{clocks::monotonic_clock, io::poll};

fn main() {
    sleep_10ms();
    sleep_0ms();
    sleep_backwards_in_time();
}

fn sleep_10ms() {
    let dur = 10_000_000;
    let p = monotonic_clock::subscribe_instant(monotonic_clock::now() + dur);
    poll::poll_one(&p);
    let p = monotonic_clock::subscribe_duration(dur);
    poll::poll_one(&p);
}

fn sleep_0ms() {
    let p = monotonic_clock::subscribe_instant(monotonic_clock::now());
    poll::poll_one(&p);
    let p = monotonic_clock::subscribe_duration(0);
    poll::poll_one(&p);
}

fn sleep_backwards_in_time() {
    let p = monotonic_clock::subscribe_instant(monotonic_clock::now() - 1);
    poll::poll_one(&p);
}
