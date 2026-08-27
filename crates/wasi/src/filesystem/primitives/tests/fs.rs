// This file is derived from Rust's library/std/src/fs/tests.rs at revision
// e4b1d5841494d6eb7f4944c91a057e16b0f0a9ea.

use super::helpers as h;
use super::sys_common::io::tmpdir;
use super::sys_common::symlink_junction;
use crate::filesystem::primitives as p;
use rand::Rng;
use std::fs::File;
use std::io::prelude::*;
use std::io::{ErrorKind, SeekFrom};
#[cfg(unix)]
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::str;
use std::thread;

// Several test fail on windows if the user does not have permission to
// create symlinks (the `SeCreateSymbolicLinkPrivilege`). Instead of
// disabling these test on Windows, use this function to test whether we
// have permission, and return otherwise. This way, we still don't run these
// tests most of the time, but at least we do if the user has the right
// permissions.
pub fn got_symlink_permission(tmpdir: &File) -> bool {
    if cfg!(unix) {
        return true;
    }
    let link = "some_hopefully_unique_link_name";

    match h::symlink_file(tmpdir, r"nonexisting_target", link) {
        // ERROR_PRIVILEGE_NOT_HELD = 1314
        Err(ref err) if err.raw_os_error() == Some(1314) => false,
        Ok(_) | Err(_) => true,
    }
}

fn able_to_not_follow_symlinks_while_hard_linking() -> bool {
    return true;
}

#[test]
fn open_directory_with_truncate_is_error() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let mut options = p::OpenOptions::new();
    // The `maybe_dir` part of this test is gone along with the option itself.
    options.truncate(true).read(true).write(true);
    p::create_dir(&start, Path::new("test"), &p::DirOptions::new()).unwrap();
    assert!(p::open(&start, Path::new("test"), &options).is_err());
}

#[test]
fn dir_entry_methods() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    h::create_dir_all(&start, "a").unwrap();
    h::create(&start, "b").unwrap();

    // `DirEntry::file_type` is gone; the metadata checks still cover this.
    for file in h::read_dir(&start, ".").unwrap().map(|f| f.unwrap()) {
        let fname = file.file_name();
        match fname.to_str() {
            Some("a") => {
                assert!(file.metadata().unwrap().is_dir());
            }
            Some("b") => {
                assert!(file.metadata().unwrap().file_type().is_file());
            }
            f => panic!("unknown file name: {:?}", f),
        }
    }
}

