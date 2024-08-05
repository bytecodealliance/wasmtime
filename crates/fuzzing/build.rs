// A small build script to include the contents of the wast test suite into the
// final fuzzing binary so the fuzzing binary can be run elsewhere and doesn't
// rely on the original source tree.

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let dirs = [
        "tests/spec_testsuite",
        "tests/misc_testsuite",
        "tests/misc_testsuite/multi-memory",
        "tests/misc_testsuite/simd",
        "tests/misc_testsuite/threads",
    ];
    let mut root = env::current_dir().unwrap();
    root.pop(); // chop off 'fuzzing'
    root.pop(); // chop off 'crates'
    let mut code = format!("static FILES: &[(&str, &str)] = &[\n");

    let mut entries = Vec::new();
    for dir in dirs {
        for entry in root.join(dir).read_dir().unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("wast") {
                entries.push(path);
            }
        }
    }
    entries.sort();
    for path in entries {
        let path = path.to_str().expect("path is not valid utf-8");
        code.push_str(&format!("({path:?}, include_str!({path:?})),\n"));
    }
    code.push_str("];\n");
    std::fs::write(out_dir.join("wasttests.rs"), code).unwrap();
}
