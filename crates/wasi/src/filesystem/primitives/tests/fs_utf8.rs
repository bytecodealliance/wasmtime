// This file is derived from Rust's library/std/src/fs/tests.rs at revision
// e4b1d5841494d6eb7f4944c91a057e16b0f0a9ea.

#![cfg_attr(io_error_uncategorized, feature(io_error_uncategorized))]
#![cfg(feature = "fs_utf8")]

#[macro_use]
mod sys_common;
#[macro_use]
mod sys;

use std::io::prelude::*;

#[cfg(target_os = "macos")]
use crate::sys::weak::weak;
use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use cap_std::ambient_authority;
use cap_std::fs_utf8::{self as fs, Dir, OpenOptions};
#[cfg(target_os = "macos")]
use libc::{c_char, c_int};
use std::io::{self, ErrorKind, SeekFrom};
use std::str;
use std::sync::Arc;
#[cfg(not(racy_asserts))] // racy asserts are racy
use std::thread;
use std::time::{Duration, Instant};
use sys_common::io::{tmpdir_utf8 as tmpdir, TempDirUtf8 as TempDir};
use sys_common::symlink_junction_utf8 as symlink_junction;

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

#[cfg(not(windows))]
fn symlink_contents<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    tmpdir: &TempDir,
    dst: Q,
) -> io::Result<()> {
    tmpdir.symlink_contents(src, dst)
}
#[cfg(not(windows))]
fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(src: P, tmpdir: &TempDir, dst: Q) -> io::Result<()> {
    tmpdir.symlink(src, dst)
}
#[cfg(not(windows))]
fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    tmpdir: &TempDir,
    dst: Q,
) -> io::Result<()> {
    tmpdir.symlink(src, dst)
}
#[cfg(windows)]
fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(src: P, tmpdir: &TempDir, dst: Q) -> io::Result<()> {
    tmpdir.symlink_dir(src, dst)
}
#[cfg(windows)]
fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    tmpdir: &TempDir,
    dst: Q,
) -> io::Result<()> {
    tmpdir.symlink_file(src, dst)
}

// Several test fail on windows if the user does not have permission to
// create symlinks (the `SeCreateSymbolicLinkPrivilege`). Instead of
// disabling these test on Windows, use this function to test whether we
// have permission, and return otherwise. This way, we still don't run these
// tests most of the time, but at least we do if the user has the right
// permissions.
pub fn got_symlink_permission(tmpdir: &TempDir) -> bool {
    if cfg!(unix) {
        return true;
    }
    let link = "some_hopefully_unique_link_name";

    match symlink_file(r"nonexisting_target", tmpdir, link) {
        // ERROR_PRIVILEGE_NOT_HELD = 1314
        Err(ref err) if err.raw_os_error() == Some(1314) => false,
        Ok(_) | Err(_) => true,
    }
}

#[cfg(target_os = "macos")]
fn able_to_not_follow_symlinks_while_hard_linking() -> bool {
    weak!(fn linkat(c_int, *const c_char, c_int, *const c_char, c_int) -> c_int);
    linkat.get().is_some()
}

#[cfg(not(target_os = "macos"))]
fn able_to_not_follow_symlinks_while_hard_linking() -> bool {
    return true;
}

#[test]
fn file_test_io_smoke_test() {
    let message = "it's alright. have a good time";
    let tmpdir = tmpdir();
    let filename = "file_rt_io_file_test.txt";
    {
        let mut write_stream = check!(tmpdir.create(filename));
        check!(write_stream.write(message.as_bytes()));
    }
    {
        let mut read_stream = check!(tmpdir.open(filename));
        let mut read_buf = [0; 1028];
        let read_str = match check!(read_stream.read(&mut read_buf)) {
            0 => panic!("shouldn't happen"),
            n => str::from_utf8(&read_buf[..n]).unwrap().to_string(),
        };
        assert_eq!(read_str, message);
    }
    check!(tmpdir.remove_file(filename));
}

#[test]
fn invalid_path_raises() {
    let tmpdir = tmpdir();
    let filename = "file_that_does_not_exist.txt";
    let result = tmpdir.open(filename);

    #[cfg(any(all(unix, not(target_os = "vxworks")), target_os = "wasi"))]
    error!(result, "No such file or directory");
    #[cfg(target_os = "vxworks")]
    error!(result, "no such file or directory");
    #[cfg(windows)]
    error!(result, 2); // ERROR_FILE_NOT_FOUND
}

#[test]
fn file_test_iounlinking_invalid_path_should_raise_condition() {
    let tmpdir = tmpdir();
    let filename = "file_another_file_that_does_not_exist.txt";

    let result = tmpdir.remove_file(filename);

    #[cfg(any(all(unix, not(target_os = "vxworks")), target_os = "wasi"))]
    error!(result, "No such file or directory");
    #[cfg(target_os = "vxworks")]
    error!(result, "no such file or directory");
    #[cfg(windows)]
    error!(result, 2); // ERROR_FILE_NOT_FOUND
}

#[test]
fn file_test_io_non_positional_read() {
    let message: &str = "ten-four";
    let mut read_mem = [0; 8];
    let tmpdir = tmpdir();
    let filename = "file_rt_io_file_test_positional.txt";
    {
        let mut rw_stream = check!(tmpdir.create(filename));
        check!(rw_stream.write(message.as_bytes()));
    }
    {
        let mut read_stream = check!(tmpdir.open(filename));
        {
            let read_buf = &mut read_mem[0..4];
            check!(read_stream.read(read_buf));
        }
        {
            let read_buf = &mut read_mem[4..8];
            check!(read_stream.read(read_buf));
        }
    }
    check!(tmpdir.remove_file(filename));
    let read_str = str::from_utf8(&read_mem).unwrap();
    assert_eq!(read_str, message);
}

#[test]
fn file_test_io_seek_and_tell_smoke_test() {
    let message = "ten-four";
    let mut read_mem = [0; 4];
    let set_cursor = 4 as u64;
    let tell_pos_pre_read;
    let tell_pos_post_read;
    let tmpdir = tmpdir();
    let filename = "file_rt_io_file_test_seeking.txt";
    {
        let mut rw_stream = check!(tmpdir.create(filename));
        check!(rw_stream.write(message.as_bytes()));
    }
    {
        let mut read_stream = check!(tmpdir.open(filename));
        check!(read_stream.seek(SeekFrom::Start(set_cursor)));
        tell_pos_pre_read = check!(read_stream.seek(SeekFrom::Current(0)));
        check!(read_stream.read(&mut read_mem));
        tell_pos_post_read = check!(read_stream.seek(SeekFrom::Current(0)));
    }
    check!(tmpdir.remove_file(filename));
    let read_str = str::from_utf8(&read_mem).unwrap();
    assert_eq!(read_str, &message[4..8]);
    assert_eq!(tell_pos_pre_read, set_cursor);
    assert_eq!(tell_pos_post_read, message.len() as u64);
}

#[test]
fn file_test_io_seek_and_write() {
    let initial_msg = "food-is-yummy";
    let overwrite_msg = "-the-bar!!";
    let final_msg = "foo-the-bar!!";
    let seek_idx = 3;
    let mut read_mem = [0; 13];
    let tmpdir = tmpdir();
    let filename = "file_rt_io_file_test_seek_and_write.txt";
    {
        let mut rw_stream = check!(tmpdir.create(filename));
        check!(rw_stream.write(initial_msg.as_bytes()));
        check!(rw_stream.seek(SeekFrom::Start(seek_idx)));
        check!(rw_stream.write(overwrite_msg.as_bytes()));
    }
    {
        let mut read_stream = check!(tmpdir.open(filename));
        check!(read_stream.read(&mut read_mem));
    }
    check!(tmpdir.remove_file(filename));
    let read_str = str::from_utf8(&read_mem).unwrap();
    assert!(read_str == final_msg);
}

