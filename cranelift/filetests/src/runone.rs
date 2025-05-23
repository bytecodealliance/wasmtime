//! Run the tests in a single test file.

use crate::new_subtest;
use crate::subtest::SubTest;
use anyhow::{Context as _, Result, bail};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::print_errors::pretty_verifier_error;
use cranelift_codegen::settings::{Flags, FlagsOrIsa};
use cranelift_codegen::timing;
use cranelift_codegen::verify_function;
use cranelift_reader::{IsaSpec, Location, ParseOptions, TestFile, parse_test};
use log::info;
use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::Lines;
use std::time;

/// Load `path` and run the test in it.
///
/// If running this test causes a panic, it will propagate as normal.
pub fn run(
    path: &Path,
    passes: Option<&[String]>,
    target: Option<&str>,
) -> anyhow::Result<time::Duration> {
    let _tt = timing::process_file();
    info!("---\nFile: {}", path.to_string_lossy());
    let started = time::Instant::now();
    let buffer =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;

    let options = ParseOptions {
        target,
        passes,
        machine_code_cfg_info: true,
        ..ParseOptions::default()
    };

    let testfile = match parse_test(&buffer, options) {
        Ok(testfile) => testfile,
        Err(e) => {
            if e.is_warning {
                log::warn!(
                    "skipping test {:?} (line {}): {}",
                    path,
                    e.location.line_number,
                    e.message
                );
                return Ok(started.elapsed());
            }
            return Err(e)
                .context(format!("failed to parse {}", path.display()))
                .into();
        }
    };

    if testfile.functions.is_empty() {
        anyhow::bail!("no functions found");
    }

    // Parse the test commands.
    let mut tests = testfile
        .commands
        .iter()
        .map(new_subtest)
        .collect::<anyhow::Result<Vec<_>>>()?;

    // Flags to use for those tests that don't need an ISA.
    // This is the cumulative effect of all the `set` commands in the file.
    let flags = match testfile.isa_spec {
        IsaSpec::None(ref f) => f,
        IsaSpec::Some(ref v) => v.last().expect("Empty ISA list").flags(),
    };

    // Sort the tests so the mutators are at the end, and those that don't need the verifier are at
    // the front.
    tests.sort_by_key(|st| (st.is_mutating(), st.needs_verifier()));

    // Expand the tests into (test, flags, isa) tuples.
    let tuples = test_tuples(&tests, &testfile.isa_spec, flags)?;

    // Bail if the test has no runnable commands
    if tuples.is_empty() {
        anyhow::bail!("no test commands found");
    }

    let mut file_update = FileUpdate::new(&path);
    let file_path = path.to_string_lossy();
    for (test, flags, isa) in &tuples {
        // Should we run the verifier before this test?
        if test.needs_verifier() {
            let fisa = FlagsOrIsa { flags, isa: *isa };
            verify_testfile(&testfile, fisa)?;
        }

        test.run_target(&testfile, &mut file_update, file_path.as_ref(), flags, *isa)?;
    }

    Ok(started.elapsed())
}

// Verifies all functions in a testfile
fn verify_testfile(testfile: &TestFile, fisa: FlagsOrIsa) -> anyhow::Result<()> {
    for (func, _) in &testfile.functions {
        verify_function(func, fisa)
            .map_err(|errors| anyhow::anyhow!("{}", pretty_verifier_error(&func, None, errors)))?;
    }

    Ok(())
}

// Given a slice of tests, generate a vector of (test, flags, isa) tuples.
fn test_tuples<'a>(
    tests: &'a [Box<dyn SubTest>],
    isa_spec: &'a IsaSpec,
    no_isa_flags: &'a Flags,
) -> anyhow::Result<Vec<(&'a dyn SubTest, &'a Flags, Option<&'a dyn TargetIsa>)>> {
    let mut out = Vec::new();
    for test in tests {
        if test.needs_isa() {
            match *isa_spec {
                IsaSpec::None(_) => {
                    // TODO: Generate a list of default ISAs.
                    anyhow::bail!("test {} requires an ISA", test.name());
                }
                IsaSpec::Some(ref isas) => {
                    for isa in isas {
                        out.push((&**test, isa.flags(), Some(&**isa)));
                    }
                }
            }
        } else {
            // This test doesn't require an ISA, and we only want to run one instance of it.
            // Still, give it an ISA ref if we happen to have a unique one.
            // For example, `test cat` can use this to print encodings and register names.
            out.push((&**test, no_isa_flags, isa_spec.unique_isa()));
        }
    }
    Ok(out)
}

/// A helper struct to update a file in-place as test expectations are
/// automatically updated.
///
/// This structure automatically handles multiple edits to one file. Our edits
/// are line-based but if editing a previous portion of the file adds lines then
/// all future edits need to know to skip over those previous lines. Note that
/// this assumes that edits are done front-to-back.
pub struct FileUpdate {
    path: PathBuf,
    line_diff: Cell<isize>,
    last_update: Cell<usize>,
}

impl FileUpdate {
    fn new(path: &Path) -> FileUpdate {
        FileUpdate {
            path: path.to_path_buf(),
            line_diff: Cell::new(0),
            last_update: Cell::new(0),
        }
    }

    /// Updates the file that this structure references at the `location`
    /// specified.
    ///
    /// The closure `f` is given first a buffer to push the new test into along
    /// with a lines iterator for the old test.
    pub fn update_at(
        &self,
        location: &Location,
        f: impl FnOnce(&mut String, &mut Lines<'_>),
    ) -> Result<()> {
        // This is required for correctness of this update.
        assert!(location.line_number > self.last_update.get());
        self.last_update.set(location.line_number);

        // Read the old test file and calculate the new line number we're
        // preserving up to based on how many lines prior to this have been
        // removed or added.
        let old_test = std::fs::read_to_string(&self.path)?;
        let mut new_test = String::new();
        let mut lines = old_test.lines();
        let lines_to_preserve =
            (((location.line_number - 1) as isize) + self.line_diff.get()) as usize;

        // Push everything leading up to the start of the function
        for _ in 0..lines_to_preserve {
            new_test.push_str(lines.next().unwrap());
            new_test.push_str("\n");
        }

        // Push the whole function, leading up to the trailing `}`
        let mut first = true;
        while let Some(line) = lines.next() {
            if first && !line.starts_with("function") {
                bail!(
                    "line {} in test file {:?} did not start with `function`, \
                     cannot automatically update test",
                    location.line_number,
                    self.path,
                );
            }
            first = false;
            new_test.push_str(line);
            new_test.push_str("\n");
            if line.starts_with("}") {
                break;
            }
        }

        // Use our custom update function to further update the test.
        f(&mut new_test, &mut lines);

        // Record the difference in line count so future updates can be adjusted
        // accordingly, and then write the file back out to the filesystem.
        let old_line_count = old_test.lines().count();
        let new_line_count = new_test.lines().count();
        self.line_diff
            .set(self.line_diff.get() + (new_line_count as isize - old_line_count as isize));

        std::fs::write(&self.path, new_test)?;
        Ok(())
    }
}
