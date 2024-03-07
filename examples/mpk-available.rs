//! This example checks if memory protection keys (MPK) are available on the
//! current system using the
//! [`PoolingAllocationConfig::are_memory_protection_keys_available`] API.

use std::process::exit;
use wasmtime::*;

fn main() {
    if PoolingAllocationConfig::are_memory_protection_keys_available() {
        eprintln!("MPK is available");
        exit(0);
    } else {
        eprintln!("MPK is not available");
        exit(1);
    }
}
