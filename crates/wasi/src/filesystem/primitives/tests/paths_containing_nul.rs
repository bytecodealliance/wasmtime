// This test module derived from Rust's src/test/ui/paths-containing-nul.rs
// at revision 108e90ca78f052c0c1c49c42a22c85620be19712.

// run-pass

#![allow(deprecated)]
// ignore-cloudabi no files or I/O
// ignore-wasm32-bare no files or I/O
// ignore-emscripten no files
// ignore-sgx no files

mod sys_common;

use std::io;
use sys_common::io::tmpdir;

fn assert_invalid_input<T>(on: &str, result: io::Result<T>) {
    fn inner(on: &str, result: io::Result<()>) {
        match result {
            Ok(()) => panic!("{} didn't return an error on a path with NUL", on),
            Err(_e) => {
                // TODO: Re-enable this assertion once the `io_error_more`
                // feature is available.
                /*
                assert_eq!(
                    e.kind(),
                    io::ErrorKind::InvalidInput || io::ErrorKind::InvalidFilename,
                    "{} returned a strange {:?} on a path with NUL",
                    on,
                    e
                );
                */
            }
        }
    }
    inner(on, result.map(drop))
}

#[test]
fn paths_containing_nul() {
    let tmpdir = tmpdir();

    assert_invalid_input("File::open", tmpdir.open("\0"));
    assert_invalid_input("File::create", tmpdir.create("\0"));
    assert_invalid_input("remove_file", tmpdir.remove_file("\0"));
    assert_invalid_input("metadata", tmpdir.metadata("\0"));
    assert_invalid_input("symlink_metadata", tmpdir.symlink_metadata("\0"));

    // Create a file inside the sandbox.
    let dummy_file = "dummy_file";
    tmpdir.create(dummy_file).expect("creating dummy_file");

    assert_invalid_input("rename1", tmpdir.rename("\0", &tmpdir, "a"));
    assert_invalid_input("rename2", tmpdir.rename(&dummy_file, &tmpdir, "\0"));
    assert_invalid_input("copy1", tmpdir.copy("\0", &tmpdir, "a"));
    assert_invalid_input("copy2", tmpdir.copy(&dummy_file, &tmpdir, "\0"));
    assert_invalid_input("hard_link1", tmpdir.hard_link("\0", &tmpdir, "a"));
    assert_invalid_input("hard_link2", tmpdir.hard_link(&dummy_file, &tmpdir, "\0"));
    //fixmeassert_invalid_input("soft_link1", tmpdir.soft_link("\0", &tmpdir,
    // "a")); fixmeassert_invalid_input("soft_link2",
    // tmpdir.soft_link(&dummy_file, &tmpdir, "\0"));
    assert_invalid_input("read_link", tmpdir.read_link("\0"));
    assert_invalid_input("canonicalize", tmpdir.canonicalize("\0"));
    assert_invalid_input("create_dir", tmpdir.create_dir("\0"));
    assert_invalid_input("create_dir_all", tmpdir.create_dir_all("\0"));
    assert_invalid_input("remove_dir", tmpdir.remove_dir("\0"));
    assert_invalid_input("remove_dir_all", tmpdir.remove_dir_all("\0"));
    assert_invalid_input("read_dir", tmpdir.read_dir("\0"));
    // `Dir` has no `set_permissions` function.
}
