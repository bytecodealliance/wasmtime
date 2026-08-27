use super::helpers as h;
use super::sys_common::io::tmpdir;
use super::sys_common::symlink_supported;
use crate::filesystem::primitives as p;

use std::path::Path;

#[test]
fn basic_symlinks() {
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();

    let dir = h::dir_of(&tmpdir);

    check!(h::create(&dir, "file"));
    check!(h::create_dir(&dir, "dir"));
    assert!(check!(h::metadata(&dir, "file")).file_type().is_file());
    assert!(!check!(h::metadata(&dir, "file")).is_dir());
    assert!(check!(h::metadata(&dir, "dir")).is_dir());
    assert!(!check!(h::metadata(&dir, "dir")).file_type().is_file());
    assert!(!check!(h::metadata(&dir, "file")).file_type().is_symlink());
    assert!(!check!(h::metadata(&dir, "dir")).file_type().is_symlink());

    check!(h::symlink_file(&dir, "file", "file_symlink_file"));
    check!(h::symlink_dir(&dir, "dir", "dir_symlink_dir"));
    check!(h::symlink(&dir, "file", "file_symlink"));
    check!(h::symlink(&dir, "dir", "dir_symlink"));

    assert!(
        check!(h::metadata(&dir, "file_symlink_file"))
            .file_type()
            .is_file()
    );
    assert!(check!(h::metadata(&dir, "dir_symlink_dir")).is_dir());
    assert!(
        check!(h::metadata(&dir, "file_symlink"))
            .file_type()
            .is_file()
    );
    assert!(check!(h::metadata(&dir, "dir_symlink")).is_dir());

    assert!(
        !check!(h::metadata(&dir, "file_symlink_file"))
            .file_type()
            .is_symlink()
    );
    assert!(
        !check!(h::metadata(&dir, "dir_symlink_dir"))
            .file_type()
            .is_symlink()
    );
    assert!(
        !check!(h::metadata(&dir, "file_symlink"))
            .file_type()
            .is_symlink()
    );
    assert!(
        !check!(h::metadata(&dir, "dir_symlink"))
            .file_type()
            .is_symlink()
    );

    assert!(
        check!(h::symlink_metadata(&dir, "file_symlink_file"))
            .file_type()
            .is_symlink()
    );
    assert!(
        check!(h::symlink_metadata(&dir, "dir_symlink_dir"))
            .file_type()
            .is_symlink()
    );
    assert!(
        check!(h::symlink_metadata(&dir, "file_symlink"))
            .file_type()
            .is_symlink()
    );
    assert!(
        check!(h::symlink_metadata(&dir, "dir_symlink"))
            .file_type()
            .is_symlink()
    );

    assert!(
        !check!(h::metadata(&dir, "file_symlink_file"))
            .file_type()
            .is_symlink()
    );
    assert!(
        !check!(h::metadata(&dir, "dir_symlink_dir"))
            .file_type()
            .is_symlink()
    );
    assert!(
        !check!(h::metadata(&dir, "file_symlink"))
            .file_type()
            .is_symlink()
    );
    assert!(
        !check!(h::metadata(&dir, "dir_symlink"))
            .file_type()
            .is_symlink()
    );

    assert!(
        check!(h::symlink_metadata(&dir, "file_symlink_file"))
            .file_type()
            .is_symlink()
    );
    assert!(
        check!(h::symlink_metadata(&dir, "dir_symlink_dir"))
            .file_type()
            .is_symlink()
    );
    assert!(
        check!(h::symlink_metadata(&dir, "file_symlink"))
            .file_type()
            .is_symlink()
    );
    assert!(
        check!(h::symlink_metadata(&dir, "dir_symlink"))
            .file_type()
            .is_symlink()
    );
}