#[test]
fn file_test_io_seek_shakedown() {
    //                   01234567890123
    let initial_msg = "qwer-asdf-zxcv";
    let chunk_one: &str = "qwer";
    let chunk_two: &str = "asdf";
    let chunk_three: &str = "zxcv";
    let mut read_mem = [0; 4];
    let tmpdir = tmpdir();
    let filename = "file_rt_io_file_test_seek_shakedown.txt";
    {
        let mut rw_stream = check!(tmpdir.create(filename));
        check!(rw_stream.write(initial_msg.as_bytes()));
    }
    {
        let mut read_stream = check!(tmpdir.open(filename));

        check!(read_stream.seek(SeekFrom::End(-4)));
        check!(read_stream.read(&mut read_mem));
        assert_eq!(str::from_utf8(&read_mem).unwrap(), chunk_three);

        check!(read_stream.seek(SeekFrom::Current(-9)));
        check!(read_stream.read(&mut read_mem));
        assert_eq!(str::from_utf8(&read_mem).unwrap(), chunk_two);

        check!(read_stream.seek(SeekFrom::Start(0)));
        check!(read_stream.read(&mut read_mem));
        assert_eq!(str::from_utf8(&read_mem).unwrap(), chunk_one);
    }
    check!(tmpdir.remove_file(filename));
}

#[test]
fn file_test_io_eof() {
    let tmpdir = tmpdir();
    let filename = "file_rt_io_file_test_eof.txt";
    let mut buf = [0; 256];
    {
        let oo = OpenOptions::new()
            .create_new(true)
            .write(true)
            .read(true)
            .clone();
        let mut rw = check!(tmpdir.open_with(filename, &oo));
        assert_eq!(check!(rw.read(&mut buf)), 0);
        assert_eq!(check!(rw.read(&mut buf)), 0);
    }
    check!(tmpdir.remove_file(filename));
}

#[test]
#[cfg(unix)]
fn file_test_io_read_write_at() {
    use cap_std::fs_utf8::FileExt;

    let tmpdir = tmpdir();
    let filename = "file_rt_io_file_test_read_write_at.txt";
    let mut buf = [0; 256];
    let write1 = "asdf";
    let write2 = "qwer-";
    let write3 = "-zxcv";
    let content = "qwer-asdf-zxcv";
    {
        let oo = OpenOptions::new()
            .create_new(true)
            .write(true)
            .read(true)
            .clone();
        let mut rw = check!(tmpdir.open_with(filename, &oo));
        assert_eq!(check!(rw.write_at(write1.as_bytes(), 5)), write1.len());
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 0);
        assert_eq!(check!(rw.read_at(&mut buf, 5)), write1.len());
        assert_eq!(str::from_utf8(&buf[..write1.len()]), Ok(write1));
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 0);
        assert_eq!(
            check!(rw.read_at(&mut buf[..write2.len()], 0)),
            write2.len()
        );
        assert_eq!(str::from_utf8(&buf[..write2.len()]), Ok("\0\0\0\0\0"));
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 0);
        assert_eq!(check!(rw.write(write2.as_bytes())), write2.len());
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 5);
        assert_eq!(check!(rw.read(&mut buf)), write1.len());
        assert_eq!(str::from_utf8(&buf[..write1.len()]), Ok(write1));
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 9);
        assert_eq!(
            check!(rw.read_at(&mut buf[..write2.len()], 0)),
            write2.len()
        );
        assert_eq!(str::from_utf8(&buf[..write2.len()]), Ok(write2));
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 9);
        assert_eq!(check!(rw.write_at(write3.as_bytes(), 9)), write3.len());
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 9);
    }
    {
        let mut read = check!(tmpdir.open(filename));
        assert_eq!(check!(read.read_at(&mut buf, 0)), content.len());
        assert_eq!(str::from_utf8(&buf[..content.len()]), Ok(content));
        assert_eq!(check!(read.seek(SeekFrom::Current(0))), 0);
        assert_eq!(check!(read.seek(SeekFrom::End(-5))), 9);
        assert_eq!(check!(read.read_at(&mut buf, 0)), content.len());
        assert_eq!(str::from_utf8(&buf[..content.len()]), Ok(content));
        assert_eq!(check!(read.seek(SeekFrom::Current(0))), 9);
        assert_eq!(check!(read.read(&mut buf)), write3.len());
        assert_eq!(str::from_utf8(&buf[..write3.len()]), Ok(write3));
        assert_eq!(check!(read.seek(SeekFrom::Current(0))), 14);
        assert_eq!(check!(read.read_at(&mut buf, 0)), content.len());
        assert_eq!(str::from_utf8(&buf[..content.len()]), Ok(content));
        assert_eq!(check!(read.seek(SeekFrom::Current(0))), 14);
        assert_eq!(check!(read.read_at(&mut buf, 14)), 0);
        assert_eq!(check!(read.read_at(&mut buf, 15)), 0);
        assert_eq!(check!(read.seek(SeekFrom::Current(0))), 14);
    }
    check!(tmpdir.remove_file(filename));
}

// Darwin doesn't have a way to change the permissions on a file, relative
// to a directory handle, with no read or write access, without blindly
// following symlinks.
#[test]
#[cfg(unix)]
#[cfg_attr(
    any(
        target_os = "macos",
        target_os = "ios",
        target_os = "tvos",
        target_os = "watchos",
        target_os = "visionos",
    ),
    ignore
)]
fn set_get_unix_permissions() {
    use cap_std::fs::PermissionsExt;

    let tmpdir = tmpdir();
    let filename = "set_get_unix_permissions";
    check!(tmpdir.create_dir(filename));
    let mask = 0o7777;

    check!(tmpdir.set_permissions(filename, fs::Permissions::from_mode(0)));
    let metadata0 = check!(tmpdir.metadata(filename));
    assert_eq!(mask & metadata0.permissions().mode(), 0);

    check!(tmpdir.set_permissions(filename, fs::Permissions::from_mode(0o1777)));
    let metadata1 = check!(tmpdir.metadata(filename));
    #[cfg(any(all(unix, not(target_os = "vxworks")), target_os = "wasi"))]
    assert_eq!(mask & metadata1.permissions().mode(), 0o1777);
    #[cfg(target_os = "vxworks")]
    assert_eq!(mask & metadata1.permissions().mode(), 0o0777);
}

