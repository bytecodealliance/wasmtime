//! The available TLS providers.

#[cfg(feature = "nativetls")]
mod native_tls;
#[cfg(feature = "nativetls")]
pub use native_tls::*;
#[cfg(feature = "rustls")]
mod rustls;
#[cfg(feature = "rustls")]
pub use rustls::*;

cfg_if::cfg_if! {
    if #[cfg(feature = "rustls")] {
        pub use RustlsProvider as DefaultProvider;
    } else if #[cfg(feature = "nativetls")] {
        pub use NativeTlsProvider as DefaultProvider;
    } else {
        compile_error!("At least one TLS provider must be enabled.");
    }
}
