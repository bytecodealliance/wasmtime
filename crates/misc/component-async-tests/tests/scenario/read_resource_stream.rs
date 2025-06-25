use anyhow::Result;

use component_async_tests::util::test_run;

#[tokio::test]
pub async fn async_read_resource_stream() -> Result<()> {
    test_run(&[test_programs_artifacts::ASYNC_READ_RESOURCE_STREAM_COMPONENT]).await
}
