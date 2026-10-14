use std::process;
use test_programs::preview1::{config, create_file, open_scratch_directory};

unsafe fn assert_not_directory(dir_fd: wasip1::Fd, old_path: &str, new_path: &str) {
    let err = match unsafe { wasip1::path_rename(dir_fd, old_path, dir_fd, new_path) } {
        Ok(()) => panic!("renaming {old_path:?} to {new_path:?} should fail"),
        Err(err) => err,
    };
    assert_eq!(
        err,
        wasip1::ERRNO_NOTDIR,
        "unexpected error renaming {old_path:?} to {new_path:?}"
    );
}

unsafe fn test_path_rename_trailing_slashes(dir_fd: wasip1::Fd) {
    unsafe {
        // A symlink to a directory outside of the preopen with a trailing
        // slash on either path isn't followed, so the directory isn't moved
        // into the preopen. The host checks that it's still in place. Windows
        // can't create such a symlink in the first place.
        match wasip1::path_symlink("../outside", dir_fd, "up") {
            Ok(()) => {
                assert_not_directory(dir_fd, "up/", "stolen");
                assert_not_directory(dir_fd, "up", "stolen/");
                wasip1::path_unlink_file(dir_fd, "up").expect("removing a symlink");
            }
            Err(wasip1::ERRNO_PERM) if config().errno_expect_windows() => {}
            Err(err) => panic!("creating a symlink: {err:?}"),
        }

        // A symlink to a directory inside of the preopen with a trailing slash
        // on either path isn't followed either.
        wasip1::path_create_directory(dir_fd, "dir").expect("creating a directory");
        wasip1::path_symlink("dir", dir_fd, "link").expect("creating a symlink");
        assert_not_directory(dir_fd, "link/", "target");
        assert_not_directory(dir_fd, "link", "target/");
        wasip1::path_filestat_get(dir_fd, 0, "dir").expect("the directory wasn't renamed");
        wasip1::path_filestat_get(dir_fd, 0, "link").expect("the symlink wasn't renamed");
        wasip1::path_unlink_file(dir_fd, "link").expect("removing a symlink");
        wasip1::path_rename(dir_fd, "dir/", dir_fd, "renamed/").expect("renaming a directory");
        wasip1::path_remove_directory(dir_fd, "renamed").expect("removing a directory");

        // A file with a trailing slash on either path isn't a directory.
        create_file(dir_fd, "file");
        assert_not_directory(dir_fd, "file/", "target");
        assert_not_directory(dir_fd, "file", "target/");
        wasip1::path_filestat_get(dir_fd, 0, "file").expect("the file wasn't renamed");
        wasip1::path_unlink_file(dir_fd, "file").expect("removing a file");
    }
}

fn main() {
    // This test program requires a preopen at the path "sandbox", next to an
    // "outside" directory which isn't preopened.
    let dir_fd = match open_scratch_directory("sandbox") {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("failed to open sandbox preopen: {err}");
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe { test_path_rename_trailing_slashes(dir_fd) }
}