#[test]
fn symlink_absolute() {
    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);

    error_contains!(
        h::symlink(&dir, "/thing", "thing_symlink_file"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        h::symlink_file(&dir, "/file", "file_symlink_file"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        h::symlink_dir(&dir, "/dir", "dir_symlink_dir"),
        "a path led outside of the filesystem"
    );
}

#[test]
fn readlink_absolute() {
    if !symlink_supported() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();

    #[cfg(not(windows))]
    check!(std::os::unix::fs::symlink(
        "/thing",
        dir.path().join("thing_symlink")
    ));
    #[cfg(windows)]
    check!(std::os::windows::fs::symlink_file(
        "/file",
        dir.path().join("file_symlink_file")
    ));
    #[cfg(windows)]
    check!(std::os::windows::fs::symlink_dir(
        "/dir",
        dir.path().join("dir_symlink_dir")
    ));

    let dir = check!(h::open_ambient_dir(dir.path()));

    #[cfg(not(windows))]
    error_contains!(
        p::read_link(&dir, Path::new("thing_symlink")),
        "a path led outside of the filesystem"
    );
    #[cfg(windows)]
    error_contains!(
        p::read_link(&dir, Path::new("file_symlink_file")),
        "a path led outside of the filesystem"
    );
    #[cfg(windows)]
    error_contains!(
        p::read_link(&dir, Path::new("dir_symlink_dir")),
        "a path led outside of the filesystem"
    );
}

/// Opening directories without following symlinks.
#[test]
fn open_dir_nofollow() {
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();

    let dir = h::dir_of(&tmpdir);

    check!(h::create(&dir, "file"));
    check!(h::create_dir(&dir, "dir"));
    check!(h::symlink_file(&dir, "file", "symlink_file"));
    check!(h::symlink_dir(&dir, "dir", "symlink_dir"));
    check!(h::symlink_dir(&dir, "dir/", "symlink_dir_slash"));
    check!(h::symlink_dir(&dir, "dir/.", "symlink_dir_slashdot"));
    check!(h::symlink_dir(&dir, "dir/..", "symlink_dir_slashdotdot"));
    check!(h::symlink_dir(
        &dir,
        "dir/../",
        "symlink_dir_slashdotdotslash"
    ));
    check!(h::symlink_dir(&dir, ".", "symlink_dot"));
    check!(h::symlink_dir(&dir, "./", "symlink_dotslash"));

    // First try without `nofollow`. The "symlink_dir" case should succeed.
    assert!(p::open_dir(&dir, Path::new("file")).is_err());
    assert!(p::open_dir(&dir, Path::new("symlink_file")).is_err());
    check!(p::open_dir(&dir, Path::new("symlink_dir")));
    #[cfg(windows)]
    check!(p::open_dir(&dir, Path::new("symlink_dir\\")));
    check!(p::open_dir(&dir, Path::new("symlink_dir/")));
    #[cfg(windows)]
    {
        error!(p::open_dir(&dir, Path::new("symlink_dir_slash")), 123);
        error!(p::open_dir(&dir, Path::new("symlink_dir_slashdotdot")), 123);
        error!(
            p::open_dir(&dir, Path::new("symlink_dir_slashdotdotslash")),
            123
        );
        error!(p::open_dir(&dir, Path::new("symlink_dotslash")), 123);
        error!(p::open_dir(&dir, Path::new("symlink_dir_slashdot")), 123);
    }
    #[cfg(not(windows))]
    {
        check!(p::open_dir(&dir, Path::new("symlink_dir_slash")));
        check!(p::open_dir(&dir, Path::new("symlink_dir_slashdotdot")));
        check!(p::open_dir(&dir, Path::new("symlink_dir_slashdotdotslash")));
        check!(p::open_dir(&dir, Path::new("symlink_dotslash")));
        check!(p::open_dir(&dir, Path::new("symlink_dir_slashdot")));
    }
    check!(p::open_dir(&dir, Path::new("symlink_dot")));
    check!(p::open_dir(&dir, Path::new("dir")));

    // Next try with `nofollow`. The "symlink_dir" case should fail.
    assert!(h::open_dir_nofollow(&dir, "file").is_err());
    assert!(h::open_dir_nofollow(&dir, "symlink_file").is_err());
    assert!(h::open_dir_nofollow(&dir, "symlink_dir").is_err());
    assert!(h::open_dir_nofollow(&dir, "symlink_dir_slash").is_err());
    assert!(h::open_dir_nofollow(&dir, "symlink_dir_slashdot").is_err());
    assert!(h::open_dir_nofollow(&dir, "symlink_dir_slashdotdot").is_err());
    assert!(h::open_dir_nofollow(&dir, "symlink_dir_slashdotdotslash").is_err());
    assert!(h::open_dir_nofollow(&dir, "symlink_dot").is_err());
    assert!(h::open_dir_nofollow(&dir, "symlink_dotslash").is_err());
    check!(h::open_dir_nofollow(&dir, "dir"));

    // Check various ways of spelling `dir/../symlink_dir`.
    for dir_name in &["dir", "symlink_dir"] {
        let name = format!("{dir_name}/../symlink_dir");
        check!(p::open_dir(&dir, Path::new(&name)));
        assert!(h::open_dir_nofollow(&dir, &name).is_err());
    }

    // Check various paths which end with a symlink (even though the symlink
    // expansion may end with `/` or a non-symlink).
    for suffix in &[""] {
        for symlink_dir in &["symlink_dot"] {
            let name = format!("{symlink_dir}{suffix}");
            check!(p::open_dir(&dir, Path::new(&name)));
            assert!(h::open_dir_nofollow(&dir, &name).is_err());
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{dir_name}/../{name}");
                check!(p::open_dir(&dir, Path::new(&name)));
                assert!(h::open_dir_nofollow(&dir, &name).is_err());
            }
        }
    }

    // Check more paths which end with a symlink. On Windows, these fail due to
    // the symlink-to-path-ending-in-trailing-slash error.
    for suffix in &[""] {
        for symlink_dir in &[
            "symlink_dir_slashdotdot",
            "symlink_dir_slashdot",
            "symlink_dir_slash",
            "symlink_dir_slashdotdotslash",
            "symlink_dotslash",
        ] {
            let name = format!("{symlink_dir}{suffix}");
            #[cfg(windows)]
            {
                error!(p::open_dir(&dir, Path::new(&name)), 123);
            }
            #[cfg(not(windows))]
            {
                check!(p::open_dir(&dir, Path::new(&name)));
            }
            assert!(h::open_dir_nofollow(&dir, &name).is_err());
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{dir_name}/../{name}");
                #[cfg(windows)]
                {
                    error!(p::open_dir(&dir, Path::new(&name)), 123);
                }
                #[cfg(not(windows))]
                {
                    check!(p::open_dir(&dir, Path::new(&name)));
                }
                assert!(h::open_dir_nofollow(&dir, &name).is_err());
            }
        }
    }

    // Check those same paths, but with various suffixes appended, so that
    // `open_dir_nofollow` can open them.
    for suffix in &["/", "/.", "/./"] {
        for symlink_dir in &["symlink_dir", "symlink_dot"] {
            let name = format!("{symlink_dir}{suffix}");
            check!(p::open_dir(&dir, Path::new(&name)));
            // On Windows, a trailing dot is stripped early.
            if cfg!(not(windows)) || suffix != &"/." {
                check!(h::open_dir_nofollow(&dir, &name));
            } else {
                assert!(h::open_dir_nofollow(&dir, &name).is_err());
            }
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{dir_name}/../{name}");
                check!(p::open_dir(&dir, Path::new(&name)));
                // On Windows, a trailing dot is stripped early.
                if cfg!(not(windows)) || suffix != &"/." {
                    check!(h::open_dir_nofollow(&dir, &name));
                } else {
                    assert!(h::open_dir_nofollow(&dir, &name).is_err());
                }
            }
        }
    }

    // Check those same paths, but with various suffixes appended. On
    // Windows, these fail due to the symlink-to-path-ending-in-trailing-slash
    // error.
    for suffix in &["/", "/.", "/./"] {
        for symlink_dir in &[
            "symlink_dir_slash",
            "symlink_dir_slashdot",
            "symlink_dir_slashdotdot",
            "symlink_dir_slashdotdotslash",
            "symlink_dotslash",
        ] {
            let name = format!("{symlink_dir}{suffix}");
            #[cfg(windows)]
            {
                error!(p::open_dir(&dir, Path::new(&name)), 123);
                assert!(h::open_dir_nofollow(&dir, &name).is_err());
            }
            #[cfg(not(windows))]
            {
                check!(p::open_dir(&dir, Path::new(&name)));
                check!(h::open_dir_nofollow(&dir, &name));
            }
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{dir_name}/../{name}");
                #[cfg(windows)]
                {
                    error!(p::open_dir(&dir, Path::new(&name)), 123);
                    assert!(h::open_dir_nofollow(&dir, &name).is_err());
                }
                #[cfg(not(windows))]
                {
                    check!(p::open_dir(&dir, Path::new(&name)));
                    check!(h::open_dir_nofollow(&dir, &name));
                }
            }
        }
    }

    // Check various ways of spelling `.`.
    for cur_dir in &["dir/..", "dir/../", ".", "./"] {
        check!(p::open_dir(&dir, Path::new(cur_dir)));
        check!(h::open_dir_nofollow(&dir, cur_dir));
    }
}