#[test]
#[cfg(windows)]
fn file_test_io_seek_read_write() {
    use cap_std::fs_utf8::FileExt;

    let tmpdir = tmpdir();
    let filename = "file_rt_io_file_test_seek_read_write.txt";
    let mut buf = [0; 256];
    let write1 = "asdf";
    let write2 = "qwer-";
    let write3 = "-zxcv";
    let content = "qwer-asdf-zxcv";
    {
        let oo = OpenOptions::new()
            .create_new(true)
            .write(true)
            .read(true)
            .clone();
        let mut rw = check!(tmpdir.open_with(filename, &oo));
        assert_eq!(check!(rw.seek_write(write1.as_bytes(), 5)), write1.len());
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 9);
        assert_eq!(check!(rw.seek_read(&mut buf, 5)), write1.len());
        assert_eq!(str::from_utf8(&buf[..write1.len()]), Ok(write1));
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 9);
        assert_eq!(check!(rw.seek(SeekFrom::Start(0))), 0);
        assert_eq!(check!(rw.write(write2.as_bytes())), write2.len());
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 5);
        assert_eq!(check!(rw.read(&mut buf)), write1.len());
        assert_eq!(str::from_utf8(&buf[..write1.len()]), Ok(write1));
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 9);
        assert_eq!(
            check!(rw.seek_read(&mut buf[..write2.len()], 0)),
            write2.len()
        );
        assert_eq!(str::from_utf8(&buf[..write2.len()]), Ok(write2));
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 5);
        assert_eq!(check!(rw.seek_write(write3.as_bytes(), 9)), write3.len());
        assert_eq!(check!(rw.seek(SeekFrom::Current(0))), 14);
    }
    {
        let mut read = check!(tmpdir.open(filename));
        assert_eq!(check!(read.seek_read(&mut buf, 0)), content.len());
        assert_eq!(str::from_utf8(&buf[..content.len()]), Ok(content));
        assert_eq!(check!(read.seek(SeekFrom::Current(0))), 14);
        assert_eq!(check!(read.seek(SeekFrom::End(-5))), 9);
        assert_eq!(check!(read.seek_read(&mut buf, 0)), content.len());
        assert_eq!(str::from_utf8(&buf[..content.len()]), Ok(content));
        assert_eq!(check!(read.seek(SeekFrom::Current(0))), 14);
        assert_eq!(check!(read.seek(SeekFrom::End(-5))), 9);
        assert_eq!(check!(read.read(&mut buf)), write3.len());
        assert_eq!(str::from_utf8(&buf[..write3.len()]), Ok(write3));
        assert_eq!(check!(read.seek(SeekFrom::Current(0))), 14);
        assert_eq!(check!(read.seek_read(&mut buf, 0)), content.len());
        assert_eq!(str::from_utf8(&buf[..content.len()]), Ok(content));
        assert_eq!(check!(read.seek(SeekFrom::Current(0))), 14);
        assert_eq!(check!(read.seek_read(&mut buf, 14)), 0);
        assert_eq!(check!(read.seek_read(&mut buf, 15)), 0);
    }
    check!(tmpdir.remove_file(filename));
}

#[test]
fn file_test_stat_is_correct_on_is_file() {
    let tmpdir = tmpdir();
    let filename = "file_stat_correct_on_is_file.txt";
    {
        let mut opts = OpenOptions::new();
        let mut fs = check!(tmpdir.open_with(filename, opts.read(true).write(true).create(true)));
        let msg = "hw";
        fs.write(msg.as_bytes()).unwrap();

        let fstat_res = check!(fs.metadata());
        assert!(fstat_res.is_file());
    }
    let stat_res_fn = check!(tmpdir.metadata(filename));
    assert!(stat_res_fn.is_file());
    check!(tmpdir.remove_file(filename));
}

#[test]
fn file_test_stat_is_correct_on_is_dir() {
    let tmpdir = tmpdir();
    let filename = "file_stat_correct_on_is_dir";
    check!(tmpdir.create_dir(filename));
    let stat_res_fn = check!(tmpdir.metadata(filename));
    assert!(stat_res_fn.is_dir());
    check!(tmpdir.remove_dir(filename));
}

#[test]
fn file_test_fileinfo_false_when_checking_is_file_on_a_directory() {
    let tmpdir = tmpdir();
    let dir = "fileinfo_false_on_dir";
    check!(tmpdir.create_dir(dir));
    assert!(!tmpdir.is_file(dir));
    check!(tmpdir.remove_dir(dir));
}

#[test]
fn file_test_fileinfo_check_exists_before_and_after_file_creation() {
    let tmpdir = tmpdir();
    let file = "fileinfo_check_exists_b_and_a.txt";
    check!(check!(tmpdir.create(file)).write(b"foo"));
    assert!(tmpdir.exists(file));
    assert!(tmpdir.try_exists(file).unwrap());
    check!(tmpdir.remove_file(file));
    assert!(!tmpdir.exists(file));
    assert!(!tmpdir.try_exists(file).unwrap());
}

#[test]
fn file_test_directoryinfo_check_exists_before_and_after_mkdir() {
    let tmpdir = tmpdir();
    let dir = "before_and_after_dir";
    assert!(!tmpdir.exists(dir));
    check!(tmpdir.create_dir(dir));
    assert!(tmpdir.exists(dir));
    assert!(tmpdir.is_dir(dir));
    check!(tmpdir.remove_dir(dir));
    assert!(!tmpdir.exists(dir));
}

#[test]
fn file_test_directoryinfo_readdir() {
    let tmpdir = tmpdir();
    let dir = "di_readdir";
    check!(tmpdir.create_dir(dir));
    let prefix = "foo";
    for n in 0..3 {
        let f = format!("{}.txt", n);
        let mut w = check!(tmpdir.create(&f));
        let msg_str = format!("{}{}", prefix, n.to_string());
        let msg = msg_str.as_bytes();
        check!(w.write(msg));
    }
    let files = check!(tmpdir.read_dir(dir));
    let mut mem = [0; 4];
    for f in files {
        let f = f.unwrap().file_name().unwrap();
        {
            check!(check!(tmpdir.open(&f)).read(&mut mem));
            let read_str = str::from_utf8(&mem).unwrap();
            let expected = format!("{}{}", prefix, f.as_str());
            assert_eq!(expected, read_str);
        }
        check!(tmpdir.remove_file(&f));
    }
    check!(tmpdir.remove_dir(dir));
}

#[test]
fn file_create_new_already_exists_error() {
    let tmpdir = tmpdir();
    let file = "file_create_new_error_exists";
    check!(tmpdir.create(file));
    let e = tmpdir
        .open_with(file, &fs::OpenOptions::new().write(true).create_new(true))
        .unwrap_err();
    assert_eq!(e.kind(), ErrorKind::AlreadyExists);
}

#[test]
fn mkdir_path_already_exists_error() {
    let tmpdir = tmpdir();
    let dir = "mkdir_error_twice";
    check!(tmpdir.create_dir(dir));
    let e = tmpdir.create_dir(dir).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::AlreadyExists);
}

#[test]
fn recursive_mkdir() {
    let tmpdir = tmpdir();
    let dir = "d1/d2";
    check!(tmpdir.create_dir_all(dir));
    assert!(tmpdir.is_dir(dir));
}

#[test]
fn recursive_mkdir_failure() {
    let tmpdir = tmpdir();
    let dir = "d1";
    let file = "f1";

    check!(tmpdir.create_dir_all(&dir));
    check!(tmpdir.create(&file));

    let result = tmpdir.create_dir_all(&file);

    assert!(result.is_err());
}

#[test]
#[cfg(not(racy_asserts))] // racy asserts are racy
fn concurrent_recursive_mkdir() {
    for _ in 0..100 {
        let dir = tmpdir();
        let mut name = PathBuf::from("a");
        for _ in 0..40 {
            name = name.join("a");
        }
        let mut join = vec![];
        for _ in 0..8 {
            let dir = check!(dir.try_clone());
            let name = name.clone();
            join.push(thread::spawn(move || {
                check!(dir.create_dir_all(&name));
            }))
        }

        // No `Display` on result of `join()`
        join.drain(..).map(|join| join.join().unwrap()).count();
    }
}

#[test]
fn recursive_mkdir_slash() {
    let tmpdir = tmpdir();
    error_contains!(
        tmpdir.create_dir_all(Path::new("/")),
        "a path led outside of the filesystem"
    );
}

#[test]
fn recursive_mkdir_dot() {
    let tmpdir = tmpdir();
    check!(tmpdir.create_dir_all(Path::new(".")));
}

