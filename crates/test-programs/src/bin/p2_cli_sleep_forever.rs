use std::time::Duration;

fn main() {
    std::thread::sleep(Duration::from_nanos(u64::MAX));
}
