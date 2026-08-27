/// Should symlinks be followed in the last component of a path?
///
/// This doesn't affect path components other than the last. So for example in
/// "foo/bar/baz", if "foo" or "bar" are symlinks, they will always be
/// followed. This enum value only determines whether "baz" is followed.
///
/// Instead of passing bare `bool`s as parameters, pass a distinct enum so that
/// the intent is clear.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FollowSymlinks {
    /// Yes, do follow symlinks in the last component of a path.
    Yes,

    /// No, do not follow symlinks in the last component of a path.
    No,
}
