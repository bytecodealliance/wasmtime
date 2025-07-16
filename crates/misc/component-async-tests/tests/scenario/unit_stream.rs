use anyhow::Result;

use super::util::test_run_with_count;

// No-op function; we only test this by composing it in `async_unit_stream_caller`
#[allow(
    dead_code,
    reason = "here only to make the `assert_test_exists` macro happy"
)]
pub fn async_unit_stream_callee() {}

#[tokio::test]
pub async fn async_unit_stream_caller() -> Result<()> {
    test_run_with_count(
        &[
            test_programs_artifacts::ASYNC_UNIT_STREAM_CALLER_COMPONENT,
            test_programs_artifacts::ASYNC_UNIT_STREAM_CALLEE_COMPONENT,
        ],
        1,
    )
    .await
}
