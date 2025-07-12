use anyhow::Result;

use super::util::test_run;

// No-op function; we only test this by composing it in
// `async_yield_callee_synchronous` and `async_yield_callee_stackful`
#[allow(
    dead_code,
    reason = "here only to make the `assert_test_exists` macro happy"
)]
pub fn async_yield_caller() {}

#[tokio::test]
pub async fn async_yield_callee_synchronous() -> Result<()> {
    test_run(&[
        test_programs_artifacts::ASYNC_YIELD_CALLER_COMPONENT,
        test_programs_artifacts::ASYNC_YIELD_CALLEE_SYNCHRONOUS_COMPONENT,
    ])
    .await
}

#[tokio::test]
pub async fn async_yield_callee_stackless() -> Result<()> {
    test_run(&[
        test_programs_artifacts::ASYNC_YIELD_CALLER_COMPONENT,
        test_programs_artifacts::ASYNC_YIELD_CALLEE_STACKLESS_COMPONENT,
    ])
    .await
}
