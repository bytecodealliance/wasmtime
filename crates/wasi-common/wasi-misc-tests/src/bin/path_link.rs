use more_asserts::assert_gt;
use std::{env, process};
use wasi::wasi_unstable;
use wasi_misc_tests::open_scratch_directory;
use wasi_misc_tests::utils::{cleanup_dir, cleanup_file, create_dir, create_file};
use wasi_misc_tests::wasi_wrappers::{
    wasi_fd_fdstat_get, wasi_fd_filestat_get, wasi_path_link, wasi_path_open, wasi_path_symlink,
};

unsafe fn create_or_open(
    dir_fd: wasi_unstable::Fd,
    name: &str,
    flags: wasi_unstable::OFlags,
) -> wasi_unstable::Fd {
    let mut file_fd = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(dir_fd, 0, name, flags, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening '{}'",
        name
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );
    file_fd
}

unsafe fn open_link(dir_fd: wasi_unstable::Fd, name: &str) -> wasi_unstable::Fd {
    let mut file_fd = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(dir_fd, 0, name, 0, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a link '{}'",
        name
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );
    file_fd
}

unsafe fn check_rights(orig_fd: wasi_unstable::Fd, link_fd: wasi_unstable::Fd) {
    use std::mem::MaybeUninit;

    // Compare FileStats
    let mut orig_filestat: wasi_unstable::FileStat = MaybeUninit::zeroed().assume_init();
    let mut link_filestat: wasi_unstable::FileStat = MaybeUninit::zeroed().assume_init();
    let mut status = wasi_fd_filestat_get(orig_fd, &mut orig_filestat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading filestat of the source"
    );
    status = wasi_fd_filestat_get(link_fd, &mut link_filestat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading filestat of the link"
    );
    assert_eq!(orig_filestat, link_filestat, "filestats should match");

    // Compare FdStats
    let mut orig_fdstat: wasi_unstable::FdStat = MaybeUninit::zeroed().assume_init();
    let mut link_fdstat: wasi_unstable::FdStat = MaybeUninit::zeroed().assume_init();
    status = wasi_fd_fdstat_get(orig_fd, &mut orig_fdstat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading fdstat of the source"
    );
    status = wasi_fd_fdstat_get(link_fd, &mut link_fdstat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading fdstat of the link"
    );
    assert_eq!(orig_fdstat, link_fdstat, "fdstats should match");
}

