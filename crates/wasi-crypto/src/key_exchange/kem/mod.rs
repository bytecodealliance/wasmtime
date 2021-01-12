#[cfg(feature = "pqcrypto")]
mod kyber;

use super::*;
#[cfg(feature = "pqcrypto")]
pub use kyber::*;

#[derive(Clone, Debug)]
pub struct EncapsulatedSecret {
    pub encapsulated_secret: Vec<u8>,
    pub secret: Vec<u8>,
}
