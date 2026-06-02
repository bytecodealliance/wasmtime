use std::process::Command;
use std::str;

fn main() {
    let (compiler_version, use_mtime) =
        match Command::new("git").args(["rev-parse", "HEAD"]).output() {
            Ok(output) if output.status.success() => (
                str::from_utf8(&output.stdout).unwrap().trim().to_string(),
                true,
            ),
            _ => (env!("CARGO_PKG_VERSION").to_string(), false),
        };
    println!("cargo:rustc-env=COMPILER_VERSION={compiler_version}");
    println!("cargo:rustc-env=USE_MTIME={use_mtime}");
}
