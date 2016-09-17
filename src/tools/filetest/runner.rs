//! Test runner.
//!
//! This module implements the `TestRunner` struct which manages executing tests as well as
//! scanning directories for tests.

use std::error::Error;
use std::ffi::OsStr;
use std::mem;
use std::panic::catch_unwind;
use std::path::{Path, PathBuf};
use filetest::{TestResult, runone};
use CommandResult;

#[derive(PartialEq, Eq, Debug)]
enum QueueEntry {
    New(PathBuf),
    Running,
    Done(PathBuf, TestResult),
}

pub struct TestRunner {
    // Directories that have not yet been scanned.
    dir_stack: Vec<PathBuf>,

    // Filenames of tests to run.
    tests: Vec<QueueEntry>,

    // Pointer into `tests` where the `New` entries begin.
    new_tests: usize,

    // Number of contiguous finished tests at the front of `tests`.
    finished_tests: usize,

    errors: usize,
}

impl TestRunner {
    /// Create a new blank TrstRunner.
    pub fn new() -> TestRunner {
        TestRunner {
            dir_stack: Vec::new(),
            tests: Vec::new(),
            new_tests: 0,
            finished_tests: 0,
            errors: 0,
        }
    }

    /// Add a directory path to be scanned later.
    ///
    /// If `dir` turns out to be a regular file, it is silently ignored.
    /// Otherwise, any problems reading the directory are reported.
    pub fn push_dir<P: Into<PathBuf>>(&mut self, dir: P) {
        self.dir_stack.push(dir.into());
    }

    /// Add a test to be executed later.
    ///
    /// Any problems reading `file` as a test case file will be reported as a test failure.
    pub fn push_test<P: Into<PathBuf>>(&mut self, file: P) {
        self.tests.push(QueueEntry::New(file.into()));
    }

    /// Take a new test for running as a job.
    /// Leaves the queue entry marked as `Runnning`.
    fn take_job(&mut self) -> Option<Job> {
        let index = self.new_tests;
        if index == self.tests.len() {
            return None;
        }
        self.new_tests += 1;

        let entry = mem::replace(&mut self.tests[index], QueueEntry::Running);
        if let QueueEntry::New(path) = entry {
            Some(Job::new(index, path))
        } else {
            // Oh, sorry about that. Put the entry back.
            self.tests[index] = entry;
            None
        }
    }

    /// Report the end of a job.
    fn finish_job(&mut self, job: Job, result: TestResult) {
        assert_eq!(self.tests[job.index], QueueEntry::Running);
        if let Err(ref e) = result {
            self.job_error(&job.path, e);
        }
        self.tests[job.index] = QueueEntry::Done(job.path, result);
        if job.index == self.finished_tests {
            while let Some(&QueueEntry::Done(_, _)) = self.tests.get(self.finished_tests) {
                self.finished_tests += 1;
            }
        }
    }

    /// Scan any directories pushed so far.
    /// Push any potential test cases found.
    pub fn scan_dirs(&mut self) {
        // This recursive search tries to minimize statting in a directory hierarchy containing
        // mostly test cases.
        //
        // - Directory entries with a "cton" extension are presumed to be test case files.
        // - Directory entries with no extension are presumed to be subdirectories.
        // - Anything else is ignored.
        //
        while let Some(dir) = self.dir_stack.pop() {
            match dir.read_dir() {
                Err(err) => {
                    // Fail silently if `dir` was actually a regular file.
                    // This lets us skip spurious extensionless files without statting everything
                    // needlessly.
                    if !dir.is_file() {
                        self.path_error(dir, err);
                    }
                }
                Ok(entries) => {
                    // Read all directory entries. Avoid statting.
                    for entry_result in entries {
                        match entry_result {
                            Err(err) => {
                                // Not sure why this would happen. `read_dir` succeeds, but there's
                                // a problem with an entry. I/O error during a getdirentries
                                // syscall seems to be the reason. The implementation in
                                // libstd/sys/unix/fs.rs seems to suggest that breaking now would
                                // be a good idea, or the iterator could keep returning the same
                                // error forever.
                                self.path_error(dir, err);
                                break;
                            }
                            Ok(entry) => {
                                let path = entry.path();
                                // Recognize directories and tests by extension.
                                // Yes, this means we ignore directories with '.' in their name.
                                match path.extension().and_then(OsStr::to_str) {
                                    Some("cton") => self.push_test(path),
                                    Some(_) => {}
                                    None => self.push_dir(path),
                                }
                            }
                        }
                    }
                }
            }
            // Get the new jobs running before moving on to the next directory.
            self.schedule_jobs();
        }
    }

    /// Report an error related to a path.
    fn path_error<E: Error>(&mut self, path: PathBuf, err: E) {
        self.errors += 1;
        println!("{}: {}", path.to_string_lossy(), err);
    }

    /// Report an error related to a job.
    fn job_error(&mut self, path: &Path, err: &str) {
        self.errors += 1;
        println!("FAIL {}: {}", path.to_string_lossy(), err);
    }

    /// Schedule and new jobs to run.
    fn schedule_jobs(&mut self) {
        while let Some(job) = self.take_job() {
            let result = job.run();
            self.finish_job(job, result);
        }
    }

    /// Scan pushed directories for tests and run them.
    pub fn run(&mut self) -> CommandResult {
        self.scan_dirs();
        self.schedule_jobs();
        println!("{} tests", self.tests.len());
        match self.errors {
            0 => Ok(()),
            1 => Err("1 failure".to_string()),
            n => Err(format!("{} failures", n)),
        }
    }
}

/// A test file waiting to be run.
struct Job {
    index: usize,
    path: PathBuf,
}

impl Job {
    pub fn new(index: usize, path: PathBuf) -> Job {
        Job {
            index: index,
            path: path,
        }
    }

    pub fn run(&self) -> TestResult {
        match catch_unwind(|| runone::run(self.path.as_path())) {
            Err(msg) => Err(format!("panic: {:?}", msg)),
            Ok(result) => result,
        }
    }
}
