// A small build script to include the contents of the spec test suite into the
// final fuzzing binary so the fuzzing binary can be run elsewhere and doesn't
// rely on the original source tree.

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let dir = env::current_dir()
        .unwrap()
        .join("../../tests/spec_testsuite");
    let mut code = format!("static FILES: &[(&str, &str)] = &[\n");
    let entries = dir
        .read_dir()
        .unwrap()
        .map(|p| p.unwrap().path().display().to_string())
        .collect::<Vec<_>>();
    for path in entries {
        if !path.ends_with(".wast") {
            continue;
        }
        code.push_str(&format!("({:?}, include_str!({0:?})),\n", path));
    }
    code.push_str("];\n");
    std::fs::write(out_dir.join("spectests.rs"), code).unwrap();
}
