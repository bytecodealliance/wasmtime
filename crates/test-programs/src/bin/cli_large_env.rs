fn main() {
    for (k, v) in std::env::vars() {
        println!("{k}={v}");
    }
}
