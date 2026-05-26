use test_programs::wasi::clocks::monotonic_clock::subscribe_duration;

fn main() {
    for _ in 0..1000 {
        std::mem::forget(subscribe_duration(1_000_000));
    }
    panic!("should have trapped before now");
}
