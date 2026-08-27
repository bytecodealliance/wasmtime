#[macro_use]
mod sys_common;

use std::path::Path;
use sys_common::io::tmpdir;

#[test]
fn canonicalize_dot() {
    let tmpdir = tmpdir();
    assert_eq!(check!(tmpdir.canonicalize(".")), Path::new("."));
}

#[test]
fn canonicalize_edge_cases() {
    let tmpdir = tmpdir();
    assert_eq!(check!(tmpdir.canonicalize(".")), Path::new("."));
    assert_eq!(check!(tmpdir.canonicalize("./")), Path::new("."));
    assert_eq!(check!(tmpdir.canonicalize("./.")), Path::new("."));

    #[cfg(not(windows))]
    error!(tmpdir.canonicalize(""), "No such file");
    #[cfg(windows)]
    error!(tmpdir.canonicalize(""), 2);

    #[cfg(not(windows))]
    error!(tmpdir.canonicalize("foo"), "No such file");
    #[cfg(windows)]
    error!(tmpdir.canonicalize("foo"), 2);

    error_contains!(
        tmpdir.canonicalize("/"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("/."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("/.."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("/./"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("/../"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("/../."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("/./."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("/./.."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("/./../"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("/../.."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("/../../"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize(".."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("../"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("../."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("./.."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("../.."),
        "a path led outside of the filesystem"
    );

    check!(tmpdir.create_dir_all("foo/bar"));
    assert_eq!(check!(tmpdir.canonicalize("foo")), Path::new("foo"));
    assert_eq!(check!(tmpdir.canonicalize("foo/")), Path::new("foo"));
    assert_eq!(check!(tmpdir.canonicalize("foo/")).to_str(), Some("foo"));
    assert_eq!(check!(tmpdir.canonicalize("foo/.")), Path::new("foo"));
    assert_eq!(check!(tmpdir.canonicalize("foo/./")), Path::new("foo"));
    assert_eq!(check!(tmpdir.canonicalize("foo/./")).to_str(), Some("foo"));
    assert_eq!(check!(tmpdir.canonicalize("foo/..")), Path::new("."));
    assert_eq!(check!(tmpdir.canonicalize("foo/../")), Path::new("."));
    assert_eq!(check!(tmpdir.canonicalize("foo/../.")), Path::new("."));

    assert_eq!(
        check!(tmpdir.canonicalize("foo/bar")),
        Path::new("foo").join("bar"),
    );
    assert_eq!(
        check!(tmpdir.canonicalize("foo/bar/")),
        Path::new("foo").join("bar")
    );
    assert_eq!(
        check!(tmpdir.canonicalize("foo/bar/")).to_str(),
        Path::new("foo").join("bar").to_str(),
    );
    assert_eq!(
        check!(tmpdir.canonicalize("foo/../foo/bar")),
        Path::new("foo").join("bar")
    );
    assert_eq!(
        check!(tmpdir.canonicalize("foo/../foo/bar/")),
        Path::new("foo").join("bar")
    );
    assert_eq!(
        check!(tmpdir.canonicalize("foo/../foo/bar/")).to_str(),
        Path::new("foo").join("bar").to_str()
    );
    error_contains!(
        tmpdir.canonicalize("foo/../.."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.canonicalize("foo/../../"),
        "a path led outside of the filesystem"
    );

    #[cfg(not(windows))]
    error!(tmpdir.canonicalize("foo/bar/qux"), "No such file");
    #[cfg(windows)]
    error!(tmpdir.canonicalize("foo/bar/qux"), 2);
}