#[test]
fn open_flavors() {
    use crate::filesystem::primitives::OpenOptions as OO;
    fn c<T: Clone>(t: &T) -> T {
        t.clone()
    }

    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let mut r = OO::new();
    r.read(true);
    let mut w = OO::new();
    w.write(true);
    let mut rw = OO::new();
    rw.read(true).write(true);

    #[cfg(windows)]
    let invalid_options = 87; // ERROR_INVALID_PARAMETER
    #[cfg(any(all(unix, not(target_os = "vxworks")), target_os = "wasi"))]
    let invalid_options = "Invalid argument";
    #[cfg(target_os = "vxworks")]
    let invalid_options = "invalid argument";

    // Test various combinations of creation modes and access modes.
    //
    // Allowed:
    // creation mode           | read  | write | read-write |
    // | :-----------------------|:-----:|:-----:|:----------:|
    // not set (open existing) |   X   |   X   |     X      |
    // create                  |       |   X   |     X      |
    // truncate                |       |   X   |     X      |
    // create and truncate     |       |   X   |     X      |
    // create_new              |       |   X   |     X      |
    //
    // tested in reverse order, so 'create_new' creates the file, and 'open
    // existing' opens it.
    //
    // The append and read-append rows are not covered: `OpenOptions::append`
    // was dropped in this vendoring.

    // write-only
    check!(p::open(&start, Path::new("a"), c(&w).create_new(true)));
    check!(p::open(
        &start,
        Path::new("a"),
        c(&w).create(true).truncate(true)
    ));
    check!(p::open(&start, Path::new("a"), c(&w).truncate(true)));
    check!(p::open(&start, Path::new("a"), c(&w).create(true)));
    check!(p::open(&start, Path::new("a"), &c(&w)));

    // read-only
    error!(
        p::open(&start, Path::new("b"), c(&r).create_new(true)),
        invalid_options
    );
    error!(
        p::open(&start, Path::new("b"), c(&r).create(true).truncate(true)),
        invalid_options
    );
    error!(
        p::open(&start, Path::new("b"), c(&r).truncate(true)),
        invalid_options
    );
    error!(
        p::open(&start, Path::new("b"), c(&r).create(true)),
        invalid_options
    );
    check!(p::open(&start, Path::new("a"), &c(&r))); // try opening the file created with write_only

    // read-write
    check!(p::open(&start, Path::new("c"), c(&rw).create_new(true)));
    check!(p::open(
        &start,
        Path::new("c"),
        c(&rw).create(true).truncate(true)
    ));
    check!(p::open(&start, Path::new("c"), c(&rw).truncate(true)));
    check!(p::open(&start, Path::new("c"), c(&rw).create(true)));
    check!(p::open(&start, Path::new("c"), &c(&rw)));

    // Test opening a file without setting an access mode
    let mut blank = OO::new();
    error!(
        p::open(&start, Path::new("f"), blank.create(true)),
        invalid_options
    );

    // Test write works
    check!(check!(h::create(&start, "h")).write("foobar".as_bytes()));

    // Test write fails for read-only
    check!(p::open(&start, Path::new("h"), &r));
    {
        let mut f = check!(p::open(&start, Path::new("h"), &r));
        assert!(f.write("wut".as_bytes()).is_err());
    }

    // Test write overwrites
    {
        let mut f = check!(p::open(&start, Path::new("h"), &c(&w)));
        check!(f.write("baz".as_bytes()));
    }
    {
        let mut f = check!(p::open(&start, Path::new("h"), &c(&r)));
        let mut b = vec![0; 6];
        check!(f.read(&mut b));
        assert_eq!(b, "bazbar".as_bytes());
    }

    // Test truncate works
    {
        let mut f = check!(p::open(&start, Path::new("h"), c(&w).truncate(true)));
        check!(f.write("foo".as_bytes()));
    }
    assert_eq!(check!(h::metadata(&start, "h")).len(), 3);
}

#[test]
fn file_test_io_smoke_test() {
    let message = "it's alright. have a good time";
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let filename = "file_rt_io_file_test.txt";
    {
        let mut write_stream = check!(h::create(&start, filename));
        check!(write_stream.write(message.as_bytes()));
    }
    {
        let mut read_stream = check!(h::open(&start, filename));
        let mut read_buf = [0; 1028];
        let read_str = match check!(read_stream.read(&mut read_buf)) {
            0 => panic!("shouldn't happen"),
            n => str::from_utf8(&read_buf[..n]).unwrap().to_string(),
        };
        assert_eq!(read_str, message);
    }
    check!(p::remove_file(&start, Path::new(filename)));
}

