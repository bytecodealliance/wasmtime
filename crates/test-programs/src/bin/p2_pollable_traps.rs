fn main() {
    // Polling an empty list should trap:
    test_programs::wasi::io::poll::poll(&[]);
}
