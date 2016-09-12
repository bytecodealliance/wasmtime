extern crate cretonne;
extern crate cton_reader;
extern crate glob;
extern crate regex;

use std::env;
use glob::glob;
use regex::Regex;
use std::fs::File;
use std::io::Read;
use self::cton_reader::parse_functions;
use self::cretonne::verifier::Verifier;

/// Compile a function and run verifier tests based on specially formatted
/// comments in the [function's] source.
fn verifier_tests_from_source(function_source: &str) {
    let func_re = Regex::new("^[ \t]*function.*").unwrap();
    let err_re = Regex::new(";[ \t]*Err\\((.*)+\\)").unwrap();

    // Each entry corresponds to an optional regular expression, where
    // the index corresponds to the function offset in our source code.
    // If no error is expected for a given function its entry will be
    // set to None.
    let mut verifier_results = Vec::new();
    for line in function_source.lines() {
        if func_re.is_match(line) {
            match err_re.captures(line) {
                Some(caps) => {
                    verifier_results.push(Some(Regex::new(caps.at(1).unwrap()).unwrap()));
                },
                None => {
                    verifier_results.push(None);
                },
            };
        }
    }

    // Run the verifier against each function and compare the output
    // with the expected result (as determined above).
    for (i, func) in parse_functions(function_source).unwrap().into_iter().enumerate() {
        let result = Verifier::new(&func).run();
        match verifier_results[i] {
            Some(ref re) => {
                assert_eq!(re.is_match(&result.err().unwrap().message), true);
            },
            None => {
                assert_eq!(result, Ok(()));
            }
        }
    }
}

#[test]
fn test_all() {
    let testdir = format!("{}/tests/verifier_testdata/*.cton",
                          env::current_dir().unwrap().display());

    for entry in glob(&testdir).unwrap() {
        let path = entry.unwrap();
        println!("Testing {:?}", path);
        let mut file = File::open(&path).unwrap();
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).unwrap();
        verifier_tests_from_source(&buffer);
    }
}
