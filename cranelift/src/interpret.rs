//! CLI tool to interpret Cranelift IR files.

use crate::utils::iterate_files;
use clap::Parser;
use cranelift_interpreter::environment::FunctionStore;
use cranelift_interpreter::interpreter::{Interpreter, InterpreterState};
use cranelift_interpreter::step::ControlFlow;
use cranelift_reader::{ParseError, ParseOptions, parse_run_command, parse_test};
use std::path::PathBuf;
use std::{fs, io};
use thiserror::Error;

/// Interpret clif code
#[derive(Parser)]
pub struct Options {
    /// Specify an input file to be used. Use '-' for stdin.
    #[arg(required = true)]
    files: Vec<PathBuf>,

    /// Be more verbose
    #[arg(short, long)]
    verbose: bool,
}

/// Run files through the Cranelift interpreter, interpreting any functions with annotations.
pub fn run(options: &Options) -> anyhow::Result<()> {
    let mut total = 0;
    let mut errors = 0;
    for file in iterate_files(&options.files) {
        total += 1;
        let runner = FileInterpreter::from_path(file)?;
        match runner.run() {
            Ok(_) => {
                if options.verbose {
                    println!("{}", runner.path());
                }
            }
            Err(e) => {
                if options.verbose {
                    println!("{}: {}", runner.path(), e.to_string());
                }
                errors += 1;
            }
        }
    }

    if options.verbose {
        match total {
            0 => println!("0 files"),
            1 => println!("1 file"),
            n => println!("{n} files"),
        }
    }

    match errors {
        0 => Ok(()),
        1 => anyhow::bail!("1 failure"),
        n => anyhow::bail!("{} failures", n),
    }
}

/// Contains CLIF code that can be executed with [FileInterpreter::run].
pub struct FileInterpreter {
    path: Option<PathBuf>,
    contents: String,
}

impl FileInterpreter {
    /// Construct a file runner from a CLIF file path.
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, io::Error> {
        let path = path.into();
        log::trace!("New file runner from path: {}:", path.to_string_lossy());
        let contents = fs::read_to_string(&path)?;
        Ok(Self {
            path: Some(path),
            contents,
        })
    }

    /// Construct a file runner from a CLIF code string. Currently only used for testing.
    #[cfg(test)]
    pub fn from_inline_code(contents: String) -> Self {
        log::trace!("New file runner from inline code: {}:", &contents[..20]);
        Self {
            path: None,
            contents,
        }
    }

    /// Return the path of the file runner or `[inline code]`.
    pub fn path(&self) -> String {
        match self.path {
            None => "[inline code]".to_string(),
            Some(ref p) => p.to_string_lossy().to_string(),
        }
    }

    /// Run the file; this searches for annotations like `; run: %fn0(42)` or
    /// `; test: %fn0(42) == 2` and executes them, performing any test comparisons if necessary.
    pub fn run(&self) -> Result<(), FileInterpreterFailure> {
        // parse file
        let test = parse_test(&self.contents, ParseOptions::default())
            .map_err(|e| FileInterpreterFailure::ParsingClif(self.path(), e))?;

        // collect functions
        let mut env = FunctionStore::default();
        let mut commands = vec![];
        for (func, details) in test.functions.iter() {
            for comment in &details.comments {
                if let Some(command) = parse_run_command(comment.text, &func.signature)
                    .map_err(|e| FileInterpreterFailure::ParsingClif(self.path(), e))?
                {
                    commands.push(command);
                }
            }
            // Note: func.name may truncate the function name
            env.add(func.name.to_string(), func);
        }

        // Run assertion commands
        for command in commands {
            command
                .run(|func_name, args| {
                    // Because we have stored function names with a leading %, we need to re-add it.
                    let func_name = &format!("%{func_name}");
                    let state = InterpreterState::default().with_function_store(env.clone());
                    match Interpreter::new(state).call_by_name(func_name, args) {
                        Ok(ControlFlow::Return(results)) => Ok(results.to_vec()),
                        Ok(_) => panic!("Unexpected returned control flow--this is likely a bug."),
                        Err(t) => Err(t.to_string()),
                    }
                })
                .map_err(|s| FileInterpreterFailure::FailedExecution(s))?;
        }

        Ok(())
    }
}

/// Possible sources of failure in this file.
#[derive(Error, Debug)]
pub enum FileInterpreterFailure {
    #[error("failure reading file")]
    Io(#[from] io::Error),
    #[error("failure parsing file {0}: {1}")]
    ParsingClif(String, ParseError),
    #[error("failed to run function: {0}")]
    FailedExecution(String),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nop() {
        let code = String::from(
            "
            function %test() -> i8 {
            block0:
                nop
                v1 = iconst.i8 -1
                v2 = iconst.i8 42
                return v1
            }
            ; run: %test() == -1
            ",
        );
        FileInterpreter::from_inline_code(code).run().unwrap()
    }

    #[test]
    fn filetests() {
        run(&Options {
            files: vec![PathBuf::from("../filetests/filetests/interpreter")],
            verbose: true,
        })
        .unwrap()
    }
}
