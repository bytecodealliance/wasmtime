//! Legacy ("old-style") backends that will be removed in the future.

// N.B.: the old x86-64 backend (`x86`) and the new one (`x64`) are both
// included whenever building with x86 support. The new backend is the default,
// but the old can be requested with `BackendVariant::Legacy`. However, if this
// crate is built with the `old-x86-backend` feature, then the old backend is
// default instead.
#[cfg(feature = "x86")]
pub(crate) mod x86;

#[cfg(feature = "riscv")]
pub(crate) mod riscv;
