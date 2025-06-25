use anyhow::Result;

use component_async_tests::util::test_run;

// No-op function; we only test this by composing it in `async_post_return_caller`
#[allow(
    dead_code,
    reason = "here only to make the `assert_test_exists` macro happy"
)]
pub fn async_post_return_callee() {}

#[tokio::test]
pub async fn async_post_return_caller() -> Result<()> {
    test_run(&[
        test_programs_artifacts::ASYNC_POST_RETURN_CALLER_COMPONENT,
        test_programs_artifacts::ASYNC_POST_RETURN_CALLEE_COMPONENT,
    ])
    .await
}
