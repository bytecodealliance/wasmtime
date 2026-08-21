mod cases;
mod compile;
mod parse;
mod report;
mod run;

use anyhow::Result;
use parse::{ParsedWast, parse_wast_file};
use report::Report;
use std::path::Path;

fn parse_cli_args<I, S>(args: I) -> (Option<String>, bool, bool)
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut path: Option<String> = None;
    let mut verbose = false;
    let mut retain_temp_files = false;

    for arg in args {
        match arg.as_ref() {
            "-v" | "--verbose" => verbose = true,
            "--retain-temp-files" => retain_temp_files = true,
            _ if path.is_none() => path = Some(arg.as_ref().to_string()),
            _ => {}
        }
    }

    (path, verbose, retain_temp_files)
}

fn main() {
    let (path, verbose, retain_temp_files) = parse_cli_args(std::env::args().skip(1));

    let path = match path {
        Some(p) => p,
        None => {
            eprintln!("usage: wast-arm-runtest [--verbose] [--retain-temp-files] <file.wast>");
            std::process::exit(1);
        }
    };

    println!("Processing: {}", path);

    let parsed = parse_wast_file(Path::new(&path), verbose).expect("Failed to parse file");

    if verbose {
        println!(
            "Commands: {}, Modules: {}",
            parsed.command_count, parsed.module_count
        );
    }

    let report =
        process_all_cases(&parsed, verbose, retain_temp_files).expect("Failed to process cases");
    report.print_and_exit();
}

fn should_keep_tempdir(stop_on_error: bool) -> bool {
    stop_on_error
}

fn process_all_cases(
    parsed: &ParsedWast,
    verbose: bool,
    retain_temp_files: bool,
) -> Result<Report> {
    let mut report = Report::default();
    let workdir = tempfile::tempdir().expect("Failed to create temp dir");
    let workdir_path = workdir.path().to_path_buf();

    for (idx, case) in parsed.cases.iter().enumerate() {
        if verbose {
            eprintln!(
                "Processing case {}: {} with args {:?}",
                idx, case.export, case.args
            );
        }
    }

    match run::run_cases_batch(&parsed.cases, workdir.path(), verbose) {
        Ok(result) => {
            let failed = result.failed > 0;
            report.add_passed(result.passed);
            report.add_failed(result.failed);
            report.add_skipped(result.skipped);
            if !result.output.trim().is_empty() {
                println!("{}", result.output);
            }
            if failed && retain_temp_files {
                eprintln!("Retaining temp files due to --retain-temp-files flag");
            }
        }
        Err(e) => {
            report.add_failed(1);
            eprintln!("ERROR batch run - {}", e);
            if verbose {
                eprintln!("  Backtrace: {:?}", e.backtrace());
            }
        }
    }

    if should_keep_tempdir(retain_temp_files) {
        let _ = workdir.keep();
        eprintln!(
            "Preserving temp directory for debugging: {}",
            workdir_path.display()
        );
    } else {
        let _ = std::fs::remove_dir_all(&workdir_path);
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::{parse_cli_args, should_keep_tempdir};

    #[test]
    fn parses_cli_flags() {
        let (path, verbose, retain_temp_files) =
            parse_cli_args(["--verbose", "--retain-temp-files", "foo.wast"]);

        assert_eq!(path.as_deref(), Some("foo.wast"));
        assert!(verbose);
        assert!(retain_temp_files);
    }

    #[test]
    fn keeps_tempdir_when_stop_on_error_is_enabled() {
        assert!(should_keep_tempdir(true));
        assert!(!should_keep_tempdir(false));
    }
}
