extern crate bindgen;
extern crate cmake;
extern crate regex;

use cmake::Config;
use regex::Regex;
use std::env;
use std::path::PathBuf;

fn main() {
    let dst = Config::new("signalhandlers").build();

    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static=SignalHandlers");

    let mut bindings_builder = bindgen::Builder::default()
        .header("signalhandlers/SignalHandlers.hpp")
        .whitelist_type("TrapContext")
        .whitelist_type("jmp_buf")
        .whitelist_function("EnsureEagerSignalHandlers");

    // If we're compiling for Darwin, compile in extra Darwin support routines.
    if Regex::new(r"-darwin[[:digit:].]*$")
        .unwrap()
        .is_match(&env::var("TARGET").unwrap())
    {
        bindings_builder = bindings_builder.whitelist_function("EnsureDarwinMachPorts");
    }

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings_builder
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join("signalhandlers.rs"))
        .expect("Couldn't write bindings!");
}