/// This test is the same as `open_dir_nofollow` but uses ambient APIs instead
/// of `cap_std`. The purpose of this test is to confirm fundamentally
/// OS-specific behaviors.
#[test]
fn open_dir_nofollow_ambient() {
    #[cfg(unix)]
    use std::os::unix::fs::{symlink as symlink_file, symlink as symlink_dir};
    #[cfg(windows)]
    use std::os::windows::fs::{symlink_dir, symlink_file};

    if !symlink_supported() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();

    check!(std::fs::File::create(dir.path().join("file")));
    check!(std::fs::create_dir(dir.path().join("dir")));
    check!(symlink_file("file", dir.path().join("symlink_file")));
    check!(symlink_dir("dir", dir.path().join("symlink_dir")));
    check!(symlink_dir("dir/", dir.path().join("symlink_dir_slash")));
    check!(symlink_dir(
        "dir/.",
        dir.path().join("symlink_dir_slashdot")
    ));
    check!(symlink_dir(
        "dir/..",
        dir.path().join("symlink_dir_slashdotdot")
    ));
    check!(symlink_dir(
        "dir/../",
        dir.path().join("symlink_dir_slashdotdotslash")
    ));
    check!(symlink_dir("./", dir.path().join("symlink_dotslash")));
    check!(symlink_dir(".", dir.path().join("symlink_dot")));

    assert!(h::open_ambient_dir(dir.path().join("file")).is_err());
    assert!(h::open_ambient_dir(dir.path().join("symlink_file")).is_err());
    check!(h::open_ambient_dir(dir.path().join("symlink_dir")));
    #[cfg(windows)]
    check!(h::open_ambient_dir(dir.path().join("symlink_dir\\")));
    check!(h::open_ambient_dir(dir.path().join("symlink_dir/")));
    #[cfg(windows)]
    {
        error!(
            h::open_ambient_dir(dir.path().join("symlink_dir_slash")),
            123
        );
        error!(
            h::open_ambient_dir(dir.path().join("symlink_dir_slashdotdot")),
            123
        );
        error!(
            h::open_ambient_dir(dir.path().join("symlink_dir_slashdotdotslash")),
            123
        );
        error!(
            h::open_ambient_dir(dir.path().join("symlink_dotslash")),
            123
        );
        error!(
            h::open_ambient_dir(dir.path().join("symlink_dir_slashdot")),
            123
        );
    }
    #[cfg(not(windows))]
    {
        check!(h::open_ambient_dir(dir.path().join("symlink_dir_slash")));
        check!(h::open_ambient_dir(
            dir.path().join("symlink_dir_slashdotdot")
        ));
        check!(h::open_ambient_dir(
            dir.path().join("symlink_dir_slashdotdotslash")
        ));
        check!(h::open_ambient_dir(dir.path().join("symlink_dotslash")));
        check!(h::open_ambient_dir(dir.path().join("symlink_dir_slashdot")));
    }
    check!(h::open_ambient_dir(dir.path().join("symlink_dot")));
    check!(h::open_ambient_dir(dir.path().join("dir")));

    // Check various ways of spelling `dir/../symlink_dir`.
    for dir_name in &["dir", "symlink_dir"] {
        let name = format!("{dir_name}/../symlink_dir");
        check!(h::open_ambient_dir(dir.path().join(&name)));
    }

    // Check various paths which end with a symlink (even though the symlink
    // expansion may end with `/` or a non-symlink).
    for suffix in &[""] {
        for symlink_dir in &["symlink_dot"] {
            let name = format!("{symlink_dir}{suffix}");
            check!(h::open_ambient_dir(dir.path().join(&name)));
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{dir_name}/../{name}");
                check!(h::open_ambient_dir(dir.path().join(&name)));
            }
        }
    }

    // Check more paths which end with a symlink. On Windows, these fail due to
    // the symlink-to-path-ending-in-trailing-slash error.
    for suffix in &[""] {
        for symlink_dir in &[
            "symlink_dir_slashdotdot",
            "symlink_dir_slashdot",
            "symlink_dir_slash",
            "symlink_dir_slashdotdotslash",
            "symlink_dotslash",
        ] {
            let name = format!("{symlink_dir}{suffix}");
            #[cfg(windows)]
            {
                error!(h::open_ambient_dir(dir.path().join(&name)), 123);
            }
            #[cfg(not(windows))]
            {
                check!(h::open_ambient_dir(dir.path().join(&name)));
            }
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{dir_name}/../{name}");
                #[cfg(windows)]
                {
                    error!(h::open_ambient_dir(dir.path().join(&name)), 123);
                }
                #[cfg(not(windows))]
                {
                    check!(h::open_ambient_dir(dir.path().join(&name)));
                }
            }
        }
    }

    // Check those same paths, but with various suffixes appended.
    for suffix in &["/", "/.", "/./"] {
        for symlink_dir in &["symlink_dir", "symlink_dot"] {
            let name = format!("{symlink_dir}{suffix}");
            check!(h::open_ambient_dir(dir.path().join(&name)));
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{dir_name}/../{name}");
                check!(h::open_ambient_dir(dir.path().join(&name)));
            }
        }
    }

    // Check those same paths, but with various suffixes appended. On
    // Windows, these fail due to the
    // symlink-to-path-ending-in-trailing-slash error.
    for suffix in &["/", "/.", "/./"] {
        for symlink_dir in &[
            "symlink_dir_slash",
            "symlink_dir_slashdot",
            "symlink_dir_slashdotdot",
            "symlink_dir_slashdotdotslash",
            "symlink_dotslash",
        ] {
            let name = format!("{symlink_dir}{suffix}");
            #[cfg(windows)]
            {
                error!(h::open_ambient_dir(dir.path().join(&name)), 123);
            }
            #[cfg(not(windows))]
            {
                check!(h::open_ambient_dir(dir.path().join(&name)));
            }
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{dir_name}/../{name}");
                #[cfg(windows)]
                {
                    error!(h::open_ambient_dir(dir.path().join(&name)), 123);
                }
                #[cfg(not(windows))]
                {
                    check!(h::open_ambient_dir(dir.path().join(&name)));
                }
            }
        }
    }

    // Check various ways of spelling `.`.
    for cur_dir in &["dir/..", "dir/../", ".", "./"] {
        check!(h::open_ambient_dir(dir.path().join(cur_dir)));
    }
}
