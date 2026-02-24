fn main() {
    let mut buf = Vec::new();
    for _ in 0..100 {
        buf.push(test_programs::wasi::clocks::monotonic_clock::subscribe_duration(0));
    }
}
