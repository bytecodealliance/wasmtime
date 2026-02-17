use test_programs::wasi::clocks::monotonic_clock::subscribe_duration;

fn main() {
    loop {
        std::mem::forget(subscribe_duration(1_000_000));
    }
}
