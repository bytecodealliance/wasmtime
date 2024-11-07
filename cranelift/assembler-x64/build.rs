use cranelift_assembler_x64_meta as meta;
use std::env;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set");
    let out_dir = Path::new(&out_dir);
    let built_files = [
        meta::generate_rust_assembler(out_dir.join("assembler.rs")),
        meta::generate_isle_macro(out_dir.join("assembler-isle-macro.rs")),
        meta::generate_isle_definitions(out_dir.join("assembler-definitions.isle")),
    ];

    println!(
        "cargo:rustc-env=ASSEMBLER_BUILT_FILES={}",
        built_files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(":")
    );
}
