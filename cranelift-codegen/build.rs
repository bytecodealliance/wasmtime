// Build script.
//
// This program is run by Cargo when building cranelift-codegen. It is used to generate Rust code from
// the language definitions in the cranelift-codegen/meta directory.
//
// Environment:
//
// OUT_DIR
//     Directory where generated files should be placed.
//
// TARGET
//     Target triple provided by Cargo.
//
// The build script expects to be run from the directory where this build.rs file lives. The
// current directory is used to find the sources.

use cranelift_codegen_meta as meta;

use std::env;
use std::process;
use std::time::Instant;

fn main() {
    let start_time = Instant::now();

    let out_dir = env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set");
    let target_triple = env::var("TARGET").expect("The TARGET environment variable must be set");

    // Configure isa targets cfg.
    let isa_targets = meta::isa::Isa::all()
        .into_iter()
        .cloned()
        .filter(|isa| {
            let env_key = format!("CARGO_FEATURE_{}", isa.to_string().to_uppercase());
            env::var(env_key).is_ok()
        })
        .collect::<Vec<_>>();

    let isas = if isa_targets.is_empty() {
        // Try to match native target.
        let target_name = target_triple.split('-').next().unwrap();
        let isa = meta::isa_from_arch(&target_name).expect("error when identifying target");
        println!("cargo:rustc-cfg=feature=\"{}\"", isa);
        vec![isa]
    } else {
        isa_targets
    };

    let cur_dir = env::current_dir().expect("Can't access current working directory");
    let crate_dir = cur_dir.as_path();

    // Make sure we rebuild if this build script changes (will not happen with
    // if the path to this file contains non-UTF8 bytes). The `build.py` script
    // prints out its own dependencies.
    println!(
        "cargo:rerun-if-changed={}",
        crate_dir.join("build.rs").to_str().unwrap()
    );

    // Scripts are in `$crate_dir/meta-python`.
    let meta_dir = crate_dir.join("meta-python");
    let build_script = meta_dir.join("build.py");

    // Launch build script with Python. We'll just find python in the path.
    // Use -B to disable .pyc files, because they cause trouble for vendoring
    // scripts, and this is a build step that isn't run very often anyway.
    let python = identify_python();
    let status = process::Command::new(python)
        .current_dir(crate_dir)
        .arg("-B")
        .arg(build_script)
        .arg("--out-dir")
        .arg(out_dir.clone())
        .status()
        .expect("Failed to launch second-level build script; is python installed?");
    if !status.success() {
        process::exit(status.code().unwrap());
    }

    // DEVELOPMENT:
    // ------------------------------------------------------------------------
    // Now that the Python build process is complete, generate files that are
    // emitted by the `meta` crate.
    // ------------------------------------------------------------------------

    if let Err(err) = meta::generate(&isas, &out_dir) {
        eprintln!("Error: {}", err);
        process::exit(1);
    }

    if let Ok(_) = env::var("CRANELIFT_VERBOSE") {
        for isa in &isas {
            println!("cargo:warning=Includes support for {} ISA", isa.to_string());
        }
        println!(
            "cargo:warning=Build step took {:?}.",
            Instant::now() - start_time
        );
        println!("cargo:warning=Generated files are in {}", out_dir);
    }
}

fn identify_python() -> &'static str {
    for python in &["python", "python3", "python2.7"] {
        if process::Command::new(python)
            .arg("--version")
            .status()
            .is_ok()
        {
            return python;
        }
    }
    panic!("The Cranelift build requires Python (version 2.7 or version 3)");
}
