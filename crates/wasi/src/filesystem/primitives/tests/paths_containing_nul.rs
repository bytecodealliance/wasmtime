// This test module derived from Rust's src/test/ui/paths-containing-nul.rs
// at revision 108e90ca78f052c0c1c49c42a22c85620be19712.

// run-pass

#![allow(deprecated)]
// ignore-cloudabi no files or I/O
// ignore-wasm32-bare no files or I/O
// ignore-emscripten no files
// ignore-sgx no files

use super::sys_common::io::tmpdir;
use crate::filesystem::primitives::{
    DirOptions, FollowSymlinks, OpenOptions, create_dir, hard_link, open, open_ambient_dir,
    read_link, remove_dir, remove_file, rename, stat,
};
use std::io;
use std::path::Path;

fn assert_invalid_input<T>(on: &str, result: io::Result<T>) {
    fn inner(on: &str, result: io::Result<()>) {
        match result {
            Ok(()) => panic!("{on} didn't return an error on a path with NUL"),
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
    let dir = open_ambient_dir(tmpdir.path()).unwrap();
    let nul = Path::new("\0");

    assert_invalid_input("open", open(&dir, nul, OpenOptions::new().read(true)));
    assert_invalid_input(
        "create",
        open(
            &dir,
            nul,
            OpenOptions::new().write(true).create(true).truncate(true),
        ),
    );
    assert_invalid_input("remove_file", remove_file(&dir, nul));
    assert_invalid_input("metadata", stat(&dir, nul, FollowSymlinks::Yes));
    assert_invalid_input("symlink_metadata", stat(&dir, nul, FollowSymlinks::No));

    // Create a file inside the sandbox.
    let dummy_file = Path::new("dummy_file");
    open(
        &dir,
        dummy_file,
        OpenOptions::new().write(true).create(true).truncate(true),
    )
    .expect("creating dummy_file");

    assert_invalid_input("rename1", rename(&dir, nul, &dir, Path::new("a")));
    assert_invalid_input("rename2", rename(&dir, dummy_file, &dir, nul));
    assert_invalid_input("hard_link1", hard_link(&dir, nul, &dir, Path::new("a")));
    assert_invalid_input("hard_link2", hard_link(&dir, dummy_file, &dir, nul));
    assert_invalid_input("read_link", read_link(&dir, nul));
    assert_invalid_input("create_dir", create_dir(&dir, nul, &DirOptions::new()));
    assert_invalid_input("remove_dir", remove_dir(&dir, nul));
}
