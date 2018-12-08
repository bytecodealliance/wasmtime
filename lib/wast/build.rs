use std::env;
use std::fs::{read_dir, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn main() {
    let out_dir =
        PathBuf::from(env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set"));
    let mut out =
        File::create(out_dir.join("run_wast_files.rs")).expect("error creating run_wast_files.rs");

    test_directory(&mut out, "misc_testsuite").unwrap();
    test_directory(&mut out, "spec_testsuite").unwrap();
}

fn test_directory(out: &mut File, testsuite: &str) -> io::Result<()> {
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
        })
        .collect();

    dir_entries.sort_by_key(|dir| dir.path());

    writeln!(
        out,
        "mod {} {{",
        Path::new(testsuite)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .replace("-", "_")
    )?;
    writeln!(out, "    use super::{{native_isa, WastContext, Path}};")?;
    for dir_entry in dir_entries {
        let path = dir_entry.path();
        let stemstr = path
            .file_stem()
            .expect("file_stem")
            .to_str()
            .expect("to_str");

        writeln!(out, "    #[test]")?;
        if ignore(testsuite, stemstr) {
            writeln!(out, "    #[ignore]")?;
        }
        writeln!(
            out,
            "    fn {}() {{",
            avoid_keywords(&stemstr.replace("-", "_"))
        )?;
        writeln!(out, "        let mut wast_context = WastContext::new().expect(\"error constructing WastContext\");")?;
        writeln!(
            out,
            "        wast_context.run_file(&*native_isa(), Path::new(\"{}\")).expect(\"error running wast file: {}\");",
            path.display(),
            path.display()
        )?;
        writeln!(out, "    }}")?;
        writeln!(out)?;
    }
    writeln!(out, "}}")?;
    Ok(())
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
            "data" | "elem" | "imports" | "linking" => true,
            _ => false,
        },
        _ => false,
    }
}
