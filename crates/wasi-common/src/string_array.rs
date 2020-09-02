use crate::Error;
use std::convert::TryInto;
use std::ffi::CString;
use wiggle::GuestPtr;

pub struct StringArray {
    elems: Vec<CString>,
    pub number_elements: u32,
    pub cumulative_size: u32,
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
}

impl StringArray {
    pub fn new(elems: Vec<String>) -> Result<Self, StringArrayError> {
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
