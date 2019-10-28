#![allow(non_camel_case_types, non_snake_case, dead_code)]

include!(concat!(env!("OUT_DIR"), "/wasmtime_ssp.rs"));

pub type char = ::std::os::raw::c_char;
pub type void = ::std::os::raw::c_void;
