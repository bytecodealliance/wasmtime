use std::time::{Duration, Instant, SystemTime};

fn main() {
    let then = Instant::now();

    assert_eq!(Duration::from_secs(42), then.elapsed());

    assert_eq!(
        SystemTime::UNIX_EPOCH + Duration::new(1431648000, 100),
        SystemTime::now()
    );
}
