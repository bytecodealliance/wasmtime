use crate::store::{Ctx, MyWasiCtx};
use std::path::Path;
use test_programs_artifacts::*;
use wasmtime::Result;
use wasmtime::component::{Component, Linker};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::p2::add_to_linker_async;
use wasmtime_wasi::p2::bindings::Command;

async fn run(path: &str, with_builder: impl FnOnce(&mut WasiCtxBuilder)) -> Result<()> {
    let path = Path::new(path);
    let name = path.file_stem().unwrap().to_str().unwrap();
    let engine = test_programs_artifacts::engine(|_config| {});
    let mut linker = Linker::new(&engine);
    add_to_linker_async(&mut linker)?;

    let (mut store, _td) = Ctx::new(&engine, name, |builder| {
        with_builder(builder);
        MyWasiCtx::new(builder.build())
    })?;
    let component = Component::from_file(&engine, path)?;
    let command = Command::instantiate_async(&mut store, &component, &linker).await?;
    command
        .wasi_cli_run()
        .call_run(&mut store)
        .await?
        .map_err(|()| wasmtime::format_err!("run returned a failure"))
}

foreach_p1!(assert_test_exists);
foreach_p2!(assert_test_exists);

// Below here is mechanical: there should be one test for every binary in
// wasi-tests.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_big_random_buf() {
    run(P1_BIG_RANDOM_BUF_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_clock_time_get() {
    run(P1_CLOCK_TIME_GET_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_close_preopen() {
    run(P1_CLOSE_PREOPEN_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_dangling_fd() {
    run(P1_DANGLING_FD_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_dangling_symlink() {
    run(P1_DANGLING_SYMLINK_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_directory_seek() {
    run(P1_DIRECTORY_SEEK_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_dir_fd_op_failures() {
    run(P1_DIR_FD_OP_FAILURES_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_fd_advise() {
    run(P1_FD_ADVISE_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_fd_filestat_get() {
    run(P1_FD_FILESTAT_GET_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_fd_filestat_set() {
    run(P1_FD_FILESTAT_SET_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_fd_flags_set() {
    run(P1_FD_FLAGS_SET_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_fd_readdir() {
    run(P1_FD_READDIR_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_allocate() {
    run(P1_FILE_ALLOCATE_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_pread_pwrite() {
    run(P1_FILE_PREAD_PWRITE_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_read_write() {
    run(P1_FILE_READ_WRITE_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_seek_tell() {
    run(P1_FILE_SEEK_TELL_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_truncation() {
    run(P1_FILE_TRUNCATION_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_unbuffered_write() {
    run(P1_FILE_UNBUFFERED_WRITE_COMPONENT, |_| {})
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_interesting_paths() {
    run(P1_INTERESTING_PATHS_COMPONENT, |b| {
        b.inherit_stdio();
    })
    .await
    .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_regular_file_isatty() {
    run(P1_REGULAR_FILE_ISATTY_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_nofollow_errors() {
    run(P1_NOFOLLOW_ERRORS_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_overwrite_preopen() {
    run(P1_OVERWRITE_PREOPEN_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_exists() {
    run(P1_PATH_EXISTS_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_filestat() {
    run(P1_PATH_FILESTAT_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_link() {
    run(P1_PATH_LINK_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_create_existing() {
    run(P1_PATH_OPEN_CREATE_EXISTING_COMPONENT, |_| {})
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_read_write() {
    run(P1_PATH_OPEN_READ_WRITE_COMPONENT, |_| {})
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_dirfd_not_dir() {
    run(P1_PATH_OPEN_DIRFD_NOT_DIR_COMPONENT, |_| {})
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_missing() {
    run(P1_PATH_OPEN_MISSING_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_nonblock() {
    run(P1_PATH_OPEN_NONBLOCK_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_rename_dir_trailing_slashes() {
    run(P1_PATH_RENAME_DIR_TRAILING_SLASHES_COMPONENT, |_| {})
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_rename() {
    run(P1_PATH_RENAME_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_symlink_trailing_slashes() {
    run(P1_PATH_SYMLINK_TRAILING_SLASHES_COMPONENT, |_| {})
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_poll_oneoff_files() {
    run(P1_POLL_ONEOFF_FILES_COMPONENT, |_| {}).await.unwrap()
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_poll_oneoff_stdio() {
    run(P1_POLL_ONEOFF_STDIO_COMPONENT, |b| {
        b.inherit_stdio();
    })
    .await
    .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_readlink() {
    run(P1_READLINK_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_remove_directory() {
    run(P1_REMOVE_DIRECTORY_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_remove_nonempty_directory() {
    run(P1_REMOVE_NONEMPTY_DIRECTORY_COMPONENT, |_| {})
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_renumber() {
    run(P1_RENUMBER_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_sched_yield() {
    run(P1_SCHED_YIELD_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_stdio() {
    run(P1_STDIO_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_stdio_isatty() {
    // If the test process is setup such that stdio is a terminal:
    if test_programs_artifacts::stdio_is_terminal() {
        // Inherit stdio, test asserts each is not tty:
        run(P1_STDIO_ISATTY_COMPONENT, |b| {
            b.inherit_stdio();
        })
        .await
        .unwrap()
    }
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_stdio_not_isatty() {
    // Don't inherit stdio, test asserts each is not tty:
    run(P1_STDIO_NOT_ISATTY_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_symlink_create() {
    run(P1_SYMLINK_CREATE_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_symlink_filestat() {
    run(P1_SYMLINK_FILESTAT_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_symlink_loop() {
    run(P1_SYMLINK_LOOP_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_unlink_file_trailing_slashes() {
    run(P1_UNLINK_FILE_TRAILING_SLASHES_COMPONENT, |_| {})
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_preopen() {
    run(P1_PATH_OPEN_PREOPEN_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_unicode_output() {
    run(P1_UNICODE_OUTPUT_COMPONENT, |b| {
        b.inherit_stdio();
    })
    .await
    .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_write() {
    run(P1_FILE_WRITE_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_path_open_lots() {
    run(P1_PATH_OPEN_LOTS_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_sleep_quickly_but_lots() {
    run(P1_SLEEP_QUICKLY_BUT_LOTS_COMPONENT, |_| {})
        .await
        .unwrap()
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_sleep() {
    run(P2_SLEEP_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_random() {
    run(P2_RANDOM_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_ip_name_lookup() {
    run(P2_IP_NAME_LOOKUP_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_tcp_sockopts() {
    run(P2_TCP_SOCKOPTS_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_tcp_sample_application() {
    run(P2_TCP_SAMPLE_APPLICATION_COMPONENT, |_| {})
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_tcp_states() {
    run(P2_TCP_STATES_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_tcp_streams() {
    run(P2_TCP_STREAMS_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_tcp_bind() {
    run(P2_TCP_BIND_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_tcp_connect() {
    run(P2_TCP_CONNECT_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_tcp_listen() {
    run(P2_TCP_LISTEN_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_tcp_busy_poll() {
    run(P2_TCP_BUSY_POLL_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_udp_sockopts() {
    run(P2_UDP_SOCKOPTS_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_udp_sample_application() {
    run(P2_UDP_SAMPLE_APPLICATION_COMPONENT, |_| {})
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_udp_states() {
    run(P2_UDP_STATES_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_udp_bind() {
    run(P2_UDP_BIND_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_udp_connect() {
    run(P2_UDP_CONNECT_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_stream_pollable_correct() {
    run(P2_STREAM_POLLABLE_CORRECT_COMPONENT, |_| {})
        .await
        .unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_stream_pollable_traps() {
    let e = run(P2_STREAM_POLLABLE_TRAPS_COMPONENT, |_| {})
        .await
        .unwrap_err();
    assert_eq!(
        format!("{}", e.source().expect("trap source")),
        "resource has children"
    )
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_pollable_correct() {
    run(P2_POLLABLE_CORRECT_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_pollable_traps() {
    let e = run(P2_POLLABLE_TRAPS_COMPONENT, |_| {}).await.unwrap_err();
    assert_eq!(
        format!("{}", e.source().expect("trap source")),
        "empty poll list"
    )
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_adapter_badfd() {
    run(P2_ADAPTER_BADFD_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_file_read_write() {
    run(P2_FILE_READ_WRITE_COMPONENT, |_| {}).await.unwrap()
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_udp_send_too_much() {
    let e = run(P2_UDP_SEND_TOO_MUCH_COMPONENT, |_| {})
        .await
        .unwrap_err();
    assert_eq!(
        format!("{}", e.source().expect("trap source")),
        "unpermitted: argument exceeds permitted size"
    )
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p1_file_truncation_readonly() {
    file_truncation_readonly(P1_FILE_TRUNCATION_READONLY_COMPONENT).await
}
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_file_truncation_readonly() {
    file_truncation_readonly(P2_FILE_TRUNCATION_READONLY_COMPONENT).await
}

async fn file_truncation_readonly(component_path: &str) {
    use std::path::PathBuf;
    use wasmtime_wasi::{DirPerms, FilePerms};

    let prefix = "wasi_components_truncation_readonly_ro_";
    let tempdir = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
        .expect("create readonly tempdir");
    const FILENAME: &str = "test.txt";
    const EXPECTED_CONTENTS: &[u8] = b"truncation test file\n";
    let mut file: PathBuf = PathBuf::from(tempdir.path());
    file.push(FILENAME);
    std::fs::write(&file, EXPECTED_CONTENTS).expect("write truncation test file");

    run(component_path, |b| {
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

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p2_clocks_zero_wait() {
    run(P2_CLOCKS_ZERO_WAIT_COMPONENT, |_| {}).await.unwrap()
}
