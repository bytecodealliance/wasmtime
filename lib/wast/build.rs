use std::env;
use std::fs::{read_dir, File};
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out_dir =
        PathBuf::from(env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set"));
    let mut out =
        File::create(out_dir.join("run_wast_files.rs")).expect("error creating run_wast_files.rs");

    test_directory(&mut out, "misc_testsuite");
    test_directory(&mut out, "spec_testsuite");
}

fn test_directory(out: &mut File, testsuite: &str) {
    let mut dir_entries: Vec<_> = read_dir(testsuite)
        .unwrap()
        .map(|r| r.unwrap())
        .filter(|dir_entry| {
            let p = dir_entry.path();
            if let Some(ext) = p.extension() {
                // Only look at wast files.
                if ext == "wast" {
                    // Ignore files starting with `.`, which could be editor temporary files
                    if let Some(stem) = p.file_stem() {
                        if let Some(stemstr) = stem.to_str() {
                            if !stemstr.starts_with('.') {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        }).collect();

    dir_entries.sort_by_key(|dir| dir.path());

    writeln!(out, "mod {} {{", testsuite);
    writeln!(out, "    use super::{{native_isa, wast_file, Path}};");
    for dir_entry in dir_entries {
        let path = dir_entry.path();
        let stemstr = path
            .file_stem()
            .expect("file_stem")
            .to_str()
            .expect("to_str");

        writeln!(out, "    #[test]");
        if ignore(testsuite, stemstr) {
            writeln!(out, "    #[ignore]");
        }
        writeln!(
            out,
            "    fn {}() {{",
            avoid_keywords(&stemstr.replace("-", "_"))
        );
        writeln!(
            out,
            "        wast_file(Path::new(\"{}\"), &*native_isa()).expect(\"error loading wast file {}\");",
            path.display(),
            path.display()
        );
        writeln!(out, "    }}");
        writeln!(out);
    }
    writeln!(out, "}}");
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

fn ignore(testsuite: &str, name: &str) -> bool {
    match testsuite {
        "spec_testsuite" => match name {
            // These are the remaining spec testsuite failures.
            "call_indirect" | "data" | "elem" | "exports" | "func" | "func_ptrs" | "globals"
            | "imports" | "linking" | "names" | "start" => true,
            _ => false,
        },
        _ => false,
    }
}
