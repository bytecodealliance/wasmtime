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

cfg_select! {
    feature = "rustls" => {
        pub use RustlsProvider as DefaultProvider;
    }
    feature = "openssl" => {
        pub use OpenSslProvider as DefaultProvider;
    }
    feature = "nativetls" => {
        pub use NativeTlsProvider as DefaultProvider;
    }
    _ => {
        pub use UnsupportedProvider as DefaultProvider;
    }
}
