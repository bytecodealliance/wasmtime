#![allow(dead_code, unreachable_code, unreachable_patterns)]
#![allow(unused_imports, unused_variables, non_snake_case, unused_mut)]
#![allow(
    irrefutable_let_patterns,
    unused_assignments,
    non_camel_case_types,
    missing_docs
)] // <-- Why are they allow the bad code?

include!(concat!(env!("ISLE_DIR"), "/isle_wasm.rs"));