unsafe fn test_path_link(dir_fd: wasi_unstable::Fd) {
    // Create a file
    let file_fd = create_or_open(dir_fd, "file", wasi_unstable::O_CREAT);

    // Create a link in the same directory and compare rights
    assert!(
        wasi_path_link(dir_fd, 0, "file", dir_fd, "link").is_ok(),
        "creating a link in the same directory"
    );
    let mut link_fd = open_link(dir_fd, "link");
    check_rights(file_fd, link_fd);
    cleanup_file(dir_fd, "link");

    // Create a link in a different directory and compare rights
    create_dir(dir_fd, "subdir");
    let subdir_fd = create_or_open(dir_fd, "subdir", wasi_unstable::O_DIRECTORY);
    assert!(
        wasi_path_link(dir_fd, 0, "file", subdir_fd, "link").is_ok(),
        "creating a link in subdirectory"
    );
    link_fd = open_link(subdir_fd, "link");
    check_rights(file_fd, link_fd);
    cleanup_file(subdir_fd, "link");
    cleanup_dir(dir_fd, "subdir");

    // Create a link to a path that already exists
    create_file(dir_fd, "link");
    assert_eq!(
        wasi_path_link(dir_fd, 0, "file", dir_fd, "link"),
        Err(wasi_unstable::EEXIST),
        "creating a link to existing path"
    );
    cleanup_file(dir_fd, "link");

    // Create a link to itself
    assert_eq!(
        wasi_path_link(dir_fd, 0, "file", dir_fd, "file"),
        Err(wasi_unstable::EEXIST),
        "creating a link to itself"
    );

    // Create a link where target is a directory
    create_dir(dir_fd, "link");
    assert_eq!(
        wasi_path_link(dir_fd, 0, "file", dir_fd, "link"),
        Err(wasi_unstable::EEXIST),
        "creating a link where target is a directory"
    );
    cleanup_dir(dir_fd, "link");

    // Create a link to a directory
    create_dir(dir_fd, "subdir");
    let subdir_fd = create_or_open(dir_fd, "subdir", wasi_unstable::O_DIRECTORY);
    assert_eq!(
        wasi_path_link(dir_fd, 0, "subdir", dir_fd, "link"),
        Err(wasi_unstable::EPERM),
        "creating a link to a directory"
    );
    cleanup_dir(dir_fd, "subdir");

    // Create a link to a file with trailing slash
    assert_eq!(
        wasi_path_link(dir_fd, 0, "file", dir_fd, "link/"),
        Err(wasi_unstable::ENOENT),
        "creating a link to a file with trailing slash"
    );

    // Create a link to a dangling symlink
    assert!(
        wasi_path_symlink("target", dir_fd, "symlink").is_ok(),
        "creating a dangling symlink"
    );
    assert_eq!(
        wasi_path_link(dir_fd, 0, "symlink", dir_fd, "link"),
        Err(wasi_unstable::ENOENT),
        "creating a link to a dangling symlink"
    );
    cleanup_file(dir_fd, "symlink");

    // Create a link to a symlink loop
    assert!(
        wasi_path_symlink("symlink", dir_fd, "symlink").is_ok(),
        "creating a symlink loop"
    );
    assert_eq!(
        wasi_path_link(dir_fd, 0, "symlink", dir_fd, "link"),
        Err(wasi_unstable::ELOOP),
        "creating a link to a symlink loop"
    );
    cleanup_file(dir_fd, "symlink");

    // Create a link where target is a dangling symlink
    assert!(
        wasi_path_symlink("target", dir_fd, "symlink").is_ok(),
        "creating a dangling symlink"
    );
    assert_eq!(
        wasi_path_link(dir_fd, 0, "file", dir_fd, "symlink"),
        Err(wasi_unstable::EEXIST),
        "creating a link where target is a dangling symlink"
    );
    cleanup_file(dir_fd, "symlink");

    // Create a link to a file following symlinks
    assert!(
        wasi_path_symlink("file", dir_fd, "symlink").is_ok(),
        "creating a valid symlink"
    );
    assert!(
        wasi_path_link(
            dir_fd,
            wasi_unstable::LOOKUP_SYMLINK_FOLLOW,
            "symlink",
            dir_fd,
            "link"
        )
        .is_ok(),
        "creating a link to a file following symlinks",
    );
    link_fd = open_link(dir_fd, "link");
    check_rights(file_fd, link_fd);
    cleanup_file(dir_fd, "link");
    cleanup_file(dir_fd, "symlink");

    // Create a link where target is a dangling symlink following symlinks
    assert!(
        wasi_path_symlink("target", dir_fd, "symlink").is_ok(),
        "creating a dangling symlink"
    );
    assert_eq!(
        wasi_path_link(
            dir_fd,
            wasi_unstable::LOOKUP_SYMLINK_FOLLOW,
            "symlink",
            dir_fd,
            "link"
        ),
        Err(wasi_unstable::ENOENT),
        "creating a link where target is a dangling symlink following symlinks"
    );
    cleanup_file(dir_fd, "symlink");

    // Create a link to a symlink loop following symlinks
    assert!(
        wasi_path_symlink("symlink", dir_fd, "symlink").is_ok(),
        "creating a symlink loop"
    );
    assert_eq!(
        wasi_path_link(
            dir_fd,
            wasi_unstable::LOOKUP_SYMLINK_FOLLOW,
            "symlink",
            dir_fd,
            "link"
        ),
        Err(wasi_unstable::ELOOP),
        "creating a link to a symlink loop following symlinks"
    );
    cleanup_file(dir_fd, "symlink");

    // Clean up.
    cleanup_file(dir_fd, "file");
}

fn main() {
    let mut args = env::args();
    let prog = args.next().unwrap();
    let arg = if let Some(arg) = args.next() {
        arg
    } else {
        eprintln!("usage: {} <scratch directory>", prog);
        process::exit(1);
    };

    // Open scratch directory
    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe { test_path_link(dir_fd) }
}
