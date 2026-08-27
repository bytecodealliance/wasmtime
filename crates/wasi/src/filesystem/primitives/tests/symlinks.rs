#[macro_use]
mod sys_common;

use cap_fs_ext::DirExt;
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use sys_common::io::tmpdir;
use sys_common::symlink_supported;

#[test]
fn basic_symlinks() {
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();

    check!(tmpdir.create("file"));
    check!(tmpdir.create_dir("dir"));
    assert!(check!(tmpdir.metadata("file")).is_file());
    assert!(!check!(tmpdir.metadata("file")).is_dir());
    assert!(check!(tmpdir.metadata("dir")).is_dir());
    assert!(!check!(tmpdir.metadata("dir")).is_file());
    assert!(!check!(tmpdir.metadata("file")).is_symlink());
    assert!(!check!(tmpdir.metadata("dir")).is_symlink());

    check!(tmpdir.symlink_file("file", "file_symlink_file"));
    check!(tmpdir.symlink_dir("dir", "dir_symlink_dir"));
    check!(tmpdir.symlink("file", "file_symlink"));
    check!(tmpdir.symlink("dir", "dir_symlink"));

    assert!(check!(tmpdir.metadata("file_symlink_file")).is_file());
    assert!(check!(tmpdir.metadata("dir_symlink_dir")).is_dir());
    assert!(check!(tmpdir.metadata("file_symlink")).is_file());
    assert!(check!(tmpdir.metadata("dir_symlink")).is_dir());

    assert!(!check!(tmpdir.metadata("file_symlink_file")).is_symlink());
    assert!(!check!(tmpdir.metadata("dir_symlink_dir")).is_symlink());
    assert!(!check!(tmpdir.metadata("file_symlink")).is_symlink());
    assert!(!check!(tmpdir.metadata("dir_symlink")).is_symlink());

    assert!(check!(tmpdir.symlink_metadata("file_symlink_file")).is_symlink());
    assert!(check!(tmpdir.symlink_metadata("dir_symlink_dir")).is_symlink());
    assert!(check!(tmpdir.symlink_metadata("file_symlink")).is_symlink());
    assert!(check!(tmpdir.symlink_metadata("dir_symlink")).is_symlink());

    assert!(!check!(tmpdir.metadata("file_symlink_file"))
        .file_type()
        .is_symlink());
    assert!(!check!(tmpdir.metadata("dir_symlink_dir"))
        .file_type()
        .is_symlink());
    assert!(!check!(tmpdir.metadata("file_symlink"))
        .file_type()
        .is_symlink());
    assert!(!check!(tmpdir.metadata("dir_symlink"))
        .file_type()
        .is_symlink());

    assert!(check!(tmpdir.symlink_metadata("file_symlink_file"))
        .file_type()
        .is_symlink());
    assert!(check!(tmpdir.symlink_metadata("dir_symlink_dir"))
        .file_type()
        .is_symlink());
    assert!(check!(tmpdir.symlink_metadata("file_symlink"))
        .file_type()
        .is_symlink());
    assert!(check!(tmpdir.symlink_metadata("dir_symlink"))
        .file_type()
        .is_symlink());
}

