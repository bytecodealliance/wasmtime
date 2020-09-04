use crate::Error;
use std::collections::HashMap;
use std::convert::TryInto;
use std::ffi::{CString, OsString};
use wiggle::GuestPtr;

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum PendingString {
    Bytes(Vec<u8>),
    OsString(OsString),
}

impl From<Vec<u8>> for PendingString {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Bytes(bytes)
    }
}

impl From<OsString> for PendingString {
    fn from(s: OsString) -> Self {
        Self::OsString(s)
    }
}

impl PendingString {
    pub fn into_string(self) -> Result<String, StringArrayError> {
        let res = match self {
            Self::Bytes(v) => String::from_utf8(v)?,
            #[cfg(unix)]
            Self::OsString(s) => {
                use std::os::unix::ffi::OsStringExt;
                String::from_utf8(s.into_vec())?
            }
            #[cfg(windows)]
            Self::OsString(s) => {
                use std::os::windows::ffi::OsStrExt;
                let bytes: Vec<u16> = s.encode_wide().collect();
                String::from_utf16(&bytes)?
            }
        };
        Ok(res)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StringArrayError {
    /// Provided sequence of bytes contained an unexpected NUL byte.
    #[error("provided sequence contained an unexpected NUL byte")]
    Nul(#[from] std::ffi::NulError),
    /// Too many elements: must fit into u32
    #[error("too many elements")]
    NumElements,
    /// Element size: must fit into u32
    #[error("element too big")]
    ElemSize,
    /// Cumulative element size: must fit into u32
    #[error("cumulative element size too big")]
    CumElemSize,
    /// Provided sequence of bytes was not a valid UTF-8.
    #[error("provided sequence is not valid UTF-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    /// Provided sequence of bytes was not a valid UTF-16.
    ///
    /// This error is expected to only occur on Windows hosts.
    #[error("provided sequence is not valid UTF-16: {0}")]
    InvalidUtf16(#[from] std::string::FromUtf16Error),
}

pub struct StringArray {
    elems: Vec<CString>,
    pub number_elements: u32,
    pub cumulative_size: u32,
}
impl StringArray {
    pub fn from_pending_vec(elems: Vec<PendingString>) -> Result<Self, StringArrayError> {
        let elems = elems
            .into_iter()
            .map(|arg| arg.into_string())
            .collect::<Result<Vec<String>, StringArrayError>>()?;
        Self::from_strings(elems)
    }
    pub fn from_pending_map(
        elems: HashMap<PendingString, PendingString>,
    ) -> Result<Self, StringArrayError> {
        let mut pairs = Vec::new();
        for (k, v) in elems.into_iter() {
            let mut pair = k.into_string()?;
            pair.push('=');
            pair.push_str(&v.into_string()?);
            pairs.push(pair);
        }
        Self::from_strings(pairs)
    }
    pub fn from_strings(elems: Vec<String>) -> Result<Self, StringArrayError> {
        let elems = elems
            .into_iter()
            .map(|s| CString::new(s))
            .collect::<Result<Vec<CString>, _>>()?;
        let number_elements = elems
            .len()
            .try_into()
            .map_err(|_| StringArrayError::NumElements)?;
        let mut cumulative_size: u32 = 0;
        for elem in elems.iter() {
            let elem_bytes = elem
                .as_bytes_with_nul()
                .len()
                .try_into()
                .map_err(|_| StringArrayError::ElemSize)?;
            cumulative_size = cumulative_size
                .checked_add(elem_bytes)
                .ok_or(StringArrayError::CumElemSize)?;
        }
        Ok(Self {
            elems,
            number_elements,
            cumulative_size,
        })
    }

    pub fn write_to_guest<'a>(
        &self,
        buffer: &GuestPtr<'a, u8>,
        element_heads: &GuestPtr<'a, GuestPtr<'a, u8>>,
    ) -> Result<(), Error> {
        let element_heads = element_heads.as_array(self.number_elements);
        let buffer = buffer.as_array(self.cumulative_size);
        let mut cursor = 0;
        for (elem, head) in self.elems.iter().zip(element_heads.iter()) {
            let bytes = elem.as_bytes_with_nul();
            let len: u32 = bytes.len().try_into()?;
            let elem_buffer = buffer
                .get_range(cursor..(cursor + len))
                .ok_or(Error::Inval)?; // Elements don't fit in buffer provided
            elem_buffer.copy_from_slice(bytes)?;
            head?.write(
                elem_buffer
                    .get(0)
                    .expect("all elem buffers at least length 1"),
            )?;
            cursor += len;
        }
        Ok(())
    }
}
