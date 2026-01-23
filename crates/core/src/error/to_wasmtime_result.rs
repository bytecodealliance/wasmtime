use crate::error::{Error, Result};

/// Extension trait for easily converting `anyhow::Result`s into
/// `wasmtime::Result`s.
///
/// This is a small convenience helper to replace
/// `anyhow_result.map_err(wasmtime::Error::from_anyhow)` with
/// `anyhow_result.to_wasmtime_result()`.
///
/// Requires that the `"anyhow"` cargo feature is enabled.
///
/// # Example
///
/// ```
/// # fn _foo() {
/// #![cfg(feature = "anyhow")]
/// # use wasmtime_internal_core::error as wasmtime;
/// use wasmtime::ToWasmtimeResult as _;
///
/// fn returns_anyhow_result() -> anyhow::Result<u32> {
///     anyhow::bail!("eep")
/// }
///
/// fn returns_wasmtime_result() -> wasmtime::Result<()> {
///     // The following is equivalent to
///     //
///     //     returns_anyhow_result()
///     //         .map_err(wasmtime::Error::from_anyhow)?;
///     returns_anyhow_result().to_wasmtime_result()?;
///     Ok(())
/// }
///
/// let error: wasmtime::Error = returns_wasmtime_result().unwrap_err();
/// assert!(error.is::<anyhow::Error>());
/// assert_eq!(error.to_string(), "eep");
/// # }
/// ```
pub trait ToWasmtimeResult<T> {
    /// Convert this `anyhow::Result<T>` into a `wasmtime::Result<T>`.
    fn to_wasmtime_result(self) -> Result<T>;
}

impl<T> ToWasmtimeResult<T> for anyhow::Result<T> {
    #[inline]
    fn to_wasmtime_result(self) -> Result<T> {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(Error::from_anyhow(e)),
        }
    }
}
