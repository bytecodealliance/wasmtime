pub mod http;
pub mod preview1;
pub mod sockets;

wit_bindgen::generate!("test-command" in "../wasi/wit");
