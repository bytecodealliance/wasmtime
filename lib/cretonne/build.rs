// Build script.
//
// This program is run by Cargo when building lib/cretonne. It is used to generate Rust code from
// the language definitions in the lib/cretonne/meta directory.
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

    let cur_dir = env::current_dir().expect("Can't access current working directory");
    let crate_dir = cur_dir.as_path();

    // Scripts are in `$crate_dir/meta`.
    let meta_dir = crate_dir.join("meta");
    let build_script = meta_dir.join("build.py");

    // Launch build script with Python. We'll just find python in the path.
    let status = process::Command::new("python")
        .current_dir(crate_dir)
        .arg(build_script)
        .arg("--out-dir")
        .arg(out_dir)
        .status()
        .expect("Failed to launch second-level build script");
    if !status.success() {
        process::exit(status.code().unwrap());
    }
}
