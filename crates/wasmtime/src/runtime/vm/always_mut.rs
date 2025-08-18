use core::fmt;

/// A helper types that is `Send` if `T` is `Sync`.
///
/// This structure is a newtype wrapper around the `T` type parameter. What
/// makes this a utility is the fact that it contains an `unsafe impl Sync`
/// implementation for the type when `T` is `Send`. This is then coupled with
/// the fact that there is no ability to access a shared reference, `&T`, from
/// this type. Instead all access is done through `&mut T`.
///
/// This means that accessing the `T` in this data structure always requires
/// exclusive `&mut self` access. This provides the trivial guarantee that this
/// type is safe to share across threads because if you do so then you're just
/// not able to do anything with it.
#[derive(Default /* Do not derive traits with &self here, that's not sound */)]
#[repr(transparent)]
pub struct AlwaysMut<T>(T);

// SAFETY: this is the purpose for existence of this type, meaning that if `T`
// is `Send` then this type is `Sync` because it statically disallows shared
// access to `T`.
unsafe impl<T: Send> Sync for AlwaysMut<T> {}

impl<T> AlwaysMut<T> {
    /// Creates a new [`AlwaysMut`] with the provided value.
    pub fn new(value: T) -> AlwaysMut<T> {
        AlwaysMut(value)
    }

    /// Return a mutable reference to the underlying data in this [`AlwaysMut`]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }

    /// Consume this [`AlwaysMut`], returning the underlying data.
    #[cfg(feature = "async")]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for AlwaysMut<T> {
    fn from(val: T) -> AlwaysMut<T> {
        AlwaysMut::new(val)
    }
}

impl<T> fmt::Debug for AlwaysMut<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AlwaysMut").finish_non_exhaustive()
    }
}