#[test]
fn invalid_path_raises() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let filename = "file_that_does_not_exist.txt";
    let result = h::open(&start, filename);

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
    let start = h::dir_of(&tmpdir);
    let filename = "file_another_file_that_does_not_exist.txt";

    let result = p::remove_file(&start, Path::new(filename));

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
    let start = h::dir_of(&tmpdir);
    let filename = "file_rt_io_file_test_positional.txt";
    {
        let mut rw_stream = check!(h::create(&start, filename));
        check!(rw_stream.write(message.as_bytes()));
    }
    {
        let mut read_stream = check!(h::open(&start, filename));
        {
            let read_buf = &mut read_mem[0..4];
            check!(read_stream.read(read_buf));
        }
        {
            let read_buf = &mut read_mem[4..8];
            check!(read_stream.read(read_buf));
        }
    }
    check!(p::remove_file(&start, Path::new(filename)));
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
    let start = h::dir_of(&tmpdir);
    let filename = "file_rt_io_file_test_seeking.txt";
    {
        let mut rw_stream = check!(h::create(&start, filename));
        check!(rw_stream.write(message.as_bytes()));
    }
    {
        let mut read_stream = check!(h::open(&start, filename));
        check!(read_stream.seek(SeekFrom::Start(set_cursor)));
        tell_pos_pre_read = check!(read_stream.seek(SeekFrom::Current(0)));
        check!(read_stream.read(&mut read_mem));
        tell_pos_post_read = check!(read_stream.seek(SeekFrom::Current(0)));
    }
    check!(p::remove_file(&start, Path::new(filename)));
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
    let start = h::dir_of(&tmpdir);
    let filename = "file_rt_io_file_test_seek_and_write.txt";
    {
        let mut rw_stream = check!(h::create(&start, filename));
        check!(rw_stream.write(initial_msg.as_bytes()));
        check!(rw_stream.seek(SeekFrom::Start(seek_idx)));
        check!(rw_stream.write(overwrite_msg.as_bytes()));
    }
    {
        let mut read_stream = check!(h::open(&start, filename));
        check!(read_stream.read(&mut read_mem));
    }
    check!(p::remove_file(&start, Path::new(filename)));
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
    let start = h::dir_of(&tmpdir);
    let filename = "file_rt_io_file_test_seek_shakedown.txt";
    {
        let mut rw_stream = check!(h::create(&start, filename));
        check!(rw_stream.write(initial_msg.as_bytes()));
    }
    {
        let mut read_stream = check!(h::open(&start, filename));

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
    check!(p::remove_file(&start, Path::new(filename)));
}

#[test]
fn file_test_io_eof() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let filename = "file_rt_io_file_test_eof.txt";
    let mut buf = [0; 256];
    {
        let oo = p::OpenOptions::new()
            .create_new(true)
            .write(true)
            .read(true)
            .clone();
        let mut rw = check!(p::open(&start, Path::new(filename), &oo));
        assert_eq!(check!(rw.read(&mut buf)), 0);
        assert_eq!(check!(rw.read(&mut buf)), 0);
    }
    check!(p::remove_file(&start, Path::new(filename)));
}

#[test]
#[cfg(unix)]
fn file_test_io_read_write_at() {
    let tmpdir = tmpdir();

    let start = h::dir_of(&tmpdir);
    let filename = "file_rt_io_file_test_read_write_at.txt";
    let mut buf = [0; 256];
    let write1 = "asdf";
    let write2 = "qwer-";
    let write3 = "-zxcv";
    let content = "qwer-asdf-zxcv";
    {
        let oo = p::OpenOptions::new()
            .create_new(true)
            .write(true)
            .read(true)
            .clone();
        let mut rw = check!(p::open(&start, Path::new(filename), &oo));
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
        let mut read = check!(h::open(&start, filename));
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
    check!(p::remove_file(&start, Path::new(filename)));
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
#[test]
#[cfg(windows)]
fn file_test_io_seek_read_write() {
    let tmpdir = tmpdir();

    let start = h::dir_of(&tmpdir);
    let filename = "file_rt_io_file_test_seek_read_write.txt";
    let mut buf = [0; 256];
    let write1 = "asdf";
    let write2 = "qwer-";
    let write3 = "-zxcv";
    let content = "qwer-asdf-zxcv";
    {
        let oo = p::OpenOptions::new()
            .create_new(true)
            .write(true)
            .read(true)
            .clone();
        let mut rw = check!(p::open(&start, Path::new(filename), &oo));
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
        let mut read = check!(h::open(&start, filename));
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
    check!(p::remove_file(&start, Path::new(filename)));
}

#[test]
fn file_test_stat_is_correct_on_is_file() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let filename = "file_stat_correct_on_is_file.txt";
    {
        let mut opts = p::OpenOptions::new();
        let mut fs = check!(p::open(
            &start,
            Path::new(filename),
            opts.read(true).write(true).create(true)
        ));
        let msg = "hw";
        fs.write(msg.as_bytes()).unwrap();

        let fstat_res = check!(fs.metadata());
        assert!(fstat_res.is_file());
    }
    let stat_res_fn = check!(h::metadata(&start, filename));
    assert!(stat_res_fn.file_type().is_file());
    check!(p::remove_file(&start, Path::new(filename)));
}

#[test]
fn file_test_stat_is_correct_on_is_dir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let filename = "file_stat_correct_on_is_dir";
    check!(h::create_dir(&start, filename));
    let stat_res_fn = check!(h::metadata(&start, filename));
    assert!(stat_res_fn.is_dir());
    check!(p::remove_dir(&start, Path::new(filename)));
}

#[test]
fn file_test_fileinfo_false_when_checking_is_file_on_a_directory() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let dir = "fileinfo_false_on_dir";
    check!(h::create_dir(&start, dir));
    assert!(!h::is_file(&start, dir));
    check!(p::remove_dir(&start, Path::new(dir)));
}

