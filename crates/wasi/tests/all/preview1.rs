use super::*;
use std::path::Path;
use test_programs_artifacts::*;
use wasmtime::{Linker, Module};
use wasmtime_wasi::preview1::add_to_linker_async;

async fn run(path: &str, inherit_stdio: bool) -> Result<()> {
    let path = Path::new(path);
    let name = path.file_stem().unwrap().to_str().unwrap();
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::<Ctx>::new(&engine);
    add_to_linker_async(&mut linker, |t| &mut t.wasi)?;

    let module = Module::from_file(&engine, path)?;
    let (mut store, _td) = store(&engine, name, |builder| {
        if inherit_stdio {
            builder.inherit_stdio();
        }
    })?;
    let instance = linker.instantiate_async(&mut store, &module).await?;
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    start.call_async(&mut store, ()).await?;
    Ok(())
}

foreach_preview1!(assert_test_exists);

// Below here is mechanical: there should be one test for every binary in
// wasi-tests.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_big_random_buf() {
    run(PREVIEW1_BIG_RANDOM_BUF, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_clock_time_get() {
    run(PREVIEW1_CLOCK_TIME_GET, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_close_preopen() {
    run(PREVIEW1_CLOSE_PREOPEN, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_dangling_fd() {
    run(PREVIEW1_DANGLING_FD, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_dangling_symlink() {
    run(PREVIEW1_DANGLING_SYMLINK, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_directory_seek() {
    run(PREVIEW1_DIRECTORY_SEEK, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_dir_fd_op_failures() {
    run(PREVIEW1_DIR_FD_OP_FAILURES, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_fd_advise() {
    run(PREVIEW1_FD_ADVISE, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_fd_filestat_get() {
    run(PREVIEW1_FD_FILESTAT_GET, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_fd_filestat_set() {
    run(PREVIEW1_FD_FILESTAT_SET, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_fd_flags_set() {
    run(PREVIEW1_FD_FLAGS_SET, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_fd_readdir() {
    run(PREVIEW1_FD_READDIR, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_file_allocate() {
    run(PREVIEW1_FILE_ALLOCATE, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_file_pread_pwrite() {
    run(PREVIEW1_FILE_PREAD_PWRITE, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_file_read_write() {
    run(PREVIEW1_FILE_READ_WRITE, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_file_seek_tell() {
    run(PREVIEW1_FILE_SEEK_TELL, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_file_truncation() {
    run(PREVIEW1_FILE_TRUNCATION, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_file_unbuffered_write() {
    run(PREVIEW1_FILE_UNBUFFERED_WRITE, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_interesting_paths() {
    run(PREVIEW1_INTERESTING_PATHS, true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_regular_file_isatty() {
    run(PREVIEW1_REGULAR_FILE_ISATTY, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_nofollow_errors() {
    run(PREVIEW1_NOFOLLOW_ERRORS, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_overwrite_preopen() {
    run(PREVIEW1_OVERWRITE_PREOPEN, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_exists() {
    run(PREVIEW1_PATH_EXISTS, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_filestat() {
    run(PREVIEW1_PATH_FILESTAT, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_link() {
    run(PREVIEW1_PATH_LINK, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_open_create_existing() {
    run(PREVIEW1_PATH_OPEN_CREATE_EXISTING, false)
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_open_read_write() {
    run(PREVIEW1_PATH_OPEN_READ_WRITE, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_open_dirfd_not_dir() {
    run(PREVIEW1_PATH_OPEN_DIRFD_NOT_DIR, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_open_missing() {
    run(PREVIEW1_PATH_OPEN_MISSING, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_open_nonblock() {
    run(PREVIEW1_PATH_OPEN_NONBLOCK, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_rename_dir_trailing_slashes() {
    run(PREVIEW1_PATH_RENAME_DIR_TRAILING_SLASHES, false)
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_rename() {
    run(PREVIEW1_PATH_RENAME, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_symlink_trailing_slashes() {
    run(PREVIEW1_PATH_SYMLINK_TRAILING_SLASHES, false)
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_poll_oneoff_files() {
    run(PREVIEW1_POLL_ONEOFF_FILES, false).await.unwrap()
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_poll_oneoff_stdio() {
    run(PREVIEW1_POLL_ONEOFF_STDIO, true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_readlink() {
    run(PREVIEW1_READLINK, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_remove_directory() {
    run(PREVIEW1_REMOVE_DIRECTORY, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_remove_nonempty_directory() {
    run(PREVIEW1_REMOVE_NONEMPTY_DIRECTORY, false)
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_renumber() {
    run(PREVIEW1_RENUMBER, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_sched_yield() {
    run(PREVIEW1_SCHED_YIELD, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_stdio() {
    run(PREVIEW1_STDIO, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_stdio_isatty() {
    // If the test process is setup such that stdio is a terminal:
    if test_programs_artifacts::stdio_is_terminal() {
        // Inherit stdio, test asserts each is not tty:
        run(PREVIEW1_STDIO_ISATTY, true).await.unwrap()
    }
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_stdio_not_isatty() {
    // Don't inherit stdio, test asserts each is not tty:
    run(PREVIEW1_STDIO_NOT_ISATTY, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_symlink_create() {
    run(PREVIEW1_SYMLINK_CREATE, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_symlink_filestat() {
    run(PREVIEW1_SYMLINK_FILESTAT, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_symlink_loop() {
    run(PREVIEW1_SYMLINK_LOOP, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_unlink_file_trailing_slashes() {
    run(PREVIEW1_UNLINK_FILE_TRAILING_SLASHES, false)
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_open_preopen() {
    run(PREVIEW1_PATH_OPEN_PREOPEN, false).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_unicode_output() {
    run(PREVIEW1_UNICODE_OUTPUT, true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_file_write() {
    run(PREVIEW1_FILE_WRITE, true).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn preview1_path_open_lots() {
    run(PREVIEW1_PATH_OPEN_LOTS, true).await.unwrap()
}
