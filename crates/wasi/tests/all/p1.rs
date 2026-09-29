use crate::store::Ctx;
use std::path::Path;
use test_programs_artifacts::*;
use wasmtime::Result;
use wasmtime::{Linker, Module};
use wasmtime_wasi::p1::{WasiP1Ctx, add_to_linker_async};
use wasmtime_wasi::{WasiCtxBuilder, WasiView};

async fn run(path: &str, with_builder: impl FnOnce(&mut WasiCtxBuilder)) -> Result<()> {
    run_with_workspace_setup(path, |_| Ok(()), with_builder).await
}

async fn run_with_workspace_setup(
    path: &str,
    setup: impl FnOnce(&Path) -> Result<()>,
    with_builder: impl FnOnce(&mut WasiCtxBuilder),
) -> Result<()> {
    let path = Path::new(path);
    let name = path.file_stem().unwrap().to_str().unwrap();
    let engine = test_programs_artifacts::engine(|_config| {});
    let mut linker = Linker::<Ctx<WasiP1Ctx>>::new(&engine);
    add_to_linker_async(&mut linker, |t| &mut t.wasi)?;

    let module = Module::from_file(&engine, path)?;
    let (mut store, _td) = Ctx::new_with_workspace_setup(&engine, name, setup, |builder| {
        with_builder(builder);
        builder.build_p1()
    })?;
    store.data_mut().wasi.ctx().table.set_max_capacity(1000);
    let instance = linker.instantiate_async(&mut store, &module).await?;
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    start.call_async(&mut store, ()).await?;
    Ok(())
}

foreach_p1!(assert_test_exists);

// Below here is mechanical: there should be one test for every binary in
// wasi-tests.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_fd_filestat_get() {
    run(P1_FD_FILESTAT_GET, |_| {}).await.unwrap()
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_stat_extreme_host_mtime() {
    run_with_workspace_setup(
        P1_STAT_EXTREME_HOST_MTIME,
        crate::store::prepare_extreme_mtime_fixture,
        |_| {},
    )
    .await
    .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_fd_flags_set() {
    run(P1_FD_FLAGS_SET, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_fd_readdir() {
    run(P1_FD_READDIR, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_pread_pwrite() {
    run(P1_FILE_PREAD_PWRITE, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_read_write() {
    run(P1_FILE_READ_WRITE, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_seek_tell() {
    run(P1_FILE_SEEK_TELL, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_truncation() {
    run(P1_FILE_TRUNCATION, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_unbuffered_write() {
    run(P1_FILE_UNBUFFERED_WRITE, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_poll_oneoff_files() {
    run(P1_POLL_ONEOFF_FILES, |_| {}).await.unwrap()
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_readlink() {
    run(P1_READLINK, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_renumber() {
    run(P1_RENUMBER, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_stdio_isatty() {
    // If the test process is setup such that stdio is a terminal:
    if test_programs_artifacts::stdio_is_terminal() {
        // Inherit stdio, test asserts each is not tty:
        run(P1_STDIO_ISATTY, |b| {
            b.inherit_stdio();
        })
        .await
        .unwrap()
    }
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_stdio_not_isatty() {
    // Don't inherit stdio, test asserts each is not tty:
    run(P1_STDIO_NOT_ISATTY, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_unicode_output() {
    run(P1_UNICODE_OUTPUT, |b| {
        b.inherit_stdio();
    })
    .await
    .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_write() {
    run(P1_FILE_WRITE, |b| {
        b.inherit_stdio();
    })
    .await
    .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_lots() {
    run(P1_PATH_OPEN_LOTS, |b| {
        b.inherit_stdio();
    })
    .await
    .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_sleep_quickly_but_lots() {
    run(P1_SLEEP_QUICKLY_BUT_LOTS, |b| {
        b.inherit_stdio();
    })
    .await
    .unwrap()
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_truncation_readonly() {
    run_with_readonly_testfile(P1_FILE_TRUNCATION_READONLY).await
}

async fn run_with_readonly_testfile(component_path: &str) {
    use std::path::PathBuf;
    use wasmtime_wasi::FsPerms;

    let prefix = format!("wasi_components_ro_");
    let tempdir = tempfile::Builder::new()
        .prefix(&prefix)
        .tempdir()
        .expect("create readonly tempdir");
    const FILENAME: &str = "test.txt";
    const EXPECTED_CONTENTS: &[u8] = b"read only test file\n";
    let mut file: PathBuf = PathBuf::from(tempdir.path());
    file.push(FILENAME);
    std::fs::write(&file, EXPECTED_CONTENTS).expect("write truncation test file");

    run(component_path, |b| {
        b.preopened_dir(tempdir.path(), "readonly", FsPerms::ReadOnly)
            .unwrap();
    })
    .await
    .expect("run guest");

    let contents = std::fs::read(&file).expect("read truncation test file");
    assert_eq!(EXPECTED_CONTENTS, contents);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_hardlink_across_perms() {
    run_with_readonly_testfile(P1_FILE_HARDLINK_ACROSS_PERMS).await
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_rename_across_perms() {
    run_with_readonly_testfile(P1_FILE_RENAME_ACROSS_PERMS).await
}