#[test]
fn file_test_fileinfo_check_exists_before_and_after_file_creation() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let file = "fileinfo_check_exists_b_and_a.txt";
    check!(check!(h::create(&start, file)).write(b"foo"));
    assert!(h::exists(&start, file));
    check!(p::remove_file(&start, Path::new(file)));
    assert!(!h::exists(&start, file));
}

#[test]
fn file_test_directoryinfo_check_exists_before_and_after_mkdir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let dir = "before_and_after_dir";
    assert!(!h::exists(&start, dir));
    check!(h::create_dir(&start, dir));
    assert!(h::exists(&start, dir));
    assert!(h::is_dir(&start, dir));
    check!(p::remove_dir(&start, Path::new(dir)));
    assert!(!h::exists(&start, dir));
}

#[test]
fn file_test_directoryinfo_readdir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let dir = "di_readdir";
    check!(h::create_dir(&start, dir));
    let prefix = "foo";
    for n in 0..3 {
        let f = format!("{}.txt", n);
        let mut w = check!(h::create(&start, &f));
        let msg_str = format!("{}{}", prefix, n.to_string());
        let msg = msg_str.as_bytes();
        check!(w.write(msg));
    }
    let files = check!(h::read_dir(&start, dir));
    let mut mem = [0; 4];
    for f in files {
        let f = f.unwrap().file_name();
        {
            check!(check!(h::open(&start, &f)).read(&mut mem));
            let read_str = str::from_utf8(&mem).unwrap();
            let expected = format!("{}{}", prefix, f.to_str().unwrap());
            assert_eq!(expected, read_str);
        }
        check!(p::remove_file(&start, Path::new(&f)));
    }
    check!(p::remove_dir(&start, Path::new(dir)));
}

#[test]
fn file_create_new_already_exists_error() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let file = "file_create_new_error_exists";
    check!(h::create(&start, file));
    let e = p::open(
        &start,
        Path::new(file),
        p::OpenOptions::new().write(true).create_new(true),
    )
    .unwrap_err();
    assert_eq!(e.kind(), ErrorKind::AlreadyExists);
}

#[test]
fn mkdir_path_already_exists_error() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let dir = "mkdir_error_twice";
    check!(h::create_dir(&start, dir));
    let e = h::create_dir(&start, dir).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::AlreadyExists);
}

#[test]
fn recursive_mkdir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let dir = "d1/d2";
    check!(h::create_dir_all(&start, dir));
    assert!(h::is_dir(&start, dir));
}

#[test]
fn concurrent_recursive_mkdir() {
    for _ in 0..100 {
        let tmpdir = tmpdir();
        let start = h::dir_of(&tmpdir);
        let mut name = PathBuf::from("a");
        for _ in 0..40 {
            name = name.join("a");
        }
        let mut join = vec![];
        for _ in 0..8 {
            let dir = check!(start.try_clone());
            let name = name.clone();
            join.push(thread::spawn(move || {
                check!(h::create_dir_all(&dir, &name));
            }))
        }

        // No `Display` on result of `join()`
        join.drain(..).map(|join| join.join().unwrap()).count();
    }
}

#[test]
fn recursive_mkdir_slash() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    error_contains!(
        h::create_dir_all(&start, Path::new("/")),
        "a path led outside of the filesystem"
    );
}

#[test]
fn recursive_mkdir_dot() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create_dir_all(&start, Path::new(".")));
}

#[test]
fn recursive_mkdir_empty() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create_dir_all(&start, Path::new("")));
}

