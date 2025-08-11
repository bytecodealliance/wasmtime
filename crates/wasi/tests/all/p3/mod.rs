use crate::store::{Ctx, MyWasiCtx};
use anyhow::{Context as _, anyhow};
use std::path::Path;
use test_programs_artifacts::*;
use wasmtime::Result;
use wasmtime::component::{Component, Linker};
use wasmtime_wasi::p3::bindings::Command;

async fn run(path: &str) -> Result<()> {
    let path = Path::new(path);
    let name = path.file_stem().unwrap().to_str().unwrap();
    let engine = test_programs_artifacts::engine(|config| {
        config.async_support(true);
        config.wasm_component_model_async(true);
    });
    let mut linker = Linker::new(&engine);
    // TODO: Remove once test components are not built for `wasm32-wasip1`
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)
        .context("failed to link `wasi:cli@0.2.x`")?;
    wasmtime_wasi::p3::add_to_linker(&mut linker).context("failed to link `wasi:cli@0.3.x`")?;

    let (mut store, _td) = Ctx::new(&engine, name, |builder| MyWasiCtx {
        wasi: builder.build(),
        table: Default::default(),
    })?;
    let component = Component::from_file(&engine, path)?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let command =
        Command::new(&mut store, &instance).context("failed to instantiate `wasi:cli/command`")?;
    instance
        .run_concurrent(&mut store, async move |store| {
            command.wasi_cli_run().call_run(store).await
        })
        .await
        .context("failed to call `wasi:cli/run#run`")?
        .context("guest trapped")?
        .map_err(|()| anyhow!("`wasi:cli/run#run` failed"))
}

foreach_p3!(assert_test_exists);

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_cli() -> anyhow::Result<()> {
    run(P3_CLI_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_clocks_sleep() -> anyhow::Result<()> {
    run(P3_CLOCKS_SLEEP_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_filesystem_file_read_write() -> anyhow::Result<()> {
    run(P3_FILESYSTEM_FILE_READ_WRITE_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_random_imports() -> anyhow::Result<()> {
    run(P3_RANDOM_IMPORTS_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_sockets_ip_name_lookup() -> anyhow::Result<()> {
    run(P3_SOCKETS_IP_NAME_LOOKUP_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_sockets_tcp_bind() -> anyhow::Result<()> {
    run(P3_SOCKETS_TCP_BIND_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_sockets_tcp_connect() -> anyhow::Result<()> {
    run(P3_SOCKETS_TCP_CONNECT_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_sockets_tcp_sample_application() -> anyhow::Result<()> {
    run(P3_SOCKETS_TCP_SAMPLE_APPLICATION_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_sockets_tcp_sockopts() -> anyhow::Result<()> {
    run(P3_SOCKETS_TCP_SOCKOPTS_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_sockets_tcp_states() -> anyhow::Result<()> {
    run(P3_SOCKETS_TCP_STATES_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_sockets_tcp_streams() -> anyhow::Result<()> {
    run(P3_SOCKETS_TCP_STREAMS_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_sockets_udp_bind() -> anyhow::Result<()> {
    run(P3_SOCKETS_UDP_BIND_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_sockets_udp_connect() -> anyhow::Result<()> {
    run(P3_SOCKETS_UDP_CONNECT_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_sockets_udp_sample_application() -> anyhow::Result<()> {
    run(P3_SOCKETS_UDP_SAMPLE_APPLICATION_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_sockets_udp_sockopts() -> anyhow::Result<()> {
    run(P3_SOCKETS_UDP_SOCKOPTS_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_sockets_udp_states() -> anyhow::Result<()> {
    run(P3_SOCKETS_UDP_STATES_COMPONENT).await
}
