/// Type alias for the standard library [`Result`](std::result::Result) type
/// to specifie [`Error`] as the error payload.
pub type Result<A, E> = std::result::Result<A, Error<E>>;

/// Error type used by the [`bindgen!`](crate::component::bindgen) macro.
///
/// This error type represents either the typed error `T` specified here or a
/// trap, represented with [`anyhow::Error`].
pub struct Error<T> {
    err: anyhow::Error,
    ty: std::marker::PhantomData<T>,
}

impl<T> Error<T> {
    /// Creates a new typed version of this error from the `T` specified.
    ///
    /// This error, if it makes its way to the guest, will be returned to the
    /// guest and the guest will be able to act upon it.
    ///
    /// Alternatively errors can be created with [`Error::trap`] which will
    /// cause the guest to trap and be unable to act upon it.
    pub fn new(err: T) -> Error<T>
    where
        T: std::error::Error + Send + Sync + 'static,
    {
        Error {
            err: err.into(),
            ty: std::marker::PhantomData,
        }
    }

    /// Creates a custom "trap" which will abort guest execution and have the
    /// specified `err` as the payload context returned from the original
    /// invocation.
    ///
    /// Note that if `err` here actually has type `T` then the error will not be
    /// considered a trap and will instead be dynamically detected as a normal
    /// error to communicate to the original module.
    pub fn trap(err: impl std::error::Error + Send + Sync + 'static) -> Error<T> {
        Error {
            err: anyhow::Error::from(err),
            ty: std::marker::PhantomData,
        }
    }

    /// Attempts to dynamically downcast this error internally to the `T`
    /// representation.
    ///
    /// If this error is internally represented as a `T` then `Ok(val)` will be
    /// returned. If this error is instead represented as a trap then
    /// `Err(trap)` will be returned instead.
    pub fn downcast(self) -> anyhow::Result<T>
    where
        T: std::error::Error + Send + Sync + 'static,
    {
        self.err.downcast::<T>()
    }

    /// Attempts to dynamically downcast this error to peek at the inner
    /// contents of `T` if present.
    pub fn downcast_ref(&self) -> Option<&T>
    where
        T: std::error::Error + Send + Sync + 'static,
    {
        self.err.downcast_ref::<T>()
    }

    /// Attempts to dynamically downcast this error to peek at the inner
    /// contents of `T` if present.
    pub fn downcast_mut(&mut self) -> Option<&mut T>
    where
        T: std::error::Error + Send + Sync + 'static,
    {
        self.err.downcast_mut::<T>()
    }

    /// Converts this error into an `anyhow::Error` which loses the `T` type
    /// information tagged to this error.
    pub fn into_inner(self) -> anyhow::Error {
        self.err
    }

    /// Same as [`anyhow::Error::context`], attaches a contextual message to
    /// this error.
    pub fn context<C>(self, context: C) -> Error<T>
    where
        C: std::fmt::Display + Send + Sync + 'static,
    {
        self.err.context(context).into()
    }
}

impl<T> std::ops::Deref for Error<T> {
    type Target = dyn std::error::Error + Send + Sync + 'static;
    fn deref(&self) -> &Self::Target {
        self.err.deref()
    }
}
impl<T> std::ops::DerefMut for Error<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.err.deref_mut()
    }
}

impl<T> std::fmt::Display for Error<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.err.fmt(f)
    }
}

impl<T> std::fmt::Debug for Error<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.err.fmt(f)
    }
}

impl<T> std::error::Error for Error<T> {}

impl<T> From<anyhow::Error> for Error<T> {
    fn from(err: anyhow::Error) -> Error<T> {
        Error {
            err,
            ty: std::marker::PhantomData,
        }
    }
}
