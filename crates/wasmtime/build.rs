fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let target_pointer_width = std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target = std::env::var("TARGET").unwrap();

    // Print a more first-class error for 32-bit platforms as none are currently
    // supported. Note that if Wasmtime grows support then this error probably
    // wants to go away entirely. In the meantime the purpose of this is to
    // help guide users locally rather than requiring deciphering of errors via
    // issues or zulip questions.
    if target_pointer_width == "32" {
        eprintln!(
            "

Wasmtime does not currently support any 32-bit platforms and will fail to
compile on these platforms. The current platform being targeted is:

    target: {target}
      arch: {target_arch}

"
        );
        let issue = match target_arch.as_str() {
            "x86" => Some(1980),
            "arm" => Some(1173),
            "riscv" => Some(8768),
            _ => None,
        };

        match issue {
            Some(i) => {
                eprintln!(
                    "\
the tracking issue for supporting this platform is:

    https://github.com/bytecodealliance/wasmtime/issues/{i}

"
                );
            }
            None => {
                eprintln!(
                    "\
there is not tracking issue for this platform but if you would like to see
Wasmtime support this platform please open an issue at

    https://github.com/bytecodealliance/wasmtime/issues/new
    //
"
                );
            }
        }

        std::process::exit(1);
    }

    #[cfg(feature = "runtime")]
    build_c_helpers();
}

#[cfg(feature = "runtime")]
fn build_c_helpers() {
    use wasmtime_versioned_export_macros::versioned_suffix;

    // NB: duplicating a workaround in the wasmtime-fiber build script.
    println!("cargo:rustc-check-cfg=cfg(asan)");
    match std::env::var("CARGO_CFG_SANITIZE") {
        Ok(s) if s == "address" => {
            println!("cargo:rustc-cfg=asan");
        }
        _ => {}
    }

    // If this platform is neither unix nor windows then there's no default need
    // for a C helper library since `helpers.c` is tailored for just these
    // platforms currently.
    if std::env::var("CARGO_CFG_UNIX").is_err() && std::env::var("CARGO_CFG_WINDOWS").is_err() {
        return;
    }

    let mut build = cc::Build::new();
    build.warnings(true);
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    build.define(&format!("CFG_TARGET_OS_{}", os), None);
    build.define(&format!("CFG_TARGET_ARCH_{}", arch), None);
    build.define("VERSIONED_SUFFIX", Some(versioned_suffix!()));
    println!("cargo:rerun-if-changed=src/runtime/vm/helpers.c");
    build.file("src/runtime/vm/helpers.c");
    build.compile("wasmtime-helpers");

    if os == "linux" {
        println!("cargo:rustc-link-lib=m");
    }
}
