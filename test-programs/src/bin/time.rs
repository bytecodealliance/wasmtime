use std::time::{Duration, Instant, SystemTime};

fn main() {
    let then = Instant::now();

    assert_eq!(Duration::from_secs(42), then.elapsed());

    assert_eq!(
        SystemTime::UNIX_EPOCH + Duration::from_secs(1431648000),
        SystemTime::now()
    );
}
