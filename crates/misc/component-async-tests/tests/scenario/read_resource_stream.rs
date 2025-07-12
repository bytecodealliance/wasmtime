use super::util::test_run;
use anyhow::Result;

#[tokio::test]
pub async fn async_read_resource_stream() -> Result<()> {
    test_run(&[test_programs_artifacts::ASYNC_READ_RESOURCE_STREAM_COMPONENT]).await
}
