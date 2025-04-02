use std::marker::PhantomData;

/// Represents the readable end of a Component Model `future`.
pub struct FutureReader<T> {
    _phantom: PhantomData<T>,
}

/// Represents the readable end of a Component Model `stream`.
pub struct StreamReader<T> {
    _phantom: PhantomData<T>,
}

/// Represents a Component Model `error-context`.
pub struct ErrorContext {}
