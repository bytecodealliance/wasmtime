fn main() {
    let pollable = wasip2::clocks::monotonic_clock::subscribe_duration(0);
    for _ in 0..20 {
        if pollable.ready() {
            return;
        }
        wasip2::clocks::monotonic_clock::subscribe_duration(0).block();
    }

    panic!("pollable should eventually be ready");
}