#[test]
fn unicode_path_is_dir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    assert!(h::is_dir(&start, Path::new(".")));
    assert!(!h::is_dir(&start, Path::new("test/stdtest/fs.rs")));

    let mut dirpath = PathBuf::new();
    dirpath.push("test-가一ー你好");
    check!(h::create_dir(&start, &dirpath));
    assert!(h::is_dir(&start, &dirpath));

    let mut filepath = dirpath;
    filepath.push("unicode-file-\u{ac00}\u{4e00}\u{30fc}\u{4f60}\u{597d}.rs");
    check!(h::create(&start, &filepath)); // ignore return; touch only
    assert!(!h::is_dir(&start, &filepath));
    assert!(h::exists(&start, filepath));
}

#[test]
fn unicode_path_exists() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    assert!(h::exists(&start, Path::new(".")));
    assert!(!h::exists(&start, Path::new("test/nonexistent-bogus-path")));

    let unicode = PathBuf::new();
    let unicode = unicode.join("test-각丁ー再见");
    check!(h::create_dir(&start, &unicode));
    assert!(h::exists(&start, unicode));
    assert!(!h::exists(
        &start,
        Path::new("test/unicode-bogus-path-각丁ー再见")
    ));
}

#[test]
fn symlinks_work() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    if !got_symlink_permission(&start) {
        return;
    };

    let input = "in.txt";
    let out = "out.txt";

    check!(check!(h::create(&start, &input)).write("foobar".as_bytes()));
    check!(h::symlink_file(&start, &input, &out));
    assert!(
        check!(h::symlink_metadata(&start, out))
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        check!(h::metadata(&start, &out)).len(),
        check!(h::metadata(&start, &input)).len()
    );
    let mut v = Vec::new();
    check!(check!(h::open(&start, &out)).read_to_end(&mut v));
    assert_eq!(v, b"foobar".to_vec());
}

#[test]
fn symlink_noexist() {
    // Symlinks can point to things that don't exist
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    if !got_symlink_permission(&start) {
        return;
    };

    // Use a relative path for testing. Symlinks get normalized by Windows,
    // so we might not get the same path back for absolute paths
    check!(h::symlink_file(&start, &"foo", "bar"));
    assert_eq!(
        check!(p::read_link(&start, Path::new("bar")))
            .to_str()
            .unwrap(),
        "foo"
    );
}

#[test]
fn read_link() {
    if cfg!(windows) {
        // directory symlink
        let root = h::open_ambient_dir(r"C:\").unwrap();
        error_contains!(
            p::read_link(&root, Path::new(r"Users\All Users")),
            "a path led outside of the filesystem"
        );
        // junction
        error_contains!(
            p::read_link(&root, Path::new(r"Users\Default User")),
            "a path led outside of the filesystem"
        );
        // junction with special permissions
        error_contains!(
            p::read_link(&root, Path::new(r"Documents and Settings\")),
            "a path led outside of the filesystem"
        );
    }
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let link = "link";
    if !got_symlink_permission(&start) {
        return;
    };
    check!(h::symlink_file(&start, &"foo", &link));
    assert_eq!(
        check!(p::read_link(&start, Path::new(&link)))
            .to_str()
            .unwrap(),
        "foo"
    );
}

#[test]
fn readlink_not_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    match p::read_link(&start, Path::new(".")) {
        Ok(..) => panic!("wanted a failure"),
        Err(..) => {}
    }
}

#[cfg(not(windows))]
#[test]
fn read_link_contents() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let link = "link";
    if !got_symlink_permission(&start) {
        return;
    };
    check!(h::symlink_file(&start, &"foo", &link));
    assert_eq!(
        check!(super::super::read_link::read_link_contents(
            &start,
            Path::new(link)
        ))
        .to_str()
        .unwrap(),
        "foo"
    );
}

#[cfg(not(windows))]
#[test]
fn read_link_contents_absolute() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let link = "link";
    if !got_symlink_permission(&start) {
        return;
    };
    check!(std::os::unix::fs::symlink("/foo", tmpdir.path().join(link)));
    assert_eq!(
        check!(super::super::read_link::read_link_contents(
            &start,
            Path::new(link)
        ))
        .to_str()
        .unwrap(),
        "/foo"
    );
}

