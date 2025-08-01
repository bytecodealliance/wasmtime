/// Error context reference count across a [`ComponentInstance`]
///
/// Contrasted to `LocalErrorContextRefCount`, this count is maintained
/// across all sub-components in a given component.
///
/// When this count is zero it is *definitely* safe to remove an error context.
#[repr(transparent)]
pub struct GlobalErrorContextRefCount(pub(crate) usize);
