use std::any::Any;

use crate::error::*;
use crate::handles::*;
use crate::key_exchange::KxOptions;
use crate::signatures::SignatureOptions;
use crate::symmetric::SymmetricOptions;
use crate::{AlgorithmType, CryptoCtx};

pub trait OptionsLike: Send + Sized {
    fn as_any(&self) -> &dyn Any;

    fn set(&mut self, _name: &str, _value: &[u8]) -> Result<(), CryptoError> {
        bail!(CryptoError::UnsupportedOption)
    }

    fn set_guest_buffer(
        &mut self,
        _name: &str,
        _buffer: &'static mut [u8],
    ) -> Result<(), CryptoError> {
        bail!(CryptoError::UnsupportedOption)
    }

    fn get(&self, _name: &str) -> Result<Vec<u8>, CryptoError> {
        bail!(CryptoError::UnsupportedOption)
    }

    fn set_u64(&mut self, _name: &str, _value: u64) -> Result<(), CryptoError> {
        bail!(CryptoError::UnsupportedOption)
    }

    fn get_u64(&self, _name: &str) -> Result<u64, CryptoError> {
        bail!(CryptoError::UnsupportedOption)
    }
}

#[derive(Clone, Debug)]
pub enum Options {
    Signatures(SignatureOptions),
    Symmetric(SymmetricOptions),
    KeyExchange(KxOptions),
}

impl Options {
    pub fn into_signatures(self) -> Result<SignatureOptions, CryptoError> {
        match self {
            Options::Signatures(options) => Ok(options),
            _ => bail!(CryptoError::InvalidHandle),
        }
    }

    pub fn into_symmetric(self) -> Result<SymmetricOptions, CryptoError> {
        match self {
            Options::Symmetric(options) => Ok(options),
            _ => bail!(CryptoError::InvalidHandle),
        }
    }

    pub fn into_key_exchange(self) -> Result<KxOptions, CryptoError> {
        match self {
            Options::KeyExchange(options) => Ok(options),
            _ => bail!(CryptoError::InvalidHandle),
        }
    }

    pub fn set(&mut self, name: &str, value: &[u8]) -> Result<(), CryptoError> {
        match self {
            Options::Signatures(options) => options.set(name, value),
            Options::Symmetric(options) => options.set(name, value),
            Options::KeyExchange(options) => options.set(name, value),
        }
    }

    pub fn set_guest_buffer(
        &mut self,
        name: &str,
        buffer: &'static mut [u8],
    ) -> Result<(), CryptoError> {
        match self {
            Options::Signatures(options) => options.set_guest_buffer(name, buffer),
            Options::Symmetric(options) => options.set_guest_buffer(name, buffer),
            Options::KeyExchange(options) => options.set_guest_buffer(name, buffer),
        }
    }

    pub fn get(&mut self, name: &str) -> Result<Vec<u8>, CryptoError> {
        match self {
            Options::Signatures(options) => options.get(name),
            Options::Symmetric(options) => options.get(name),
            Options::KeyExchange(options) => options.get(name),
        }
    }

    pub fn set_u64(&mut self, name: &str, value: u64) -> Result<(), CryptoError> {
        match self {
            Options::Signatures(options) => options.set_u64(name, value),
            Options::Symmetric(options) => options.set_u64(name, value),
            Options::KeyExchange(options) => options.set_u64(name, value),
        }
    }

    pub fn get_u64(&mut self, name: &str) -> Result<u64, CryptoError> {
        match self {
            Options::Signatures(options) => options.get_u64(name),
            Options::Symmetric(options) => options.get_u64(name),
            Options::KeyExchange(options) => options.get_u64(name),
        }
    }
}

impl CryptoCtx {
    pub fn options_open(&self, algorithm_type: AlgorithmType) -> Result<Handle, CryptoError> {
        let options = match algorithm_type {
            AlgorithmType::Signatures => Options::Signatures(SignatureOptions::default()),
            AlgorithmType::Symmetric => Options::Symmetric(SymmetricOptions::default()),
            AlgorithmType::KeyExchange => Options::KeyExchange(KxOptions::default()),
        };
        let handle = self.handles.options.register(options)?;
        Ok(handle)
    }

    pub fn options_close(&self, options_handle: Handle) -> Result<(), CryptoError> {
        self.handles.options.close(options_handle)
    }

    pub fn options_set(
        &self,
        options_handle: Handle,
        name: &str,
        value: &[u8],
    ) -> Result<(), CryptoError> {
        let mut options = self.handles.options.get(options_handle)?;
        options.set(name, value)
    }

    pub fn options_set_guest_buffer(
        &self,
        options_handle: Handle,
        name: &str,
        buffer: &'static mut [u8],
    ) -> Result<(), CryptoError> {
        let mut options = self.handles.options.get(options_handle)?;
        options.set_guest_buffer(name, buffer)
    }

    pub fn options_set_u64(
        &self,
        options_handle: Handle,
        name: &str,
        value: u64,
    ) -> Result<(), CryptoError> {
        let mut options = self.handles.options.get(options_handle)?;
        options.set_u64(name, value)
    }

    pub fn options_get(
        &self,
        options_handle: Handle,
        name: &str,
        value: &mut [u8],
    ) -> Result<usize, CryptoError> {
        let mut options = self.handles.options.get(options_handle)?;
        let v = options.get(name)?;
        let v_len = v.len();
        ensure!(v_len <= value.len(), CryptoError::Overflow);
        value[..v_len].copy_from_slice(&v);
        Ok(v_len)
    }

    pub fn options_get_u64(&self, options_handle: Handle, name: &str) -> Result<u64, CryptoError> {
        let mut options = self.handles.options.get(options_handle)?;
        let v = options.get_u64(name)?;
        Ok(v)
    }
}
