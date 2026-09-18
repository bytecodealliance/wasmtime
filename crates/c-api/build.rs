use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=include");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let include_src = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("include");
    let dst = out_dir.join("include");

    generate_conf_header(&include_src, &dst);
    copy_headers(&include_src, &dst);

    println!("cargo:include={}", dst.display());
}

/// Generates `wasmtime/conf.h` from `wasmtime/conf.h.in`, mirroring what
/// `cmake/install-headers.cmake` does for standalone CMake builds: each
/// `#cmakedefine WASMTIME_FEATURE_X` line becomes `#define WASMTIME_FEATURE_X`
/// when the corresponding Cargo feature is enabled and `/* #undef ... */`
/// otherwise. The feature list is read from the template itself, so this
/// function needs no copy of the WASMTIME_FEATURE_LIST.
///
/// Note the CRLF line endings, matching cmake's `NEWLINE_STYLE CRLF` so that
/// the generated header is byte-identical either way.
fn generate_conf_header(include_src: &Path, dst: &Path) {
    let template_path = include_src.join("wasmtime").join("conf.h.in");
    let template = fs::read_to_string(&template_path)
        .unwrap_or_else(|e| panic!("failed to read {template_path:?}: {e}"));
    let mut conf = String::new();
    for line in template.lines() {
        if let Some(rest) = line.strip_prefix("#cmakedefine ") {
            let var = rest.split_whitespace().next().unwrap();
            let feature = var
                .strip_prefix("WASMTIME_FEATURE_")
                .unwrap_or_else(|| panic!("unexpected #cmakedefine {var} in conf.h.in"));
            if env::var_os(format!("CARGO_FEATURE_{feature}")).is_some() {
                conf.push_str("#define ");
                conf.push_str(var);
            } else {
                conf.push_str("/* #undef ");
                conf.push_str(var);
                conf.push_str(" */");
            }
        } else {
            conf.push_str(line);
        }
        conf.push_str("\r\n");
    }
    let conf_dir = dst.join("wasmtime");
    fs::create_dir_all(&conf_dir).unwrap();
    fs::write(conf_dir.join("conf.h"), conf).unwrap();
}

/// Copies all `.h`/`.hh` files under `include/` into `$OUT_DIR/include`,
/// preserving the directory structure (the equivalent of cmake's
/// `file(INSTALL ... FILES_MATCHING REGEX "\.hh?$")`).
fn copy_headers(src: &Path, dst: &Path) {
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            copy_headers(&path, &dst.join(entry.file_name()));
        } else {
            let is_header = path
                .extension()
                .map(|e| e == "h" || e == "hh")
                .unwrap_or(false);
            if is_header {
                fs::create_dir_all(dst).unwrap();
                fs::copy(&path, dst.join(entry.file_name())).unwrap();
            }
        }
    }
}
