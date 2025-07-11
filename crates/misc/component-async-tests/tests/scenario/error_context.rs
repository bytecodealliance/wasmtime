use anyhow::Result;

use super::util::test_run;

#[tokio::test]
pub async fn async_error_context() -> Result<()> {
    test_run(&[test_programs_artifacts::ASYNC_ERROR_CONTEXT_COMPONENT]).await
}

// No-op function; we only test this by composing it in `async_error_context_caller`
#[allow(
    dead_code,
    reason = "here only to make the `assert_test_exists` macro happy"
)]
pub fn async_error_context_callee() {}

#[tokio::test]
pub async fn async_error_context_caller() -> Result<()> {
    test_run(&[
        test_programs_artifacts::ASYNC_ERROR_CONTEXT_CALLER_COMPONENT,
        test_programs_artifacts::ASYNC_ERROR_CONTEXT_CALLEE_COMPONENT,
    ])
    .await
}
