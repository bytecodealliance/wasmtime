use anyhow::Result;

use super::util::test_run;

// No-op function; we only test this by composing it in `async_coop_threads_caller`
#[allow(
    dead_code,
    reason = "here only to make the `assert_test_exists` macro happy"
)]
pub fn async_coop_threads_callee() {}

#[tokio::test]
pub async fn async_coop_threads_caller() -> Result<()> {
    test_run(&[
        test_programs_artifacts::ASYNC_COOP_THREADS_CALLER_COMPONENT,
        test_programs_artifacts::ASYNC_COOP_THREADS_CALLEE_COMPONENT,
    ])
    .await
}