#[test]
fn recursive_mkdir_empty() {
    let tmpdir = tmpdir();
    check!(tmpdir.create_dir_all(Path::new("")));
}

#[test]
fn recursive_rmdir() {
    let tmpdir = tmpdir();
    let d1 = PathBuf::from("d1");
    let dt = d1.join("t");
    let dtt = dt.join("t");
    let d2 = PathBuf::from("d2");
    let canary = d2.join("do_not_delete");
    check!(tmpdir.create_dir_all(&dtt));
    check!(tmpdir.create_dir_all(&d2));
    check!(check!(tmpdir.create(&canary)).write(b"foo"));
    check!(symlink_junction(d2, &tmpdir, &dt.join("d2")));
    let _ = symlink_file(&canary, &tmpdir, &d1.join("canary"));
    check!(tmpdir.remove_dir_all(&d1));

    assert!(!tmpdir.is_dir(d1));
    assert!(tmpdir.exists(canary));
}

// See the comments on `create_dir_all_with_junctions` in tests/fs.rs about Windows.
#[cfg_attr(windows, ignore)]
#[test]
fn recursive_rmdir_of_symlink() {
    // test we do not recursively delete a symlink but only dirs.
    let tmpdir = tmpdir();
    let link = "d1";
    let dir = "d2";
    let canary = "do_not_delete";
    check!(tmpdir.create_dir_all(&dir));
    check!(check!(tmpdir.create(&canary)).write(b"foo"));
    check!(symlink_junction(&dir, &tmpdir, link));
    check!(tmpdir.remove_dir_all(link));

    assert!(!tmpdir.is_dir(link));
    assert!(tmpdir.exists(canary));
}

#[test]
fn recursive_rmdir_of_file_fails() {
    // test we do not delete a directly specified file.
    let tmpdir = tmpdir();
    let canary = "do_not_delete";
    check!(check!(tmpdir.create(canary)).write(b"foo"));
    let result = tmpdir.remove_dir_all(canary);
    #[cfg(unix)]
    error!(result, "Not a directory");
    #[cfg(windows)]
    error!(result, 267); // ERROR_DIRECTORY - The directory name is invalid.
    assert!(result.is_err());
    assert!(tmpdir.exists(canary));
}

#[test]
// only Windows makes a distinction between file and directory symlinks.
#[cfg(windows)]
fn recursive_rmdir_of_file_symlink() {
    let tmpdir = tmpdir();
    if !got_symlink_permission(&tmpdir) {
        return;
    };

    let f1 = "f1";
    let f2 = "f2";
    check!(check!(tmpdir.create(&f1)).write(b"foo"));
    check!(symlink_file(&f1, &tmpdir, &f2));
    match tmpdir.remove_dir_all(&f2) {
        Ok(..) => panic!("wanted a failure"),
        Err(..) => {}
    }
}

#[test]
#[ignore] // takes too much time
fn recursive_rmdir_toctou() {
    // Test for time-of-check to time-of-use issues.
    //
    // Scenario:
    // The attacker wants to get directory contents deleted, to which he does not
    // have access. He has a way to get a privileged Rust binary call
    // `std::fs::remove_dir_all()` on a directory he controls, e.g. in his home
    // directory.
    //
    // The POC sets up the `attack_dest/attack_file` which the attacker wants to
    // have deleted. The attacker repeatedly creates a directory and replaces
    // it with a symlink from `victim_del` to `attack_dest` while the victim
    // code calls `std::fs::remove_dir_all()` on `victim_del`. After a few
    // seconds the attack has succeeded and `attack_dest/attack_file` is
    // deleted.
    let tmpdir = tmpdir();
    let victim_del_path = "victim_del";

    // setup dest
    let attack_dest_dir = "attack_dest";
    let attack_dest_dir = Path::new(attack_dest_dir);
    tmpdir.create_dir(attack_dest_dir).unwrap();
    let attack_dest_file = "attack_dest/attack_file";
    tmpdir.create(attack_dest_file).unwrap();

    let drop_canary_arc = Arc::new(());
    let drop_canary_weak = Arc::downgrade(&drop_canary_arc);

    eprintln!("x: {:?}", &victim_del_path);

    // victim just continuously removes `victim_del`
    let tmpdir_clone = tmpdir.try_clone().unwrap();
    let _t = thread::spawn(move || {
        while drop_canary_weak.upgrade().is_some() {
            let _ = tmpdir_clone.remove_dir_all(victim_del_path);
        }
    });

    // attacker (could of course be in a separate process)
    let start_time = Instant::now();
    while Instant::now().duration_since(start_time) < Duration::from_secs(1000) {
        if !tmpdir.exists(attack_dest_file) {
            panic!(
                "Victim deleted symlinked file outside of victim_del. Attack succeeded in {:?}.",
                Instant::now().duration_since(start_time)
            );
        }
        let _ = tmpdir.create_dir(victim_del_path);
        let _ = tmpdir.remove_dir(victim_del_path);
        let _ = symlink_dir(attack_dest_dir, &tmpdir, victim_del_path);
    }
}

#[test]
fn unicode_path_is_dir() {
    let tmpdir = tmpdir();

    assert!(tmpdir.is_dir(Path::new(".")));
    assert!(!tmpdir.is_dir(Path::new("test/stdtest/fs.rs")));

    let mut dirpath = PathBuf::new();
    dirpath.push("test-가一ー你好");
    check!(tmpdir.create_dir(&dirpath));
    assert!(tmpdir.is_dir(&dirpath));

    let mut filepath = dirpath;
    filepath.push("unicode-file-\u{ac00}\u{4e00}\u{30fc}\u{4f60}\u{597d}.rs");
    check!(tmpdir.create(&filepath)); // ignore return; touch only
    assert!(!tmpdir.is_dir(&filepath));
    assert!(tmpdir.exists(filepath));
}

#[test]
fn unicode_path_exists() {
    let tmpdir = tmpdir();

    assert!(tmpdir.exists(Path::new(".")));
    assert!(!tmpdir.exists(Path::new("test/nonexistent-bogus-path")));

    let unicode = PathBuf::new();
    let unicode = unicode.join("test-각丁ー再见");
    check!(tmpdir.create_dir(&unicode));
    assert!(tmpdir.exists(unicode));
    assert!(!tmpdir.exists(Path::new("test/unicode-bogus-path-각丁ー再见")));
}

#[test]
fn copy_file_does_not_exist() {
    let tmpdir = tmpdir();
    let from = Path::new("test/nonexistent-bogus-path");
    let to = Path::new("test/other-bogus-path");

    match tmpdir.copy(&from, &tmpdir, &to) {
        Ok(..) => panic!(),
        Err(..) => {
            assert!(!tmpdir.exists(from));
            assert!(!tmpdir.exists(to));
        }
    }
}

#[test]
fn copy_src_does_not_exist() {
    let tmpdir = tmpdir();
    let from = Path::new("test/nonexistent-bogus-path");
    let to = "out.txt";
    check!(check!(tmpdir.create(&to)).write(b"hello"));
    assert!(tmpdir.copy(&from, &tmpdir, &to).is_err());
    assert!(!tmpdir.exists(from));
    let mut v = Vec::new();
    check!(check!(tmpdir.open(&to)).read_to_end(&mut v));
    assert_eq!(v, b"hello");
}

