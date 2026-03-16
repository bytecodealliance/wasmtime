//! The available TLS providers.

mod unsupported;
pub use unsupported::*;
#[cfg(feature = "rustls")]
mod rustls;
#[cfg(feature = "rustls")]
pub use rustls::RustlsProvider;
#[cfg(feature = "openssl")]
mod openssl;
#[cfg(feature = "openssl")]
pub use openssl::OpenSslProvider;
#[cfg(feature = "nativetls")]
mod nativetls;
#[cfg(feature = "nativetls")]
pub use nativetls::NativeTlsProvider;

cfg_if::cfg_if! {
    if #[cfg(feature = "rustls")] {
        pub use RustlsProvider as DefaultProvider;
    } else if #[cfg(feature = "openssl")] {
        pub use OpenSslProvider as DefaultProvider;
    } else if #[cfg(feature = "nativetls")] {
        pub use NativeTlsProvider as DefaultProvider;
    } else {
        pub use UnsupportedProvider as DefaultProvider;
    }
}
