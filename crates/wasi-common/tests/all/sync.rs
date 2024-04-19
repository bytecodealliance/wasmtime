use super::*;
use test_programs_artifacts::*;
use wasi_common::sync::{add_to_linker, WasiCtxBuilder};

foreach_preview1!(assert_test_exists);

fn run(path: &str, inherit_stdio: bool) -> Result<()> {
    let path = Path::new(path);
    let name = path.file_stem().unwrap().to_str().unwrap();
    let workspace = prepare_workspace(name)?;
    let stdout = WritePipe::new_in_memory();
    let stderr = WritePipe::new_in_memory();
    let r = {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        add_to_linker(&mut linker, |cx| cx)?;

        // Create our wasi context.
        // Additionally register any preopened directories if we have them.
        let mut builder = WasiCtxBuilder::new();

        if inherit_stdio {
            builder.inherit_stdio();
        } else {
            builder
                .stdout(Box::new(stdout.clone()))
                .stderr(Box::new(stderr.clone()));
        }
        builder.arg(name)?.arg(".")?;
        println!("preopen: {:?}", workspace);
        let preopen_dir =
            cap_std::fs::Dir::open_ambient_dir(workspace.path(), cap_std::ambient_authority())?;
        builder.preopened_dir(preopen_dir, ".")?;
        for (var, val) in test_programs_artifacts::wasi_tests_environment() {
            builder.env(var, val)?;
        }

        let mut store = Store::new(&engine, builder.build());
        let module = Module::from_file(&engine, path)?;
        let instance = linker.instantiate(&mut store, &module)?;
        let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
        start.call(&mut store, ())?;
        Ok(())
    };

    r.map_err(move |trap: anyhow::Error| {
        let stdout = stdout
            .try_into_inner()
            .expect("sole ref to stdout")
            .into_inner();
        if !stdout.is_empty() {
            println!("guest stdout:\n{}\n===", String::from_utf8_lossy(&stdout));
        }
        let stderr = stderr
            .try_into_inner()
            .expect("sole ref to stderr")
            .into_inner();
        if !stderr.is_empty() {
            println!("guest stderr:\n{}\n===", String::from_utf8_lossy(&stderr));
        }
        trap.context(format!(
            "error while testing wasi-tests {} with cap-std-sync",
            name
        ))
    })?;
    Ok(())
}