#[test]
fn copy_file_ok() {
    let tmpdir = tmpdir();
    let input = "in.txt";
    let out = "out.txt";

    check!(check!(tmpdir.create(&input)).write(b"hello"));
    check!(tmpdir.copy(&input, &tmpdir, &out));
    let mut v = Vec::new();
    check!(check!(tmpdir.open(&out)).read_to_end(&mut v));
    assert_eq!(v, b"hello");

    assert_eq!(
        check!(tmpdir.metadata(input)).permissions(),
        check!(tmpdir.metadata(out)).permissions()
    );
}

#[test]
fn copy_file_dst_dir() {
    let tmpdir = tmpdir();
    let out = "out";

    check!(tmpdir.create(&out));
    match tmpdir.copy(&*out, &tmpdir, ".") {
        Ok(..) => panic!(),
        Err(..) => {}
    }
}

#[test]
fn copy_file_dst_exists() {
    let tmpdir = tmpdir();
    let input = "in";
    let output = "out";

    check!(check!(tmpdir.create(&input)).write("foo".as_bytes()));
    check!(check!(tmpdir.create(&output)).write("bar".as_bytes()));
    check!(tmpdir.copy(&input, &tmpdir, &output));

    let mut v = Vec::new();
    check!(check!(tmpdir.open(&output)).read_to_end(&mut v));
    assert_eq!(v, b"foo".to_vec());
}

#[test]
fn copy_file_src_dir() {
    let tmpdir = tmpdir();
    let out = "out";

    match tmpdir.copy(".", &tmpdir, &out) {
        Ok(..) => panic!(),
        Err(..) => {}
    }
    assert!(!tmpdir.exists(out));
}

#[test]
fn copy_file_preserves_perm_bits() {
    let tmpdir = tmpdir();
    let input = "in.txt";
    let out = "out.txt";

    let attr = check!(check!(tmpdir.create(&input)).metadata());
    let mut p = attr.permissions();
    p.set_readonly(true);
    check!(tmpdir.set_permissions(&input, p));
    check!(tmpdir.copy(&input, &tmpdir, &out));
    assert!(check!(tmpdir.metadata(out)).permissions().readonly());
    check!(tmpdir.set_permissions(&input, attr.permissions()));
    check!(tmpdir.set_permissions(&out, attr.permissions()));
}

#[test]
#[cfg(windows)]
fn copy_file_preserves_streams() {
    let tmp = tmpdir();
    check!(check!(tmp.create("in.txt:bunny")).write("carrot".as_bytes()));
    assert_eq!(check!(tmp.copy("in.txt", &tmp, "out.txt")), 0);
    assert_eq!(check!(tmp.metadata("out.txt")).len(), 0);
    let mut v = Vec::new();
    check!(check!(tmp.open("out.txt:bunny")).read_to_end(&mut v));
    assert_eq!(v, b"carrot".to_vec());
}

#[test]
fn copy_file_returns_metadata_len() {
    let tmp = tmpdir();
    let in_path = "in.txt";
    let out_path = "out.txt";
    check!(check!(tmp.create(&in_path)).write(b"lettuce"));
    #[cfg(windows)]
    check!(check!(tmp.create("in.txt:bunny")).write(b"carrot"));
    let copied_len = check!(tmp.copy(&in_path, &tmp, &out_path));
    assert_eq!(check!(tmp.metadata(out_path)).len(), copied_len);
}

#[test]
fn copy_file_follows_dst_symlink() {
    let tmp = tmpdir();
    if !got_symlink_permission(&tmp) {
        return;
    };

    let in_path = "in.txt";
    let out_path = "out.txt";
    let out_path_symlink = "out_symlink.txt";

    check!(tmp.write(&in_path, "foo"));
    check!(tmp.write(&out_path, "bar"));
    check!(symlink_file(&out_path, &tmp, &out_path_symlink));

    check!(tmp.copy(&in_path, &tmp, &out_path_symlink));

    assert!(check!(tmp.symlink_metadata(out_path_symlink))
        .file_type()
        .is_symlink());
    assert_eq!(check!(tmp.read(&out_path_symlink)), b"foo".to_vec());
    assert_eq!(check!(tmp.read(&out_path)), b"foo".to_vec());
}

#[test]
fn symlinks_work() {
    let tmpdir = tmpdir();
    if !got_symlink_permission(&tmpdir) {
        return;
    };

    let input = "in.txt";
    let out = "out.txt";

    check!(check!(tmpdir.create(&input)).write("foobar".as_bytes()));
    check!(symlink_file(&input, &tmpdir, &out));
    assert!(check!(tmpdir.symlink_metadata(out))
        .file_type()
        .is_symlink());
    assert_eq!(
        check!(tmpdir.metadata(&out)).len(),
        check!(tmpdir.metadata(&input)).len()
    );
    let mut v = Vec::new();
    check!(check!(tmpdir.open(&out)).read_to_end(&mut v));
    assert_eq!(v, b"foobar".to_vec());
}

#[test]
fn symlink_noexist() {
    // Symlinks can point to things that don't exist
    let tmpdir = tmpdir();
    if !got_symlink_permission(&tmpdir) {
        return;
    };

    // Use a relative path for testing. Symlinks get normalized by Windows,
    // so we might not get the same path back for absolute paths
    check!(symlink_file(&"foo", &tmpdir, "bar"));
    assert_eq!(check!(tmpdir.read_link("bar")).as_str(), "foo");
}