#[test]
fn links_work() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let input = "in.txt";
    let out = "out.txt";

    check!(check!(h::create(&start, &input)).write("foobar".as_bytes()));
    check!(p::hard_link(
        &start,
        Path::new(&input),
        &start,
        Path::new(&out)
    ));
    assert_eq!(
        check!(h::metadata(&start, &out)).len(),
        check!(h::metadata(&start, &input)).len()
    );
    assert_eq!(
        check!(h::metadata(&start, &out)).len(),
        check!(h::metadata(&start, input)).len()
    );
    let mut v = Vec::new();
    check!(check!(h::open(&start, &out)).read_to_end(&mut v));
    assert_eq!(v, b"foobar".to_vec());

    // can't link to yourself
    match p::hard_link(&start, Path::new(&input), &start, Path::new(&input)) {
        Ok(..) => panic!("wanted a failure"),
        Err(..) => {}
    }
    // can't link to something that doesn't exist
    match p::hard_link(&start, Path::new("foo"), &start, Path::new("bar")) {
        Ok(..) => panic!("wanted a failure"),
        Err(..) => {}
    }
}

#[test]
fn sync_doesnt_kill_anything() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let path = "in.txt";

    let mut file = check!(h::create(&start, &path));
    check!(file.sync_all());
    check!(file.sync_data());
    check!(file.write(b"foo"));
    check!(file.sync_all());
    check!(file.sync_data());
}

#[test]
fn truncate_works() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let path = "in.txt";

    let mut file = check!(h::create(&start, &path));
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
    check!(check!(h::open(&start, &path)).read_to_end(&mut v));
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
    check!(check!(h::open(&start, &path)).read_to_end(&mut v));
    assert_eq!(v, b"fo\0\0\0\0wut".to_vec());
}

#[test]
fn _assert_send_sync() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<p::OpenOptions>();
}

#[test]
fn binary_file() {
    let mut bytes = [0; 1024];
    rand::rng().fill_bytes(&mut bytes);

    let tmpdir = tmpdir();

    let start = h::dir_of(&tmpdir);

    check!(check!(h::create(&start, "test")).write(&bytes));
    let mut v = Vec::new();
    check!(check!(h::open(&start, "test")).read_to_end(&mut v));
    assert!(v == &bytes[..]);
}

#[test]
fn write_then_read() {
    let mut bytes = [0; 1024];
    rand::rng().fill_bytes(&mut bytes);

    let tmpdir = tmpdir();

    let start = h::dir_of(&tmpdir);

    check!(h::write(&start, "test", &bytes[..]));
    let v = check!(h::read(&start, "test"));
    assert!(v == &bytes[..]);

    check!(h::write(&start, "not-utf8", &[0xFF]));
    error_contains!(
        h::read_to_string(&start, "not-utf8"),
        "stream did not contain valid UTF-8"
    );

    let s = "𐁁𐀓𐀠𐀴𐀍";
    check!(h::write(&start, "utf8", s.as_bytes()));
    let string = check!(h::read_to_string(&start, "utf8"));
    assert_eq!(string, s);
}

#[test]
fn file_try_clone() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let mut f1 = check!(p::open(
        &start,
        Path::new("test"),
        p::OpenOptions::new().read(true).write(true).create(true)
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
fn mkdir_trailing_slash() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let path = PathBuf::from("file");
    check!(h::create_dir_all(&start, &path.join("a/")));
}

#[test]
fn dir_entry_debug() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    h::create(&start, "b").unwrap();
    let mut read_dir = h::read_dir(&start, ".").unwrap();
    let dir_entry = read_dir.next().unwrap().unwrap();
    let actual = format!("{:?}", dir_entry);
    let expected = format!("DirEntry({:?})", dir_entry.file_name());
    assert_eq!(actual, expected);
}

#[test]
fn read_dir_not_found() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let res = h::read_dir(&start, "path/that/does/not/exist");
    assert_eq!(res.err().unwrap().kind(), ErrorKind::NotFound);
}

