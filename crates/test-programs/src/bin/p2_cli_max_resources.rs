fn main() {
    let mut buf = Vec::new();
    for _ in 0..100 {
        buf.push(wasip2::clocks::monotonic_clock::subscribe_duration(0));
    }
}
