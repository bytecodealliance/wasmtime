fn main() {
    let name = std::env::var("NAME").unwrap();
    println!("Hello, world! My name is {}", name);
    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("Goodbye from {}", name);
}
