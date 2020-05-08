//! Test command for `peepmatic`-generated peephole optimizers.

use crate::subtest::{run_filecheck, Context, SubTest, SubtestResult};
use cranelift_codegen;
use cranelift_codegen::ir::Function;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

struct TestPreopt;

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "peepmatic");
    if parsed.options.is_empty() {
        Ok(Box::new(TestPreopt))
    } else {
        Err(format!("No options allowed on {}", parsed))
    }
}

impl SubTest for TestPreopt {
    fn name(&self) -> &'static str {
        "peepmatic"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> SubtestResult<()> {
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());
        let isa = context.isa.expect("preopt needs an ISA");

        comp_ctx.compute_cfg();
        comp_ctx
            .preopt(isa)
            .map_err(|e| pretty_error(&comp_ctx.func, context.isa, Into::into(e)))?;
        let text = &comp_ctx.func.display(isa).to_string();
        log::debug!("After peepmatic-based simple_preopt:\n{}", text);

        // Only actually run the filecheck if peepmatic is enabled, because it
        // can generate slightly different code (alias a result vs replace an
        // instruction) than the non-peepmatic versions of peephole
        // optimizations. Note that the non-`peepmatic` results can be tested
        // with the `test simple_preopt` subtest.
        if cfg!(feature = "enable-peepmatic") {
            run_filecheck(&text, context)
        } else {
            Ok(())
        }
    }
}
