//! Test command for checking the IR verifier.
//!
//! The `test verifier` test command looks for annotations on instructions like this:
//!
//! ```clif
//!     jump block3 ; error: jump to non-existent block
//! ```
//!
//! This annotation means that the verifier is expected to given an error for the jump instruction
//! containing the substring "jump to non-existent block".

use crate::match_directive::match_directive;
use crate::subtest::{Context, SubTest};
use cranelift_codegen::ir::Function;
use cranelift_codegen::verify_function;
use cranelift_reader::TestCommand;
use std::borrow::{Borrow, Cow};
use std::fmt::Write;

struct TestVerifier;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "verifier");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestVerifier))
}

impl SubTest for TestVerifier {
    fn name(&self) -> &'static str {
        "verifier"
    }

    fn needs_verifier(&self) -> bool {
        // Running the verifier before this test would defeat its purpose.
        false
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> anyhow::Result<()> {
        let func = func.borrow();

        // Scan source annotations for "error:" directives.
        let mut expected = Vec::new();

        for comment in &context.details.comments {
            if let Some(tail) = match_directive(comment.text, "error:") {
                expected.push((comment.entity, tail));
            }
        }

        match verify_function(func, context.flags_or_isa()) {
            Ok(()) if expected.is_empty() => Ok(()),
            Ok(()) => anyhow::bail!("passed, but expected errors: {:?}", expected),

            Err(ref errors) if expected.is_empty() => {
                anyhow::bail!("expected no error, but got:\n{}", errors);
            }

            Err(errors) => {
                let mut errors = errors.0;
                let mut msg = String::new();

                // For each expected error, find a suitable match.
                for expect in expected {
                    let pos = errors
                        .iter()
                        .position(|err| err.location == expect.0 && err.message.contains(expect.1));

                    match pos {
                        None => {
                            writeln!(msg, "  expected error {}: {}", expect.0, expect.1).unwrap();
                        }
                        Some(pos) => {
                            errors.swap_remove(pos);
                        }
                    }
                }

                // Report remaining errors.
                for err in errors {
                    writeln!(msg, "unexpected error {err}").unwrap();
                }

                if msg.is_empty() {
                    Ok(())
                } else {
                    anyhow::bail!("{}", msg);
                }
            }
        }
    }
}