#[test]
fn read_link() {
    if cfg!(windows) {
        // directory symlink
        let root = Dir::open_ambient_dir(r"C:\", ambient_authority()).unwrap();
        error_contains!(
            root.read_link(r"Users\All Users"),
            "a path led outside of the filesystem"
        );
        // junction
        error_contains!(
            root.read_link(r"Users\Default User"),
            "a path led outside of the filesystem"
        );
        // junction with special permissions
        error_contains!(
            root.read_link(r"Documents and Settings\"),
            "a path led outside of the filesystem"
        );
    }
    let tmpdir = tmpdir();
    let link = "link";
    if !got_symlink_permission(&tmpdir) {
        return;
    };
    check!(symlink_file(&"foo", &tmpdir, &link));
    assert_eq!(check!(tmpdir.read_link(&link)).as_str(), "foo");
}

#[test]
fn readlink_not_symlink() {
    let tmpdir = tmpdir();
    match tmpdir.read_link(".") {
        Ok(..) => panic!("wanted a failure"),
        Err(..) => {}
    }
}

#[cfg(not(windows))]
#[test]
fn read_link_contents() {
    let tmpdir = tmpdir();
    let link = "link";
    if !got_symlink_permission(&tmpdir) {
        return;
    };
    check!(symlink_file(&"foo", &tmpdir, &link));
    assert_eq!(check!(tmpdir.read_link_contents(&link)).as_str(), "foo");
}

#[cfg(not(windows))]
#[test]
fn read_link_contents_absolute() {
    let tmpdir = tmpdir();
    let link = "link";
    if !got_symlink_permission(&tmpdir) {
        return;
    };
    check!(symlink_contents(&"/foo", &tmpdir, &link));
    assert_eq!(check!(tmpdir.read_link_contents(&link)).as_str(), "/foo");
}

#[test]
fn links_work() {
    let tmpdir = tmpdir();
    let input = "in.txt";
    let out = "out.txt";

    check!(check!(tmpdir.create(&input)).write("foobar".as_bytes()));
    check!(tmpdir.hard_link(&input, &tmpdir, &out));
    assert_eq!(
        check!(tmpdir.metadata(&out)).len(),
        check!(tmpdir.metadata(&input)).len()
    );
    assert_eq!(
        check!(tmpdir.metadata(&out)).len(),
        check!(tmpdir.metadata(input)).len()
    );
    let mut v = Vec::new();
    check!(check!(tmpdir.open(&out)).read_to_end(&mut v));
    assert_eq!(v, b"foobar".to_vec());

    // can't link to yourself
    match tmpdir.hard_link(&input, &tmpdir, &input) {
        Ok(..) => panic!("wanted a failure"),
        Err(..) => {}
    }
    // can't link to something that doesn't exist
    match tmpdir.hard_link("foo", &tmpdir, "bar") {
        Ok(..) => panic!("wanted a failure"),
        Err(..) => {}
    }
}

#[test]
fn chmod_works() {
    let tmpdir = tmpdir();
    let file = "in.txt";

    check!(tmpdir.create(&file));
    let attr = check!(tmpdir.metadata(&file));
    assert!(!attr.permissions().readonly());
    let mut p = attr.permissions();
    p.set_readonly(true);
    check!(tmpdir.set_permissions(&file, p.clone()));
    let attr = check!(tmpdir.metadata(&file));
    assert!(attr.permissions().readonly());

    match tmpdir.set_permissions("foo", p.clone()) {
        Ok(..) => panic!("wanted an error"),
        Err(..) => {}
    }

    p.set_readonly(false);
    check!(tmpdir.set_permissions(&file, p));
}

#[test]
fn fchmod_works() {
    let tmpdir = tmpdir();
    let path = "in.txt";

    let file = check!(tmpdir.create(&path));
    let attr = check!(tmpdir.metadata(&path));
    assert!(!attr.permissions().readonly());
    let mut p = attr.permissions();
    p.set_readonly(true);
    check!(file.set_permissions(p.clone()));
    let attr = check!(tmpdir.metadata(&path));
    assert!(attr.permissions().readonly());

    p.set_readonly(false);
    check!(file.set_permissions(p));
}

#[test]
fn sync_doesnt_kill_anything() {
    let tmpdir = tmpdir();
    let path = "in.txt";

    let mut file = check!(tmpdir.create(&path));
    check!(file.sync_all());
    check!(file.sync_data());
    check!(file.write(b"foo"));
    check!(file.sync_all());
    check!(file.sync_data());
}

#[test]
fn truncate_works() {
    let tmpdir = tmpdir();
    let path = "in.txt";

    let mut file = check!(tmpdir.create(&path));
    check!(file.write(b"foo"));
    check!(file.sync_all());

    // Do some simple things with truncation
    assert_eq!(check!(file.metadata()).len(), 3);
    check!(file.set_len(10));
    assert_eq!(check!(file.metadata()).len(), 10);
    check!(file.write(b"bar"));
    check!(file.sync_all());
    assert_eq!(check!(file.metadata()).len(), 10);

    let mut v = Vec::new();
    check!(check!(tmpdir.open(&path)).read_to_end(&mut v));
    assert_eq!(v, b"foobar\0\0\0\0".to_vec());

    // Truncate to a smaller length, don't seek, and then write something.
    // Ensure that the intermediate zeroes are all filled in (we have `seek`ed
    // past the end of the file).
    check!(file.set_len(2));
    assert_eq!(check!(file.metadata()).len(), 2);
    check!(file.write(b"wut"));
    check!(file.sync_all());
    assert_eq!(check!(file.metadata()).len(), 9);
    let mut v = Vec::new();
    check!(check!(tmpdir.open(&path)).read_to_end(&mut v));
    assert_eq!(v, b"fo\0\0\0\0wut".to_vec());
}

#[test]
fn open_flavors() {
    use cap_std::fs::OpenOptions as OO;
    fn c<T: Clone>(t: &T) -> T {
        t.clone()
    }

    let tmpdir = tmpdir();

    let mut r = OO::new();
    r.read(true);
    let mut w = OO::new();
    w.write(true);
    let mut rw = OO::new();
    rw.read(true).write(true);
    let mut a = OO::new();
    a.append(true);
    let mut ra = OO::new();
    ra.read(true).append(true);

    #[cfg(windows)]
    let invalid_options = 87; // ERROR_INVALID_PARAMETER
    #[cfg(any(all(unix, not(target_os = "vxworks")), target_os = "wasi"))]
    let invalid_options = "Invalid argument";
    #[cfg(target_os = "vxworks")]
    let invalid_options = "invalid argument";

    // Test various combinations of creation modes and access modes.
    //
    // Allowed:
    // creation mode           | read  | write | read-write | append | read-append
    // | :-----------------------|:-----:|:-----:|:----------:|:------:|:
    // -----------:| not set (open existing) |   X   |   X   |     X      |   X
    // |      X      | create                  |       |   X   |     X      |
    // X    |      X      | truncate                |       |   X   |     X
    // |        |             | create and truncate     |       |   X   |     X
    // |        |             | create_new              |       |   X   |     X
    // |   X    |      X      |
    //
    // tested in reverse order, so 'create_new' creates the file, and 'open
    // existing' opens it.

    // write-only
    check!(tmpdir.open_with("a", c(&w).create_new(true)));
    check!(tmpdir.open_with("a", c(&w).create(true).truncate(true)));
    check!(tmpdir.open_with("a", c(&w).truncate(true)));
    check!(tmpdir.open_with("a", c(&w).create(true)));
    check!(tmpdir.open_with("a", &c(&w)));

    // read-only
    error!(
        tmpdir.open_with("b", c(&r).create_new(true)),
        invalid_options
    );
    error!(
        tmpdir.open_with("b", c(&r).create(true).truncate(true)),
        invalid_options
    );
    error!(tmpdir.open_with("b", c(&r).truncate(true)), invalid_options);
    error!(tmpdir.open_with("b", c(&r).create(true)), invalid_options);
    check!(tmpdir.open_with("a", &c(&r))); // try opening the file created with write_only

    // read-write
    check!(tmpdir.open_with("c", c(&rw).create_new(true)));
    check!(tmpdir.open_with("c", c(&rw).create(true).truncate(true)));
    check!(tmpdir.open_with("c", c(&rw).truncate(true)));
    check!(tmpdir.open_with("c", c(&rw).create(true)));
    check!(tmpdir.open_with("c", &c(&rw)));

    // append
    check!(tmpdir.open_with("d", c(&a).create_new(true)));
    error!(
        tmpdir.open_with("d", c(&a).create(true).truncate(true)),
        invalid_options
    );
    error!(tmpdir.open_with("d", c(&a).truncate(true)), invalid_options);
    check!(tmpdir.open_with("d", c(&a).create(true)));
    check!(tmpdir.open_with("d", &c(&a)));

    // read-append
    check!(tmpdir.open_with("e", c(&ra).create_new(true)));
    error!(
        tmpdir.open_with("e", c(&ra).create(true).truncate(true)),
        invalid_options
    );
    error!(
        tmpdir.open_with("e", c(&ra).truncate(true)),
        invalid_options
    );
    check!(tmpdir.open_with("e", c(&ra).create(true)));
    check!(tmpdir.open_with("e", &c(&ra)));

    // Test opening a file without setting an access mode
    let mut blank = OO::new();
    error!(tmpdir.open_with("f", blank.create(true)), invalid_options);

    // Test write works
    check!(check!(tmpdir.create("h")).write("foobar".as_bytes()));

    // Test write fails for read-only
    check!(tmpdir.open_with("h", &r));
    {
        let mut f = check!(tmpdir.open_with("h", &r));
        assert!(f.write("wut".as_bytes()).is_err());
    }

    // Test write overwrites
    {
        let mut f = check!(tmpdir.open_with("h", &c(&w)));
        check!(f.write("baz".as_bytes()));
    }
    {
        let mut f = check!(tmpdir.open_with("h", &c(&r)));
        let mut b = vec![0; 6];
        check!(f.read(&mut b));
        assert_eq!(b, "bazbar".as_bytes());
    }

    // Test truncate works
    {
        let mut f = check!(tmpdir.open_with("h", c(&w).truncate(true)));
        check!(f.write("foo".as_bytes()));
    }
    assert_eq!(check!(tmpdir.metadata("h")).len(), 3);

    // Test append works
    assert_eq!(check!(tmpdir.metadata("h")).len(), 3);
    {
        let mut f = check!(tmpdir.open_with("h", &c(&a)));
        check!(f.write("bar".as_bytes()));
    }
    assert_eq!(check!(tmpdir.metadata("h")).len(), 6);

    // Test .append(true) equals .write(true).append(true)
    {
        let mut f = check!(tmpdir.open_with("h", c(&w).append(true)));
        check!(f.write("baz".as_bytes()));
    }
    assert_eq!(check!(tmpdir.metadata("h")).len(), 9);
}

#[test]
fn _assert_send_sync() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<OpenOptions>();
}

#[test]
fn binary_file() {
    let mut bytes = [0; 1024];
    StdRng::from_os_rng().fill_bytes(&mut bytes);

    let tmpdir = tmpdir();

    check!(check!(tmpdir.create("test")).write(&bytes));
    let mut v = Vec::new();
    check!(check!(tmpdir.open("test")).read_to_end(&mut v));
    assert!(v == &bytes[..]);
}

#[test]
fn write_then_read() {
    let mut bytes = [0; 1024];
    StdRng::from_os_rng().fill_bytes(&mut bytes);

    let tmpdir = tmpdir();

    check!(tmpdir.write("test", &bytes[..]));
    let v = check!(tmpdir.read("test"));
    assert!(v == &bytes[..]);

    check!(tmpdir.write("not-utf8", &[0xFF]));
    error_contains!(
        tmpdir.read_to_string("not-utf8"),
        "stream did not contain valid UTF-8"
    );

    let s = "𐁁𐀓𐀠𐀴𐀍";
    check!(tmpdir.write("utf8", s.as_bytes()));
    let string = check!(tmpdir.read_to_string("utf8"));
    assert_eq!(string, s);
}

#[test]
fn file_try_clone() {
    let tmpdir = tmpdir();

    let mut f1 = check!(tmpdir.open_with(
        "test",
        OpenOptions::new().read(true).write(true).create(true)
    ));
    let mut f2 = check!(f1.try_clone());

    check!(f1.write_all(b"hello world"));
    check!(f1.seek(SeekFrom::Start(2)));

    let mut buf = vec![];
    check!(f2.read_to_end(&mut buf));
    assert_eq!(buf, b"llo world");
    drop(f2);

    check!(f1.write_all(b"!"));
}

#[test]
#[cfg(not(windows))]
fn unlink_readonly() {
    let tmpdir = tmpdir();
    let path = "file";
    check!(tmpdir.create(&path));
    let mut perm = check!(tmpdir.metadata(&path)).permissions();
    perm.set_readonly(true);
    check!(tmpdir.set_permissions(&path, perm));
    check!(tmpdir.remove_file(&path));
}

#[test]
fn mkdir_trailing_slash() {
    let tmpdir = tmpdir();
    let path = PathBuf::from("file");
    check!(tmpdir.create_dir_all(&path.join("a/")));
}

#[test]
fn canonicalize_works_simple() {
    let tmpdir = tmpdir();
    let file = Path::new("test");
    tmpdir.create(&file).unwrap();
    assert_eq!(tmpdir.canonicalize(&file).unwrap(), file);
}

#[test]
fn realpath_works() {
    let tmpdir = tmpdir();
    if !got_symlink_permission(&tmpdir) {
        return;
    };

    let file = PathBuf::from("test");
    let dir = PathBuf::from("test2");
    let link = dir.join("link");
    let linkdir = PathBuf::from("test3");

    tmpdir.create(&file).unwrap();
    tmpdir.create_dir(&dir).unwrap();
    symlink_file(Path::new("..").join(&file), &tmpdir, &link).unwrap();
    symlink_dir(&dir, &tmpdir, &linkdir).unwrap();

    assert!(tmpdir
        .symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());

    assert_eq!(tmpdir.canonicalize(".").unwrap(), PathBuf::from("."));
    assert_eq!(tmpdir.canonicalize(&file).unwrap(), file);
    assert_eq!(tmpdir.canonicalize(&link).unwrap(), file);
    assert_eq!(tmpdir.canonicalize(&linkdir).unwrap(), dir);
    assert_eq!(tmpdir.canonicalize(&linkdir.join("link")).unwrap(), file);
}

#[test]
fn realpath_works_tricky() {
    let tmpdir = tmpdir();
    if !got_symlink_permission(&tmpdir) {
        return;
    };

    let a = PathBuf::from("a");
    let b = a.join("b");
    let c = b.join("c");
    let d = a.join("d");
    let e = d.join("e");
    let f = a.join("f");

    tmpdir.create_dir_all(&b).unwrap();
    tmpdir.create_dir_all(&d).unwrap();
    tmpdir.create(&f).unwrap();
    if cfg!(not(windows)) {
        symlink_file("../d/e", &tmpdir, &c).unwrap();
        symlink_file("../f", &tmpdir, &e).unwrap();
    }
    if cfg!(windows) {
        symlink_file(r"..\d\e", &tmpdir, &c).unwrap();
        symlink_file(r"..\f", &tmpdir, &e).unwrap();
    }

    assert_eq!(tmpdir.canonicalize(&c).unwrap(), f);
    assert_eq!(tmpdir.canonicalize(&e).unwrap(), f);
}

#[test]
fn dir_entry_methods() {
    let tmpdir = tmpdir();

    tmpdir.create_dir_all("a").unwrap();
    tmpdir.create("b").unwrap();

    for file in tmpdir.read_dir(".").unwrap().map(|f| f.unwrap()) {
        let fname = file.file_name().unwrap();
        match fname.as_str() {
            "a" => {
                assert!(file.file_type().unwrap().is_dir());
                assert!(file.metadata().unwrap().is_dir());
            }
            "b" => {
                assert!(file.file_type().unwrap().is_file());
                assert!(file.metadata().unwrap().is_file());
            }
            f => panic!("unknown file name: {:?}", f),
        }
    }
}

#[test]
fn dir_entry_debug() {
    let tmpdir = tmpdir();
    tmpdir.create("b").unwrap();
    let mut read_dir = tmpdir.read_dir(".").unwrap();
    let dir_entry = read_dir.next().unwrap().unwrap();
    let actual = format!("{:?}", dir_entry);
    let expected = format!("DirEntry({:?})", dir_entry.file_name().unwrap());
    assert_eq!(actual, expected);
}

#[test]
fn read_dir_not_found() {
    let tmpdir = tmpdir();
    let res = tmpdir.read_dir("path/that/does/not/exist");
    assert_eq!(res.err().unwrap().kind(), ErrorKind::NotFound);
}

// See the comments on `create_dir_all_with_junctions` in tests/fs.rs about Windows.
#[test]
#[cfg_attr(windows, ignore)]
fn create_dir_all_with_junctions() {
    let tmpdir = tmpdir();
    let target = "target";

    let junction = PathBuf::from("junction");
    let b = junction.join("a/b");

    let link = PathBuf::from("link");
    let d = link.join("c/d");

    tmpdir.create_dir(&target).unwrap();

    check!(symlink_junction(&target, &tmpdir, &junction));
    check!(tmpdir.create_dir_all(&b));
    // the junction itself is not a directory, but `is_dir()` on a Path
    // follows links
    assert!(tmpdir.is_dir(junction));
    assert!(tmpdir.exists(b));

    if !got_symlink_permission(&tmpdir) {
        return;
    };
    check!(symlink_dir(&target, &tmpdir, &link));
    check!(tmpdir.create_dir_all(&d));
    assert!(tmpdir.is_dir(link));
    assert!(tmpdir.exists(d));
}

#[test]
fn metadata_access_times() {
    let tmpdir = tmpdir();

    let b = "b";
    tmpdir.create(&b).unwrap();

    let a = check!(tmpdir.metadata("."));
    let b = check!(tmpdir.metadata(&b));

    assert_eq!(check!(a.accessed()), check!(a.accessed()));
    assert_eq!(check!(a.modified()), check!(a.modified()));
    // This assert from std's testsuite is racy.
    //assert_eq!(check!(b.accessed()), check!(b.modified()));

    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
        check!(a.created());
        check!(b.created());
    }

    if cfg!(any(target_os = "android", target_os = "linux")) {
        // Not always available
        match (a.created(), b.created()) {
            (Ok(t1), Ok(t2)) => assert!(t1 <= t2),
            #[cfg(not(io_error_uncategorized))]
            (Err(e1), Err(e2))
                if e1.kind() == ErrorKind::Other && e2.kind() == ErrorKind::Other => {}
            #[cfg(io_error_uncategorized)]
            (Err(e1), Err(e2))
                if e1.kind() == ErrorKind::Uncategorized
                    && e2.kind() == ErrorKind::Uncategorized
                    || e1.kind() == ErrorKind::Unsupported
                        && e2.kind() == ErrorKind::Unsupported => {}
            (a, b) => panic!(
                "creation time must be always supported or not supported: {:?} {:?}",
                a, b,
            ),
        }
    }
}