// On Windows, `symlink_junction` somehow creates a symlink where `read_link`
// returns a relative path prefixed with "\\\\?\\", which `std::path::Path`
// parses as a `Prefix`, making cap-std think it's an absolute path and
// therefore a sandbox escape attempt. This only seems to happen with
// `symlink_junction`, and not symlinks created with standard library APIs. I
// don't know what the right thing to do here is. For now, disable these tests.
#[cfg_attr(windows, ignore)]
#[test]
fn create_dir_all_with_junctions() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let target = "target";

    let junction = PathBuf::from("junction");
    let b = junction.join("a/b");

    let link = PathBuf::from("link");
    let d = link.join("c/d");

    h::create_dir(&start, &target).unwrap();

    check!(symlink_junction(&target, &start, &junction));
    check!(h::create_dir_all(&start, &b));
    // the junction itself is not a directory, but `is_dir()` on a Path
    // follows links
    assert!(h::is_dir(&start, junction));
    assert!(h::exists(&start, b));

    if !got_symlink_permission(&start) {
        return;
    };
    check!(h::symlink_dir(&start, &target, &link));
    check!(h::create_dir_all(&start, &d));
    assert!(h::is_dir(&start, link));
    assert!(h::exists(&start, d));
}

#[test]
fn metadata_access_times() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let b = "b";
    h::create(&start, &b).unwrap();

    let a = check!(h::metadata(&start, "."));
    let b = check!(h::metadata(&start, &b));

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
            (Err(e1), Err(e2))
                if e1.kind() == ErrorKind::Other && e2.kind() == ErrorKind::Other => {}
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
    let start = h::dir_of(&tmpdir);
    if !got_symlink_permission(&start) {
        return;
    }
    if !able_to_not_follow_symlinks_while_hard_linking() {
        return;
    }

    // Create "file", a file.
    check!(h::create(&start, "file"));

    // Create "symlink", a symlink to "file".
    check!(h::symlink_file(&start, "file", "symlink"));

    // Create "hard_link", a hard link to "symlink".
    check!(p::hard_link(
        &start,
        Path::new("symlink"),
        &start,
        Path::new("hard_link")
    ));

    // "hard_link" should appear as a symlink.
    assert!(
        check!(h::symlink_metadata(&start, "hard_link"))
            .file_type()
            .is_symlink()
    );

    // We sould be able to open "file" via any of the above names.
    let _ = check!(h::open(&start, "file"));
    assert!(h::open(&start, "file.renamed").is_err());
    let _ = check!(h::open(&start, "symlink"));
    let _ = check!(h::open(&start, "hard_link"));

    // Rename "file" to "file.renamed".
    check!(p::rename(
        &start,
        Path::new("file"),
        &start,
        Path::new("file.renamed")
    ));

    // Now, the symlink and the hard link should be dangling.
    assert!(h::open(&start, "file").is_err());
    let _ = check!(h::open(&start, "file.renamed"));
    assert!(h::open(&start, "symlink").is_err());
    assert!(h::open(&start, "hard_link").is_err());

    // The symlink and the hard link should both still point to "file".
    assert!(p::read_link(&start, Path::new("file")).is_err());
    assert!(p::read_link(&start, Path::new("file.renamed")).is_err());
    assert_eq!(
        check!(p::read_link(&start, Path::new("symlink"))),
        Path::new("file")
    );
    assert_eq!(
        check!(p::read_link(&start, Path::new("hard_link"))),
        Path::new("file")
    );

    // Remove "file.renamed".
    check!(p::remove_file(&start, Path::new("file.renamed")));

    // Now, we can't open the file by any name.
    assert!(h::open(&start, "file").is_err());
    assert!(h::open(&start, "file.renamed").is_err());
    assert!(h::open(&start, "symlink").is_err());
    assert!(h::open(&start, "hard_link").is_err());

    // "hard_link" should still appear as a symlink.
    assert!(
        check!(h::symlink_metadata(&start, "hard_link"))
            .file_type()
            .is_symlink()
    );
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

    let start = h::dir_of(&tmpdir);
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
    h::create_dir(&start, &path).unwrap();

    // This will fail if the path isn't converted to verbatim.
    path.push("a");
    h::create_dir(&start, &path).unwrap();
}
