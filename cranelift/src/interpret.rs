//! CLI tool to interpret Cranelift IR files.

use crate::utils::iterate_files;
use cranelift_interpreter::environment::Environment;
use cranelift_interpreter::interpreter::{ControlFlow, Interpreter};
use cranelift_reader::{parse_run_command, parse_test, ParseError, ParseOptions};
use log::debug;
use std::path::PathBuf;
use std::{fs, io};
use thiserror::Error;

/// Run files through the Cranelift interpreter, interpreting any functions with annotations.
pub fn run(files: Vec<String>, flag_print: bool) -> Result<(), String> {
    let mut total = 0;
    let mut errors = 0;
    for file in iterate_files(files) {
        total += 1;
        let runner = FileInterpreter::from_path(file).map_err(|e| e.to_string())?;
        match runner.run() {
            Ok(_) => {
                if flag_print {
                    println!("{}", runner.path());
                }
            }
            Err(e) => {
                if flag_print {
                    println!("{}: {}", runner.path(), e.to_string());
                }
                errors += 1;
            }
        }
    }

    if flag_print {
        match total {
            0 => println!("0 files"),
            1 => println!("1 file"),
            n => println!("{} files", n),
        }
    }

    match errors {
        0 => Ok(()),
        1 => Err(String::from("1 failure")),
        n => Err(format!("{} failures", n)),
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
        debug!("New file runner from path: {}:", path.to_string_lossy());
        let contents = fs::read_to_string(&path)?;
        Ok(Self {
            path: Some(path),
            contents,
        })
    }

    /// Construct a file runner from a CLIF code string. Currently only used for testing.
    #[cfg(test)]
    pub fn from_inline_code(contents: String) -> Self {
        debug!("New file runner from inline code: {}:", &contents[..20]);
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
        let mut env = Environment::default();
        let mut commands = vec![];
        for (func, details) in test.functions.into_iter() {
            for comment in details.comments {
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
        let interpreter = Interpreter::new(env);
        for command in commands {
            command
                .run(|func_name, args| {
                    // Because we have stored function names with a leading %, we need to re-add it.
                    let func_name = &format!("%{}", func_name);
                    match interpreter.call_by_name(func_name, args) {
                        Ok(ControlFlow::Return(results)) => Ok(results),
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
            function %test() -> b8 {
            block0:
                nop
                v1 = bconst.b8 true
                v2 = iconst.i8 42
                return v1
            }
            ; run: %test() == true
            ",
        );
        FileInterpreter::from_inline_code(code).run().unwrap()
    }

    #[test]
    fn filetests() {
        run(vec!["../filetests/filetests/interpreter".to_string()], true).unwrap()
    }
}
