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
use std::io::Read;
use std::process;
use std::time::Instant;

fn main() {
    let start_time = Instant::now();

    let out_dir = env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set");
    let target_triple = env::var("TARGET").expect("The TARGET environment variable must be set");

    let new_backend_isas = if env::var("CARGO_FEATURE_X64").is_ok() {
        // The x64 (new backend for x86_64) is a bit particular: it only requires generating
        // the shared meta code; the only ISA-specific code is for settings.
        vec![meta::isa::Isa::X86]
    } else {
        Vec::new()
    };

    // Configure isa targets using the old backend.
    let isa_targets = meta::isa::Isa::all()
        .iter()
        .cloned()
        .filter(|isa| {
            let env_key = format!("CARGO_FEATURE_{}", isa.to_string().to_uppercase());
            env::var(env_key).is_ok()
        })
        .collect::<Vec<_>>();

    let old_backend_isas = if new_backend_isas.is_empty() && isa_targets.is_empty() {
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

    if let Err(err) = meta::generate(&old_backend_isas, &new_backend_isas, &out_dir) {
        eprintln!("Error: {}", err);
        process::exit(1);
    }

    if env::var("CRANELIFT_VERBOSE").is_ok() {
        for isa in &old_backend_isas {
            println!(
                "cargo:warning=Includes old-backend support for {} ISA",
                isa.to_string()
            );
        }
        for isa in &new_backend_isas {
            println!(
                "cargo:warning=Includes new-backend support for {} ISA",
                isa.to_string()
            );
        }
        println!(
            "cargo:warning=Build step took {:?}.",
            Instant::now() - start_time
        );
        println!("cargo:warning=Generated files are in {}", out_dir);
    }

    #[cfg(feature = "rebuild-peephole-optimizers")]
    {
        std::fs::write(
            std::path::Path::new(&out_dir).join("CRANELIFT_CODEGEN_PATH"),
            cur_dir.to_str().unwrap(),
        )
        .unwrap()
    }

    let pkg_version = env::var("CARGO_PKG_VERSION").unwrap();
    let mut cmd = std::process::Command::new("git");
    cmd.arg("rev-parse")
        .arg("HEAD")
        .stdout(std::process::Stdio::piped())
        .current_dir(env::var("CARGO_MANIFEST_DIR").unwrap());
    let version = if let Ok(mut child) = cmd.spawn() {
        let mut git_rev = String::new();
        child
            .stdout
            .as_mut()
            .unwrap()
            .read_to_string(&mut git_rev)
            .unwrap();
        let status = child.wait().unwrap();
        if status.success() {
            let git_rev = git_rev.trim().chars().take(9).collect::<String>();
            format!("{}-{}", pkg_version, git_rev)
        } else {
            // not a git repo
            pkg_version
        }
    } else {
        // git not available
        pkg_version
    };
    std::fs::write(
        std::path::Path::new(&out_dir).join("version.rs"),
        format!(
            "/// Version number of this crate. \n\
            pub const VERSION: &str = \"{}\";",
            version
        ),
    )
    .unwrap();
}
