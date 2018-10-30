// Build script.
//
// This program is run by Cargo when building lib/codegen. It is used to generate Rust code from
// the language definitions in the lib/codegen/meta directory.
//
// Environment:
//
// OUT_DIR
//     Directory where generated files should be placed.
//
// TARGET
//     Target triple provided by Cargo.
//
// CRANELIFT_TARGETS (Optional)
//     A setting for conditional compilation of isa targets. Possible values can be "native" or
//     known isa targets separated by ','.
//
// The build script expects to be run from the directory where this build.rs file lives. The
// current directory is used to find the sources.

extern crate cranelift_codegen_meta as meta;

use meta::isa::Isa;
use std::env;
use std::process;

use std::time::Instant;

fn main() {
    let start_time = Instant::now();

    let out_dir = env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set");
    let target_triple = env::var("TARGET").expect("The TARGET environment variable must be set");
    let cranelift_targets = env::var("CRANELIFT_TARGETS").ok();
    let cranelift_targets = cranelift_targets.as_ref().map(|s| s.as_ref());
    let python = identify_python();

    // Configure isa targets cfg.
    match isa_targets(cranelift_targets, &target_triple) {
        Ok(isa_targets) => {
            for isa in &isa_targets {
                println!("cargo:rustc-cfg=build_{}", isa.to_string());
            }
        }
        Err(err) => {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    }

    let cur_dir = env::current_dir().expect("Can't access current working directory");
    let crate_dir = cur_dir.as_path();

    // Make sure we rebuild if this build script changes.
    // I guess that won't happen if you have non-UTF8 bytes in your path names.
    // The `build.py` script prints out its own dependencies.
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
    let isas = meta::isa::define_all();

    if let Err(err) = meta::gen_types::generate("types.rs", &out_dir) {
        eprintln!("Error: {}", err);
        process::exit(1);
    }

    for isa in isas {
        if let Err(err) = meta::gen_registers::generate(isa, "registers", &out_dir) {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    }

    println!(
        "cargo:warning=Cranelift meta-build step took {:?}",
        Instant::now() - start_time
    );
    println!(
        "cargo:warning=Meta-build script generated files in {}",
        out_dir
    );
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

/// Returns isa targets to configure conditional compilation.
fn isa_targets(cranelift_targets: Option<&str>, target_triple: &str) -> Result<Vec<Isa>, String> {
    match cranelift_targets {
        Some("native") => Isa::from_arch(target_triple.split('-').next().unwrap())
            .map(|isa| vec![isa])
            .ok_or_else(|| {
                format!(
                    "no supported isa found for target triple `{}`",
                    target_triple
                )
            }),
        Some(targets) => {
            let unknown_isa_targets = targets
                .split(',')
                .filter(|target| Isa::new(target).is_none())
                .collect::<Vec<_>>();
            let isa_targets = targets.split(',').flat_map(Isa::new).collect::<Vec<_>>();
            match (unknown_isa_targets.is_empty(), isa_targets.is_empty()) {
                (true, true) => Ok(Isa::all().to_vec()),
                (true, _) => Ok(isa_targets),
                (_, _) => Err(format!(
                    "unknown isa targets: `{}`",
                    unknown_isa_targets.join(", ")
                )),
            }
        }
        None => Ok(Isa::all().to_vec()),
    }
}