#[test]
fn symlink_absolute() {
    let tmpdir = tmpdir();

    error_contains!(
        tmpdir.symlink("/thing", "thing_symlink_file"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.symlink_file("/file", "file_symlink_file"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.symlink_dir("/dir", "dir_symlink_dir"),
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

    let tmpdir = check!(Dir::open_ambient_dir(dir.path(), ambient_authority()));

    #[cfg(not(windows))]
    error_contains!(
        tmpdir.read_link("thing_symlink"),
        "a path led outside of the filesystem"
    );
    #[cfg(windows)]
    error_contains!(
        tmpdir.read_link("file_symlink_file"),
        "a path led outside of the filesystem"
    );
    #[cfg(windows)]
    error_contains!(
        tmpdir.read_link("dir_symlink_dir"),
        "a path led outside of the filesystem"
    );
}

/// Opening directories without following symlinks.
#[test]
fn open_dir_nofollow() {
    use cap_fs_ext::DirExt;

    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();

    check!(tmpdir.create("file"));
    check!(tmpdir.create_dir("dir"));
    check!(tmpdir.symlink_file("file", "symlink_file"));
    check!(tmpdir.symlink_dir("dir", "symlink_dir"));
    check!(tmpdir.symlink_dir("dir/", "symlink_dir_slash"));
    check!(tmpdir.symlink_dir("dir/.", "symlink_dir_slashdot"));
    check!(tmpdir.symlink_dir("dir/..", "symlink_dir_slashdotdot"));
    check!(tmpdir.symlink_dir("dir/../", "symlink_dir_slashdotdotslash"));
    check!(tmpdir.symlink_dir(".", "symlink_dot"));
    check!(tmpdir.symlink_dir("./", "symlink_dotslash"));

    // First try without `nofollow`. The "symlink_dir" case should succeed.
    assert!(tmpdir.open_dir("file").is_err());
    assert!(tmpdir.open_dir("symlink_file").is_err());
    check!(tmpdir.open_dir("symlink_dir"));
    #[cfg(windows)]
    check!(tmpdir.open_dir("symlink_dir\\"));
    check!(tmpdir.open_dir("symlink_dir/"));
    #[cfg(windows)]
    {
        error!(tmpdir.open_dir("symlink_dir_slash"), 123);
        error!(tmpdir.open_dir("symlink_dir_slashdotdot"), 123);
        error!(tmpdir.open_dir("symlink_dir_slashdotdotslash"), 123);
        error!(tmpdir.open_dir("symlink_dotslash"), 123);
        error!(tmpdir.open_dir("symlink_dir_slashdot"), 123);
    }
    #[cfg(not(windows))]
    {
        check!(tmpdir.open_dir("symlink_dir_slash"));
        check!(tmpdir.open_dir("symlink_dir_slashdotdot"));
        check!(tmpdir.open_dir("symlink_dir_slashdotdotslash"));
        check!(tmpdir.open_dir("symlink_dotslash"));
        check!(tmpdir.open_dir("symlink_dir_slashdot"));
    }
    check!(tmpdir.open_dir("symlink_dot"));
    check!(tmpdir.open_dir("dir"));

    // Next try with `nofollow`. The "symlink_dir" case should fail.
    assert!(tmpdir.open_dir_nofollow("file").is_err());
    assert!(tmpdir.open_dir_nofollow("symlink_file").is_err());
    assert!(tmpdir.open_dir_nofollow("symlink_dir").is_err());
    assert!(tmpdir.open_dir_nofollow("symlink_dir_slash").is_err());
    assert!(tmpdir.open_dir_nofollow("symlink_dir_slashdot").is_err());
    assert!(tmpdir.open_dir_nofollow("symlink_dir_slashdotdot").is_err());
    assert!(tmpdir
        .open_dir_nofollow("symlink_dir_slashdotdotslash")
        .is_err());
    assert!(tmpdir.open_dir_nofollow("symlink_dot").is_err());
    assert!(tmpdir.open_dir_nofollow("symlink_dotslash").is_err());
    check!(tmpdir.open_dir_nofollow("dir"));

    // Check various ways of spelling `dir/../symlink_dir`.
    for dir_name in &["dir", "symlink_dir"] {
        let name = format!("{}/../symlink_dir", dir_name);
        check!(tmpdir.open_dir(&name));
        assert!(tmpdir.open_dir_nofollow(&name).is_err());
    }

    // Check various paths which end with a symlink (even though the symlink
    // expansion may end with `/` or a non-symlink).
    for suffix in &[""] {
        for symlink_dir in &["symlink_dot"] {
            let name = format!("{}{}", symlink_dir, suffix);
            check!(tmpdir.open_dir(&name));
            assert!(tmpdir.open_dir_nofollow(&name).is_err());
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{}/../{}", dir_name, name);
                check!(tmpdir.open_dir(&name));
                assert!(tmpdir.open_dir_nofollow(&name).is_err());
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
            let name = format!("{}{}", symlink_dir, suffix);
            #[cfg(windows)]
            {
                error!(tmpdir.open_dir(&name), 123);
            }
            #[cfg(not(windows))]
            {
                check!(tmpdir.open_dir(&name));
            }
            assert!(tmpdir.open_dir_nofollow(&name).is_err());
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{}/../{}", dir_name, name);
                #[cfg(windows)]
                {
                    error!(tmpdir.open_dir(&name), 123);
                }
                #[cfg(not(windows))]
                {
                    check!(tmpdir.open_dir(&name));
                }
                assert!(tmpdir.open_dir_nofollow(&name).is_err());
            }
        }
    }

    // Check those same paths, but with various suffixes appended, so that
    // `open_dir_nofollow` can open them.
    for suffix in &["/", "/.", "/./"] {
        for symlink_dir in &["symlink_dir", "symlink_dot"] {
            let name = format!("{}{}", symlink_dir, suffix);
            check!(tmpdir.open_dir(&name));
            // On Windows, a trailing dot is stripped early.
            if cfg!(not(windows)) || suffix != &"/." {
                check!(tmpdir.open_dir_nofollow(&name));
            } else {
                assert!(tmpdir.open_dir_nofollow(&name).is_err());
            }
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{}/../{}", dir_name, name);
                check!(tmpdir.open_dir(&name));
                // On Windows, a trailing dot is stripped early.
                if cfg!(not(windows)) || suffix != &"/." {
                    check!(tmpdir.open_dir_nofollow(&name));
                } else {
                    assert!(tmpdir.open_dir_nofollow(&name).is_err());
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
            let name = format!("{}{}", symlink_dir, suffix);
            #[cfg(windows)]
            {
                error!(tmpdir.open_dir(&name), 123);
                assert!(tmpdir.open_dir_nofollow(&name).is_err());
            }
            #[cfg(not(windows))]
            {
                check!(tmpdir.open_dir(&name));
                check!(tmpdir.open_dir_nofollow(&name));
            }
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{}/../{}", dir_name, name);
                #[cfg(windows)]
                {
                    error!(tmpdir.open_dir(&name), 123);
                    assert!(tmpdir.open_dir_nofollow(&name).is_err());
                }
                #[cfg(not(windows))]
                {
                    check!(tmpdir.open_dir(&name));
                    check!(tmpdir.open_dir_nofollow(&name));
                }
            }
        }
    }

    // Check various ways of spelling `.`.
    for cur_dir in &["dir/..", "dir/../", ".", "./"] {
        check!(tmpdir.open_dir(cur_dir));
        check!(tmpdir.open_dir_nofollow(cur_dir));
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

    assert!(Dir::open_ambient_dir(dir.path().join("file"), ambient_authority()).is_err());
    assert!(Dir::open_ambient_dir(dir.path().join("symlink_file"), ambient_authority()).is_err());
    check!(Dir::open_ambient_dir(
        dir.path().join("symlink_dir"),
        ambient_authority()
    ));
    #[cfg(windows)]
    check!(Dir::open_ambient_dir(
        dir.path().join("symlink_dir\\"),
        ambient_authority()
    ));
    check!(Dir::open_ambient_dir(
        dir.path().join("symlink_dir/"),
        ambient_authority()
    ));
    #[cfg(windows)]
    {
        error!(
            Dir::open_ambient_dir(dir.path().join("symlink_dir_slash"), ambient_authority()),
            123
        );
        error!(
            Dir::open_ambient_dir(
                dir.path().join("symlink_dir_slashdotdot"),
                ambient_authority()
            ),
            123
        );
        error!(
            Dir::open_ambient_dir(
                dir.path().join("symlink_dir_slashdotdotslash"),
                ambient_authority()
            ),
            123
        );
        error!(
            Dir::open_ambient_dir(dir.path().join("symlink_dotslash"), ambient_authority()),
            123
        );
        error!(
            Dir::open_ambient_dir(dir.path().join("symlink_dir_slashdot"), ambient_authority()),
            123
        );
    }
    #[cfg(not(windows))]
    {
        check!(Dir::open_ambient_dir(
            dir.path().join("symlink_dir_slash"),
            ambient_authority()
        ));
        check!(Dir::open_ambient_dir(
            dir.path().join("symlink_dir_slashdotdot"),
            ambient_authority()
        ));
        check!(Dir::open_ambient_dir(
            dir.path().join("symlink_dir_slashdotdotslash"),
            ambient_authority()
        ));
        check!(Dir::open_ambient_dir(
            dir.path().join("symlink_dotslash"),
            ambient_authority()
        ));
        check!(Dir::open_ambient_dir(
            dir.path().join("symlink_dir_slashdot"),
            ambient_authority()
        ));
    }
    check!(Dir::open_ambient_dir(
        dir.path().join("symlink_dot"),
        ambient_authority()
    ));
    check!(Dir::open_ambient_dir(
        dir.path().join("dir"),
        ambient_authority()
    ));

    // Check various ways of spelling `dir/../symlink_dir`.
    for dir_name in &["dir", "symlink_dir"] {
        let name = format!("{}/../symlink_dir", dir_name);
        check!(Dir::open_ambient_dir(
            dir.path().join(&name),
            ambient_authority()
        ));
    }

    // Check various paths which end with a symlink (even though the symlink
    // expansion may end with `/` or a non-symlink).
    for suffix in &[""] {
        for symlink_dir in &["symlink_dot"] {
            let name = format!("{}{}", symlink_dir, suffix);
            check!(Dir::open_ambient_dir(
                dir.path().join(&name),
                ambient_authority()
            ));
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{}/../{}", dir_name, name);
                check!(Dir::open_ambient_dir(
                    dir.path().join(&name),
                    ambient_authority()
                ));
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
            let name = format!("{}{}", symlink_dir, suffix);
            #[cfg(windows)]
            {
                error!(
                    Dir::open_ambient_dir(dir.path().join(&name), ambient_authority()),
                    123
                );
            }
            #[cfg(not(windows))]
            {
                check!(Dir::open_ambient_dir(
                    dir.path().join(&name),
                    ambient_authority()
                ));
            }
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{}/../{}", dir_name, name);
                #[cfg(windows)]
                {
                    error!(
                        Dir::open_ambient_dir(dir.path().join(&name), ambient_authority()),
                        123
                    );
                }
                #[cfg(not(windows))]
                {
                    check!(Dir::open_ambient_dir(
                        dir.path().join(&name),
                        ambient_authority()
                    ));
                }
            }
        }
    }

    // Check those same paths, but with various suffixes appended.
    for suffix in &["/", "/.", "/./"] {
        for symlink_dir in &["symlink_dir", "symlink_dot"] {
            let name = format!("{}{}", symlink_dir, suffix);
            check!(Dir::open_ambient_dir(
                dir.path().join(&name),
                ambient_authority()
            ));
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{}/../{}", dir_name, name);
                check!(Dir::open_ambient_dir(
                    dir.path().join(&name),
                    ambient_authority()
                ));
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
            let name = format!("{}{}", symlink_dir, suffix);
            #[cfg(windows)]
            {
                error!(
                    Dir::open_ambient_dir(dir.path().join(&name), ambient_authority()),
                    123
                );
            }
            #[cfg(not(windows))]
            {
                check!(Dir::open_ambient_dir(
                    dir.path().join(&name),
                    ambient_authority()
                ));
            }
            for dir_name in &["dir", "symlink_dir"] {
                let name = format!("{}/../{}", dir_name, name);
                #[cfg(windows)]
                {
                    error!(
                        Dir::open_ambient_dir(dir.path().join(&name), ambient_authority()),
                        123
                    );
                }
                #[cfg(not(windows))]
                {
                    check!(Dir::open_ambient_dir(
                        dir.path().join(&name),
                        ambient_authority()
                    ));
                }
            }
        }
    }

    // Check various ways of spelling `.`.
    for cur_dir in &["dir/..", "dir/../", ".", "./"] {
        check!(Dir::open_ambient_dir(
            dir.path().join(cur_dir),
            ambient_authority()
        ));
    }
}
