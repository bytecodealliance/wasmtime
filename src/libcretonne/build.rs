
// Build script.
//
// This program is run by Cargo when building libcretonne. It is used to generate Rust code from
// the language definitions in the meta directory.
//
// Environment:
//
// OUT_DIR
//     Directory where generated files should be placed.
//
// The build script expects to be run from the directory where this build.rs file lives. The
// current directory is used to find the sources.


use std::env;
use std::process;

fn main() {
    let out_dir = env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set");

    println!("Build script generating files in {}", out_dir);

    let mut cur_dir = env::current_dir().expect("Can't access current working directory");

    // We're in src/libcretonne. Find the top-level directory.
    assert!(cur_dir.pop(), "No parent 'src' directory");
    assert!(cur_dir.pop(), "No top-level directory");
    let top_dir = cur_dir.as_path();

    // Scripts are in $top_dir/meta.
    let meta_dir = top_dir.join("meta");
    let build_script = meta_dir.join("build.py");

    // Let Cargo known that this script should be rerun if anything changes in the meta directory.
    println!("cargo:rerun-if-changed={}", meta_dir.display());

    // Launch build script with Python. We'll just find python in the path.
    let status = process::Command::new("python")
                     .current_dir(top_dir)
                     .arg(build_script)
                     .arg("--out-dir")
                     .arg(out_dir)
                     .status()
                     .expect("Failed to launch second-level build script");
    if !status.success() {
        process::exit(status.code().unwrap());
    }
}
