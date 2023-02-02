use anyhow::anyhow;
use cranelift_codegen::ir::{ArgumentPurpose, Function};
use cranelift_reader::Comment;

/// Stores info about the expected environment for a test function.
#[derive(Debug, Clone)]
pub struct RuntestEnvironment {}

impl RuntestEnvironment {
    /// Parse the environment from a set of comments
    pub fn parse(comments: &[Comment]) -> anyhow::Result<Self> {
        let mut env = RuntestEnvironment {};
        Ok(env)
    }

    /// Validates the signature of a [Function] ensuring that if this environment is active, the
    /// function has a `vmctx` argument
    pub fn validate_signature(&self, func: &Function) -> Result<(), String> {
        let first_arg_is_vmctx = func
            .signature
            .params
            .first()
            .map(|p| p.purpose == ArgumentPurpose::VMContext)
            .unwrap_or(false);

        if !first_arg_is_vmctx && self.is_active() {
            return Err(concat!(
                "This test requests a heap, but the first argument is not `i64 vmctx`.\n",
                "See docs/testing.md for more info on using heap annotations."
            )
            .to_string());
        }

        Ok(())
    }
}
