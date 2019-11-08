/// Representation of the various permissions on a file.
///
/// This corresponds to [`std::fs::Permissions`].
///
/// TODO: Not yet implemented.
///
/// [`std::fs::Permissions`]: https://doc.rust-lang.org/std/fs/struct.Permissions.html
#[derive(Eq, PartialEq, Clone)]
pub struct Permissions {}

impl Permissions {
    /// Returns true if these permissions describe a readonly (unwritable) file.
    ///
    /// This corresponds to [`std::fs::Permissions::readonly`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::Permissions::readonly`]: https://doc.rust-lang.org/std/fs/struct.Permissions.html#method.readonly
    pub fn readonly(&self) -> bool {
        unimplemented!("Permissions::readonly");
    }

    /// Modifies the readonly flag for this set of permissions.
    ///
    /// This corresponds to [`std::fs::Permissions::set_readonly`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::Permissions::set_readonly`]: https://doc.rust-lang.org/std/fs/struct.Permissions.html#method.set_readonly
    pub fn set_readonly(&mut self, readonly: bool) {
        unimplemented!("Permissions::set_readonly");
    }
}

// TODO: functions from PermissionsExt?

// TODO: impl Debug for Permissions
