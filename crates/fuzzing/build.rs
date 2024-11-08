// A small build script to include the contents of the wast test suite into the
// final fuzzing binary so the fuzzing binary can be run elsewhere and doesn't
// rely on the original source tree.

use std::env;
use std::path::PathBuf;
use wasmtime_wast_util::WastTest;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let mut root = env::current_dir().unwrap();
    root.pop(); // chop off 'fuzzing'
    root.pop(); // chop off 'crates'

    let tests = wasmtime_wast_util::find_tests(&root).unwrap();

    let mut code = format!("static FILES: &[fn() -> wasmtime_wast_util::WastTest] = &[\n");

    for test in tests {
        let WastTest {
            path,
            contents: _,
            config,
        } = test;
        println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
        code.push_str(&format!(
            "|| {{
                wasmtime_wast_util::WastTest {{
                    path: {path:?}.into(),
                    contents: include_str!({path:?}).into(),
                    config: wasmtime_wast_util::{config:?},
                }}
            }},"
        ));
    }

    code.push_str("];\n");
    std::fs::write(out_dir.join("wasttests.rs"), code).unwrap();
}
