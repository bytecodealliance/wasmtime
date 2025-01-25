use std::thread::sleep;
use std::time::{Duration, Instant};

fn main() {
    println!("Hello, world!");
    let start = Instant::now();
    sleep(Duration::from_millis(100));
    println!("Napped for {:?}", Instant::now().duration_since(start));
}
