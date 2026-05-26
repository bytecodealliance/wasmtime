use test_programs::wasi::clocks::monotonic_clock;

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
    p.block();
}

fn sleep_backwards_in_time() {
    let p = monotonic_clock::subscribe_instant(monotonic_clock::now() - 1);
    p.block();
}
