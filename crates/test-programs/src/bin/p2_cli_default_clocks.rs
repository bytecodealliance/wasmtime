fn main() {
    let a = std::time::Instant::now();
    let b = std::time::Instant::now();
    let _ = b.checked_duration_since(a).unwrap();

    let c = std::time::SystemTime::now();
    let d = std::time::SystemTime::now();
    let _ = c.duration_since(std::time::UNIX_EPOCH).unwrap();
    let _ = d.duration_since(std::time::UNIX_EPOCH).unwrap();
}
