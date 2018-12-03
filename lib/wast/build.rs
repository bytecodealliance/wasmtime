use std::env;
use std::fs::{read_dir, File};
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out_dir =
        PathBuf::from(env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set"));
    let mut out =
        File::create(out_dir.join("run_wast_files.rs")).expect("error creating run_wast_files.rs");

    let mut paths: Vec<_> = read_dir("spec_testsuite")
        .unwrap()
        .map(|r| r.unwrap())
        .filter(|p| {
            // Ignore files starting with `.`, which could be editor temporary files
            if let Some(stem) = p.path().file_stem() {
                if let Some(stemstr) = stem.to_str() {
                    return !stemstr.starts_with('.');
                }
            }
            false
        }).collect();

    paths.sort_by_key(|dir| dir.path());
    for path in paths {
        let path = path.path();
        writeln!(out, "#[test]");
        writeln!(
            out,
            "fn {}() {{",
            avoid_keywords(
                &path
                    .file_stem()
                    .expect("file_stem")
                    .to_str()
                    .expect("to_str")
                    .replace("-", "_")
            )
        );
        writeln!(
            out,
            "    wast_file(Path::new(\"{}\"), &*native_isa()).expect(\"error loading wast file {}\");",
            path.display(),
            path.display()
        );
        writeln!(out, "}}");
        writeln!(out);
    }
}

fn avoid_keywords(name: &str) -> &str {
    match name {
        "if" => "if_",
        "loop" => "loop_",
        "type" => "type_",
        "const" => "const_",
        "return" => "return_",
        other => other,
    }
}