// Below here is mechanical: there should be one test for every binary in
// wasi-tests.
#[test_log::test]
fn preview1_big_random_buf() {
    run(PREVIEW1_BIG_RANDOM_BUF, true).unwrap()
}
#[test_log::test]
fn preview1_clock_time_get() {
    run(PREVIEW1_CLOCK_TIME_GET, true).unwrap()
}
#[test_log::test]
fn preview1_close_preopen() {
    run(PREVIEW1_CLOSE_PREOPEN, true).unwrap()
}
#[test_log::test]
fn preview1_dangling_fd() {
    run(PREVIEW1_DANGLING_FD, true).unwrap()
}
#[test_log::test]
fn preview1_dangling_symlink() {
    run(PREVIEW1_DANGLING_SYMLINK, true).unwrap()
}
#[test_log::test]
fn preview1_directory_seek() {
    run(PREVIEW1_DIRECTORY_SEEK, true).unwrap()
}
#[test_log::test]
fn preview1_dir_fd_op_failures() {
    run(PREVIEW1_DIR_FD_OP_FAILURES, true).unwrap()
}
#[test_log::test]
fn preview1_fd_advise() {
    run(PREVIEW1_FD_ADVISE, true).unwrap()
}
#[test_log::test]
fn preview1_fd_filestat_get() {
    run(PREVIEW1_FD_FILESTAT_GET, true).unwrap()
}
#[test_log::test]
fn preview1_fd_filestat_set() {
    run(PREVIEW1_FD_FILESTAT_SET, true).unwrap()
}
#[test_log::test]
fn preview1_fd_flags_set() {
    run(PREVIEW1_FD_FLAGS_SET, true).unwrap()
}
#[test_log::test]
fn preview1_fd_readdir() {
    run(PREVIEW1_FD_READDIR, true).unwrap()
}
#[test_log::test]
fn preview1_file_allocate() {
    run(PREVIEW1_FILE_ALLOCATE, true).unwrap()
}
#[test_log::test]
fn preview1_file_pread_pwrite() {
    run(PREVIEW1_FILE_PREAD_PWRITE, true).unwrap()
}
#[test_log::test]
fn preview1_file_read_write() {
    run(PREVIEW1_FILE_READ_WRITE, true).unwrap()
}
#[test_log::test]
fn preview1_file_seek_tell() {
    run(PREVIEW1_FILE_SEEK_TELL, true).unwrap()
}
#[test_log::test]
fn preview1_file_truncation() {
    run(PREVIEW1_FILE_TRUNCATION, true).unwrap()
}
#[test_log::test]
fn preview1_file_unbuffered_write() {
    run(PREVIEW1_FILE_UNBUFFERED_WRITE, true).unwrap()
}
#[test_log::test]
fn preview1_interesting_paths() {
    run(PREVIEW1_INTERESTING_PATHS, true).unwrap()
}
#[test_log::test]
fn preview1_regular_file_isatty() {
    run(PREVIEW1_REGULAR_FILE_ISATTY, true).unwrap()
}
#[test_log::test]
fn preview1_nofollow_errors() {
    run(PREVIEW1_NOFOLLOW_ERRORS, true).unwrap()
}
#[test_log::test]
fn preview1_overwrite_preopen() {
    run(PREVIEW1_OVERWRITE_PREOPEN, true).unwrap()
}
#[test_log::test]
fn preview1_path_exists() {
    run(PREVIEW1_PATH_EXISTS, true).unwrap()
}
#[test_log::test]
fn preview1_path_filestat() {
    run(PREVIEW1_PATH_FILESTAT, true).unwrap()
}
#[test_log::test]
fn preview1_path_link() {
    run(PREVIEW1_PATH_LINK, true).unwrap()
}
#[test_log::test]
fn preview1_path_open_create_existing() {
    run(PREVIEW1_PATH_OPEN_CREATE_EXISTING, true).unwrap()
}
#[test_log::test]
fn preview1_path_open_read_write() {
    run(PREVIEW1_PATH_OPEN_READ_WRITE, true).unwrap()
}
#[test_log::test]
fn preview1_path_open_dirfd_not_dir() {
    run(PREVIEW1_PATH_OPEN_DIRFD_NOT_DIR, true).unwrap()
}
#[test_log::test]
fn preview1_path_open_missing() {
    run(PREVIEW1_PATH_OPEN_MISSING, true).unwrap()
}
#[test_log::test]
fn preview1_path_open_nonblock() {
    run(PREVIEW1_PATH_OPEN_NONBLOCK, true).unwrap()
}
#[test_log::test]
fn preview1_path_rename_dir_trailing_slashes() {
    run(PREVIEW1_PATH_RENAME_DIR_TRAILING_SLASHES, true).unwrap()
}
#[test_log::test]
fn preview1_path_rename() {
    run(PREVIEW1_PATH_RENAME, true).unwrap()
}
#[test_log::test]
fn preview1_path_symlink_trailing_slashes() {
    run(PREVIEW1_PATH_SYMLINK_TRAILING_SLASHES, true).unwrap()
}
#[test_log::test]
fn preview1_poll_oneoff_files() {
    run(PREVIEW1_POLL_ONEOFF_FILES, false).unwrap()
}
#[test_log::test]
fn preview1_poll_oneoff_stdio() {
    run(PREVIEW1_POLL_ONEOFF_STDIO, true).unwrap()
}
#[test_log::test]
fn preview1_readlink() {
    run(PREVIEW1_READLINK, true).unwrap()
}
#[test_log::test]
fn preview1_remove_directory() {
    run(PREVIEW1_REMOVE_DIRECTORY, true).unwrap()
}
#[test_log::test]
fn preview1_remove_nonempty_directory() {
    run(PREVIEW1_REMOVE_NONEMPTY_DIRECTORY, true).unwrap()
}
#[test_log::test]
fn preview1_renumber() {
    run(PREVIEW1_RENUMBER, true).unwrap()
}
#[test_log::test]
fn preview1_sched_yield() {
    run(PREVIEW1_SCHED_YIELD, true).unwrap()
}
#[test_log::test]
fn preview1_stdio() {
    run(PREVIEW1_STDIO, true).unwrap()
}
#[test_log::test]
fn preview1_stdio_isatty() {
    if test_programs_artifacts::stdio_is_terminal() {
        // Inherit stdio, which is a terminal in the test runner's environment:
        run(PREVIEW1_STDIO_ISATTY, true).unwrap()
    }
}
#[test_log::test]
fn preview1_stdio_not_isatty() {
    // Don't inherit stdio, test asserts each is not tty:
    run(PREVIEW1_STDIO_NOT_ISATTY, false).unwrap()
}

#[test_log::test]
fn preview1_symlink_create() {
    run(PREVIEW1_SYMLINK_CREATE, true).unwrap()
}
#[test_log::test]
fn preview1_symlink_filestat() {
    run(PREVIEW1_SYMLINK_FILESTAT, true).unwrap()
}
#[test_log::test]
fn preview1_symlink_loop() {
    run(PREVIEW1_SYMLINK_LOOP, true).unwrap()
}
#[test_log::test]
fn preview1_unlink_file_trailing_slashes() {
    run(PREVIEW1_UNLINK_FILE_TRAILING_SLASHES, true).unwrap()
}
#[test_log::test]
fn preview1_path_open_preopen() {
    run(PREVIEW1_PATH_OPEN_PREOPEN, true).unwrap()
}
#[test_log::test]
fn preview1_unicode_output() {
    run(PREVIEW1_UNICODE_OUTPUT, true).unwrap()
}
#[test_log::test]
fn preview1_file_write() {
    run(PREVIEW1_FILE_WRITE, true).unwrap()
}
#[test_log::test]
fn preview1_path_open_lots() {
    run(PREVIEW1_PATH_OPEN_LOTS, true).unwrap()
}
