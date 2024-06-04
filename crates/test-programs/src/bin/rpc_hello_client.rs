fn main() {
    let greeting = test_programs::rpc_hello::rpc_test::hello::handler::hello("wasmtime");
    print!("{greeting}")
}
