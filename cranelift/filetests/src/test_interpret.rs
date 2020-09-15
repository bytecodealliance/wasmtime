//! Test command for interpreting CLIF files and verifying their results
//!
//! The `interpret` test command interprets each function on the host machine
//! using [RunCommand](cranelift_reader::RunCommand)s.

use crate::subtest::{Context, SubTest};
use cranelift_codegen::{self, ir};
use cranelift_interpreter::environment::Environment;
use cranelift_interpreter::interpreter::{ControlFlow, Interpreter};
use cranelift_reader::{parse_run_command, TestCommand};
use log::trace;
use std::borrow::Cow;

struct TestInterpret;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "interpret");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestInterpret))
}

impl SubTest for TestInterpret {
    fn name(&self) -> &'static str {
        "interpret"
    }

    fn is_mutating(&self) -> bool {
        false
    }

    fn needs_isa(&self) -> bool {
        false
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> anyhow::Result<()> {
        for comment in context.details.comments.iter() {
            if let Some(command) = parse_run_command(comment.text, &func.signature)? {
                trace!("Parsed run command: {}", command);

                let mut env = Environment::default();
                env.add(func.name.to_string(), func.clone().into_owned());
                let interpreter = Interpreter::new(env);

                command
                    .run(|func_name, args| {
                        // Because we have stored function names with a leading %, we need to re-add it.
                        let func_name = &format!("%{}", func_name);
                        match interpreter.call_by_name(func_name, args) {
                            Ok(ControlFlow::Return(results)) => Ok(results),
                            Ok(_) => {
                                panic!("Unexpected returned control flow--this is likely a bug.")
                            }
                            Err(t) => Err(format!("unexpected trap: {:?}", t)),
                        }
                    })
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
            }
        }
        Ok(())
    }
}
