use crate::store::Ctx;
use std::path::Path;
use test_programs_artifacts::*;
use wasmtime::Result;
use wasmtime::{Linker, Module};
use wasmtime_wasi::p1::{WasiP1Ctx, add_to_linker_async};
use wasmtime_wasi::{WasiCtxBuilder, WasiView};

async fn run(path: &str, with_builder: impl FnOnce(&mut WasiCtxBuilder)) -> Result<()> {
    let path = Path::new(path);
    let name = path.file_stem().unwrap().to_str().unwrap();
    let engine = test_programs_artifacts::engine(|_config| {});
    let mut linker = Linker::<Ctx<WasiP1Ctx>>::new(&engine);
    add_to_linker_async(&mut linker, |t| &mut t.wasi)?;

    let module = Module::from_file(&engine, path)?;
    let (mut store, _td) = Ctx::new(&engine, name, |builder| {
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
async fn p1_big_random_buf() {
    run(P1_BIG_RANDOM_BUF, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_clock_time_get() {
    run(P1_CLOCK_TIME_GET, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_close_preopen() {
    run(P1_CLOSE_PREOPEN, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_dangling_fd() {
    run(P1_DANGLING_FD, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_dangling_symlink() {
    run(P1_DANGLING_SYMLINK, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_directory_seek() {
    run(P1_DIRECTORY_SEEK, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_dir_fd_op_failures() {
    run(P1_DIR_FD_OP_FAILURES, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_fd_advise() {
    run(P1_FD_ADVISE, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_fd_filestat_get() {
    run(P1_FD_FILESTAT_GET, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_fd_filestat_set() {
    run(P1_FD_FILESTAT_SET, |_| {}).await.unwrap()
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
async fn p1_file_allocate() {
    run(P1_FILE_ALLOCATE, |_| {}).await.unwrap()
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
async fn p1_interesting_paths() {
    run(P1_INTERESTING_PATHS, |b| {
        b.inherit_stdio();
    })
    .await
    .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_regular_file_isatty() {
    run(P1_REGULAR_FILE_ISATTY, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_nofollow_errors() {
    run(P1_NOFOLLOW_ERRORS, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_overwrite_preopen() {
    run(P1_OVERWRITE_PREOPEN, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_exists() {
    run(P1_PATH_EXISTS, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_filestat() {
    run(P1_PATH_FILESTAT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_link() {
    run(P1_PATH_LINK, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_create_existing() {
    run(P1_PATH_OPEN_CREATE_EXISTING, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_read_write() {
    run(P1_PATH_OPEN_READ_WRITE, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_dirfd_not_dir() {
    run(P1_PATH_OPEN_DIRFD_NOT_DIR, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_missing() {
    run(P1_PATH_OPEN_MISSING, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_nonblock() {
    run(P1_PATH_OPEN_NONBLOCK, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_rename_dir_trailing_slashes() {
    run(P1_PATH_RENAME_DIR_TRAILING_SLASHES, |_| {})
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_rename() {
    run(P1_PATH_RENAME, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_symlink_trailing_slashes() {
    run(P1_PATH_SYMLINK_TRAILING_SLASHES, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_poll_oneoff_files() {
    run(P1_POLL_ONEOFF_FILES, |_| {}).await.unwrap()
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_poll_oneoff_stdio() {
    run(P1_POLL_ONEOFF_STDIO, |b| {
        b.inherit_stdio();
    })
    .await
    .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_readlink() {
    run(P1_READLINK, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_remove_directory() {
    run(P1_REMOVE_DIRECTORY, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_remove_nonempty_directory() {
    run(P1_REMOVE_NONEMPTY_DIRECTORY, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_renumber() {
    run(P1_RENUMBER, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_sched_yield() {
    run(P1_SCHED_YIELD, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_stdio() {
    run(P1_STDIO, |_| {}).await.unwrap()
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
async fn p1_symlink_create() {
    run(P1_SYMLINK_CREATE, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_symlink_filestat() {
    run(P1_SYMLINK_FILESTAT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_symlink_loop() {
    run(P1_SYMLINK_LOOP, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_unlink_file_trailing_slashes() {
    run(P1_UNLINK_FILE_TRAILING_SLASHES, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_preopen() {
    run(P1_PATH_OPEN_PREOPEN, |_| {}).await.unwrap()
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
    use std::path::PathBuf;
    use wasmtime_wasi::{DirPerms, FilePerms};

    let prefix = format!("wasi_components_truncation_readonly_ro_");
    let tempdir = tempfile::Builder::new()
        .prefix(&prefix)
        .tempdir()
        .expect("create readonly tempdir");
    const FILENAME: &str = "test.txt";
    const EXPECTED_CONTENTS: &[u8] = b"truncation test file\n";
    let mut file: PathBuf = PathBuf::from(tempdir.path());
    file.push(FILENAME);
    std::fs::write(&file, EXPECTED_CONTENTS).expect("write truncation test file");

    run(P1_FILE_TRUNCATION_READONLY, |b| {
        b.preopened_dir(
            tempdir.path(),
            "readonly",
            DirPerms::READ | DirPerms::MUTATE,
            FilePerms::READ,
        )
        .unwrap();
    })
    .await
    .expect("run p1_file_truncation_readonly guest");

    let contents = std::fs::read(&file).expect("read truncation test file");
    assert_eq!(EXPECTED_CONTENTS, contents);
}
