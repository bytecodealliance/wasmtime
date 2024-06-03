fn main() {
    let greeting = test_programs::rpc_hello::rpc_examples::hello::handler::hello("wasmtime");
    print!("{greeting}")
}
