use test_programs::wasi::{clocks::monotonic_clock, io::poll};

fn main() {
    sleep_10ms();
    sleep_0ms();
    sleep_backwards_in_time();
}

fn sleep_10ms() {
    let dur = 10_000_000;
    let p = monotonic_clock::subscribe(monotonic_clock::now() + dur, true);
    poll::poll_one(&p);
    let p = monotonic_clock::subscribe(dur, false);
    poll::poll_one(&p);
}

fn sleep_0ms() {
    let p = monotonic_clock::subscribe(monotonic_clock::now(), true);
    poll::poll_one(&p);
    let p = monotonic_clock::subscribe(0, false);
    poll::poll_one(&p);
}

fn sleep_backwards_in_time() {
    let p = monotonic_clock::subscribe(monotonic_clock::now() - 1, true);
    poll::poll_one(&p);
}
