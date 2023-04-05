// The macro will generate a macro for defining exports which we won't be reusing
#![allow(unused)]
wit_bindgen::generate!({ path: "../../wasi-http/wasi-http/wit" });
