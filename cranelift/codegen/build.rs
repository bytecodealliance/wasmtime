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
use cranelift_isle::error::Errors;
use meta::isle::IsleCompilation;

use std::env;
use std::io::Read;
use std::process;
use std::time::Instant;

fn main() {
    let start_time = Instant::now();

    let out_dir = env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set");
    let out_dir = std::path::Path::new(&out_dir);
    let target_triple = env::var("TARGET").expect("The TARGET environment variable must be set");

    let all_arch = env::var("CARGO_FEATURE_ALL_ARCH").is_ok();

    let mut isas = meta::isa::Isa::all()
        .iter()
        .cloned()
        .filter(|isa| {
            let env_key = format!("CARGO_FEATURE_{}", isa.to_string().to_uppercase());
            all_arch || env::var(env_key).is_ok()
        })
        .collect::<Vec<_>>();

    // Don't require host isa if under 'all-arch' feature.
    let host_isa = env::var("CARGO_FEATURE_HOST_ARCH").is_ok() && !all_arch;

    if isas.is_empty() || host_isa {
        // Try to match native target.
        let target_name = target_triple.split('-').next().unwrap();
        let isa = meta::isa_from_arch(&target_name).expect("error when identifying target");
        println!("cargo:rustc-cfg=feature=\"{}\"", isa);
        isas.push(isa);
    }

    let cur_dir = env::current_dir().expect("Can't access current working directory");
    let crate_dir = cur_dir.as_path();

    println!("cargo:rerun-if-changed=build.rs");

    let explicit_isle_dir = &crate_dir.join("isle_generated_code");
    #[cfg(feature = "isle-in-source-tree")]
    let isle_dir = explicit_isle_dir;
    #[cfg(not(feature = "isle-in-source-tree"))]
    let isle_dir = &out_dir;

    #[cfg(feature = "isle-in-source-tree")]
    {
        std::fs::create_dir_all(isle_dir).expect("Could not create ISLE source directory");
    }
    #[cfg(not(feature = "isle-in-source-tree"))]
    {
        if explicit_isle_dir.is_dir() {
            eprintln!(concat!(
                "Error: directory isle_generated_code/ exists but is only used when\n",
                "`--feature isle-in-source-tree` is specified. To prevent confusion,\n",
                "this build script requires the directory to be removed when reverting\n",
                "to the usual generated code in target/. Please delete the directory and\n",
                "re-run this build.\n",
            ));
            std::process::exit(1);
        }
    }

    if let Err(err) = meta::generate(&isas, &out_dir, isle_dir) {
        eprintln!("Error: {}", err);
        process::exit(1);
    }

    if &std::env::var("SKIP_ISLE").unwrap_or("0".to_string()) != "1" {
        if let Err(err) = build_isle(crate_dir, isle_dir) {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    }

    if env::var("CRANELIFT_VERBOSE").is_ok() {
        for isa in &isas {
            println!("cargo:warning=Includes support for {} ISA", isa.to_string());
        }
        println!(
            "cargo:warning=Build step took {:?}.",
            Instant::now() - start_time
        );
        println!("cargo:warning=Generated files are in {}", out_dir.display());
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

/// Strip the current directory from the file paths, because `islec`
/// includes them in the generated source, and this helps us maintain
/// deterministic builds that don't include those local file paths.
fn make_isle_source_path_relative(
    cur_dir: &std::path::Path,
    filename: &std::path::Path,
) -> std::path::PathBuf {
    if let Ok(suffix) = filename.strip_prefix(&cur_dir) {
        suffix.to_path_buf()
    } else {
        filename.to_path_buf()
    }
}

fn build_isle(
    crate_dir: &std::path::Path,
    isle_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let cur_dir = std::env::current_dir()?;
    let isle_compilations = meta::isle::get_isle_compilations(
        &make_isle_source_path_relative(&cur_dir, &crate_dir),
        &make_isle_source_path_relative(&cur_dir, &isle_dir),
    );

    let mut had_error = false;
    for compilation in &isle_compilations.items {
        for file in &compilation.tracked_inputs {
            println!("cargo:rerun-if-changed={}", file.display());
        }

        if let Err(e) = run_compilation(compilation) {
            had_error = true;
            eprintln!("Error building ISLE files:");
            eprintln!("{:?}", e);
            #[cfg(not(feature = "isle-errors"))]
            {
                eprintln!("To see a more detailed error report, run: ");
                eprintln!();
                eprintln!("    $ cargo check -p cranelift-codegen --features isle-errors");
                eprintln!();
            }
        }
    }

    if had_error {
        std::process::exit(1);
    }

    println!("cargo:rustc-env=ISLE_DIR={}", isle_dir.to_str().unwrap());

    Ok(())
}

/// Build ISLE DSL source text into generated Rust code.
///
/// NB: This must happen *after* the `cranelift-codegen-meta` functions, since
/// it consumes files generated by them.
fn run_compilation(compilation: &IsleCompilation) -> Result<(), Errors> {
    use cranelift_isle as isle;

    eprintln!("Rebuilding {}", compilation.output.display());

    let code = {
        let file_paths = compilation
            .tracked_inputs
            .iter()
            .chain(compilation.untracked_inputs.iter());

        let mut options = isle::codegen::CodegenOptions::default();
        // Because we include!() the generated ISLE source, we cannot
        // put the global pragmas (`#![allow(...)]`) in the ISLE
        // source itself; we have to put them in the source that
        // include!()s it. (See
        // https://github.com/rust-lang/rust/issues/47995.)
        options.exclude_global_allow_pragmas = true;

        isle::compile::from_files(file_paths, &options)?
    };

    let code = rustfmt(&code).unwrap_or_else(|e| {
        println!(
            "cargo:warning=Failed to run `rustfmt` on ISLE-generated code: {:?}",
            e
        );
        code
    });

    eprintln!(
        "Writing ISLE-generated Rust code to {}",
        compilation.output.display()
    );
    std::fs::write(&compilation.output, code)
        .map_err(|e| Errors::from_io(e, "failed writing output"))?;

    Ok(())
}

fn rustfmt(code: &str) -> std::io::Result<String> {
    use std::io::Write;

    let mut rustfmt = std::process::Command::new("rustfmt")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let mut stdin = rustfmt.stdin.take().unwrap();
    stdin.write_all(code.as_bytes())?;
    drop(stdin);

    let mut stdout = rustfmt.stdout.take().unwrap();
    let mut data = vec![];
    stdout.read_to_end(&mut data)?;

    let status = rustfmt.wait()?;
    if !status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("`rustfmt` exited with status {}", status),
        ));
    }

    Ok(String::from_utf8(data).expect("rustfmt always writs utf-8 to stdout"))
}