/// Test creating hard links to symlinks.
#[test]
fn symlink_hard_link() {
    let tmpdir = tmpdir();
    if !got_symlink_permission(&tmpdir) {
        return;
    }
    if !able_to_not_follow_symlinks_while_hard_linking() {
        return;
    }

    // Create "file", a file.
    check!(tmpdir.create("file"));

    // Create "symlink", a symlink to "file".
    check!(symlink_file("file", &tmpdir, "symlink"));

    // Create "hard_link", a hard link to "symlink".
    check!(tmpdir.hard_link("symlink", &tmpdir, "hard_link"));

    // "hard_link" should appear as a symlink.
    assert!(check!(tmpdir.symlink_metadata("hard_link"))
        .file_type()
        .is_symlink());

    // We sould be able to open "file" via any of the above names.
    let _ = check!(tmpdir.open("file"));
    assert!(tmpdir.open("file.renamed").is_err());
    let _ = check!(tmpdir.open("symlink"));
    let _ = check!(tmpdir.open("hard_link"));

    // Rename "file" to "file.renamed".
    check!(tmpdir.rename("file", &tmpdir, "file.renamed"));

    // Now, the symlink and the hard link should be dangling.
    assert!(tmpdir.open("file").is_err());
    let _ = check!(tmpdir.open("file.renamed"));
    assert!(tmpdir.open("symlink").is_err());
    assert!(tmpdir.open("hard_link").is_err());

    // The symlink and the hard link should both still point to "file".
    assert!(tmpdir.read_link("file").is_err());
    assert!(tmpdir.read_link("file.renamed").is_err());
    assert_eq!(check!(tmpdir.read_link("symlink")), Path::new("file"));
    assert_eq!(check!(tmpdir.read_link("hard_link")), Path::new("file"));

    // Remove "file.renamed".
    check!(tmpdir.remove_file("file.renamed"));

    // Now, we can't open the file by any name.
    assert!(tmpdir.open("file").is_err());
    assert!(tmpdir.open("file.renamed").is_err());
    assert!(tmpdir.open("symlink").is_err());
    assert!(tmpdir.open("hard_link").is_err());

    // "hard_link" should still appear as a symlink.
    assert!(check!(tmpdir.symlink_metadata("hard_link"))
        .file_type()
        .is_symlink());
}

