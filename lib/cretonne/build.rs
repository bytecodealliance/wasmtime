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
// TARGET
//     Target triple provided by Cargo.
//
// CRETONNE_TARGETS (Optional)
//     A setting for conditional compilation of isa targets. Possible values can be "native" or
//     known isa targets separated by ','.
//
// The build script expects to be run from the directory where this build.rs file lives. The
// current directory is used to find the sources.


use std::env;
use std::process;

fn main() {
    let out_dir = env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set");
    let target_triple = env::var("TARGET").expect("The TARGET environment variable must be set");
    let cretonne_targets = env::var("CRETONNE_TARGETS").ok();
    let cretonne_targets = cretonne_targets.as_ref().map(|s| s.as_ref());

    // Configure isa targets cfg.
    match isa_targets(cretonne_targets, &target_triple) {
        Ok(isa_targets) => {
            for isa in &isa_targets {
                println!("cargo:rustc-cfg=build_{}", isa.name());
            }
        }
        Err(err) => {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    }

    println!("Build script generating files in {}", out_dir);

    let cur_dir = env::current_dir().expect("Can't access current working directory");
    let crate_dir = cur_dir.as_path();

    // Make sure we rebuild is this build script changes.
    // I guess that won't happen if you have non-UTF8 bytes in your path names.
    // The `build.py` script prints out its own dependencies.
    println!("cargo:rerun-if-changed={}",
             crate_dir.join("build.rs").to_string_lossy());

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

/// Represents known ISA target.
#[derive(Copy, Clone)]
enum Isa {
    Riscv,
    Intel,
    Arm32,
    Arm64,
}

impl Isa {
    /// Creates isa target using name.
    fn new(name: &str) -> Option<Self> {
        Isa::all()
            .iter()
            .cloned()
            .filter(|isa| isa.name() == name)
            .next()
    }

    /// Creates isa target from arch.
    fn from_arch(arch: &str) -> Option<Isa> {
        Isa::all()
            .iter()
            .cloned()
            .filter(|isa| isa.is_arch_applicable(arch))
            .next()
    }

    /// Returns all supported isa targets.
    fn all() -> [Isa; 4] {
        [Isa::Riscv, Isa::Intel, Isa::Arm32, Isa::Arm64]
    }

    /// Returns name of the isa target.
    fn name(&self) -> &'static str {
        match *self {
            Isa::Riscv => "riscv",
            Isa::Intel => "intel",
            Isa::Arm32 => "arm32",
            Isa::Arm64 => "arm64",
        }
    }

    /// Checks if arch is applicable for the isa target.
    fn is_arch_applicable(&self, arch: &str) -> bool {
        match *self {
            Isa::Riscv => arch == "riscv",
            Isa::Intel => ["x86_64", "i386", "i586", "i686"].contains(&arch),
            Isa::Arm32 => arch.starts_with("arm") || arch.starts_with("thumb"),
            Isa::Arm64 => arch == "aarch64",
        }
    }
}

/// Returns isa targets to configure conditional compilation.
fn isa_targets(cretonne_targets: Option<&str>, target_triple: &str) -> Result<Vec<Isa>, String> {
    match cretonne_targets {
        Some("native") => {
            Isa::from_arch(target_triple.split('-').next().unwrap())
                .map(|isa| vec![isa])
                .ok_or_else(|| {
                                format!("no supported isa found for target triple `{}`",
                                        target_triple)
                            })
        }
        Some(targets) => {
            let unknown_isa_targets = targets
                .split(',')
                .filter(|target| Isa::new(target).is_none())
                .collect::<Vec<_>>();
            let isa_targets = targets.split(',').flat_map(Isa::new).collect::<Vec<_>>();
            match (unknown_isa_targets.is_empty(), isa_targets.is_empty()) {
                (true, true) => Ok(Isa::all().to_vec()),
                (true, _) => Ok(isa_targets),
                (_, _) => Err(format!("unknown isa targets: `{}`", unknown_isa_targets.join(", "))),
            }
        }
        None => Ok(Isa::all().to_vec()),
    }
}
