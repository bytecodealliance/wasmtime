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
        .iter()
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
    // if the path to this file contains non-UTF-8 bytes).
    println!(
        "cargo:rerun-if-changed={}",
        crate_dir.join("build.rs").to_str().unwrap()
    );

    if let Err(err) = meta::generate(&isas, &out_dir) {
        eprintln!("Error: {}", err);
        process::exit(1);
    }

    if env::var("CRANELIFT_VERBOSE").is_ok() {
        for isa in &isas {
            println!("cargo:warning=Includes support for {} ISA", isa.to_string());
        }
        println!(
            "cargo:warning=Build step took {:?}.",
            Instant::now() - start_time
        );
        println!("cargo:warning=Generated files are in {}", out_dir);
    }

    #[cfg(feature = "rebuild-peephole-optimizers")]
    rebuild_peephole_optimizers();
}

#[cfg(feature = "rebuild-peephole-optimizers")]
fn rebuild_peephole_optimizers() {
    use std::path::Path;

    let source_path = Path::new("src").join("preopt.peepmatic");
    println!("cargo:rerun-if-changed={}", source_path.display());

    let preopt =
        peepmatic::compile_file(&source_path).expect("failed to compile `src/preopt.peepmatic`");

    preopt
        .serialize_to_file(&Path::new("src").join("preopt.serialized"))
        .expect("failed to serialize peephole optimizer to `src/preopt.serialized`");
}