/// Ensure `fs::create_dir` works on Windows with longer paths.
#[test]
#[cfg(windows)]
fn create_dir_long_paths() {
    use std::ffi::OsStr;
    use std::iter;
    use std::os::windows::ffi::OsStrExt;
    const PATH_LEN: usize = 247;

    let tmpdir = tmpdir();
    let mut path = PathBuf::new();
    path.push("a");
    let mut path = path.into_os_string();

    let utf16_len = path.encode_wide().count();
    if utf16_len >= PATH_LEN {
        // Skip the test in the unlikely event the local user has a long temp directory
        // path. This should not affect CI.
        return;
    }
    // Increase the length of the path.
    path.extend(iter::repeat(OsStr::new("a")).take(PATH_LEN - utf16_len));

    // This should succeed.
    tmpdir
        .create_dir(&PathBuf::from_path_buf(path.clone().into()).unwrap())
        .unwrap();

    // This will fail if the path isn't converted to verbatim.
    path.push("a");
    tmpdir
        .create_dir(&PathBuf::from_path_buf(path.clone().into()).unwrap())
        .unwrap();

    // #90940: Ensure an empty path returns the "Not Found" error.
    let path = Path::new("");
    assert_eq!(
        tmpdir.canonicalize(path).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
}

/// Test creating files with invalid names and reading their parent
/// directories.
#[test]
fn test_invalid_utf8() {
    use camino::Utf8Path;
    #[cfg(not(windows))]
    use std::ffi::OsStr;
    #[cfg(windows)]
    use std::ffi::OsString;
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(target_os = "wasi")]
    use std::os::wasi::ffi::OsStrExt;
    #[cfg(windows)]
    use std::os::windows::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    #[cfg(not(windows))]
    let invalid_path = dir.path().join(OsStr::from_bytes(b"invalid\xffbyte"));
    #[cfg(windows)]
    let invalid_path = dir.path().join(OsString::from_wide(&[
        0x69, 0x6e, 0x76, 0x61, 0x6c, 0x69, 0x64, 0xd800, 0x62, 0x79, 0x74, 0x65,
    ]));

    // On some OS's, it's not possible to create files with invalid paths.
    // If it is, test that we do the right thing with them.
    if let Ok(_) = std::fs::File::create(invalid_path) {
        let cap_dir = cap_std::fs_utf8::Dir::open_ambient_dir(
            Utf8Path::from_path(dir.path()).unwrap(),
            cap_std::ambient_authority(),
        )
        .unwrap();

        for entry in cap_dir.entries().unwrap() {
            let entry = entry.unwrap();
            #[cfg(feature = "arf_strings")]
            {
                let name: String = entry.file_name().unwrap();
                let _ = cap_dir.open(name).unwrap();
            }
            #[cfg(not(feature = "arf_strings"))]
            {
                let _ = entry.file_name().unwrap_err();
            }
        }
    }
}

#[test]
fn from_cap_std() {
    let tmpdir = sys_common::io::tmpdir();
    let dir = "d1/d2";
    check!(tmpdir.create_dir_all(dir));
    let d1_entry = tmpdir.entries_utf8().unwrap().next().unwrap().unwrap();
    assert_eq!(d1_entry.file_name().unwrap(), "d1");

    let d1 = tmpdir.open_dir_utf8("d1").unwrap();
    let d2_entry = d1.entries().unwrap().next().unwrap().unwrap();
    assert_eq!(d2_entry.file_name().unwrap(), "d2");
}
