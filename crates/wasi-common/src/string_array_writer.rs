use crate::{Error, Result};
use std::convert::TryInto;
use std::ffi::CString;
use wiggle::GuestPtr;

pub trait StringArrayWriter {
    fn number_elements(&self) -> Result<u32>;
    fn cumulative_size(&self) -> Result<u32>;
    fn write_to_guest<'a>(
        &self,
        buffer: &GuestPtr<'a, u8>,
        elements: &GuestPtr<'a, GuestPtr<'a, u8>>,
    ) -> Result<()>;
}

impl StringArrayWriter for Vec<CString> {
    fn number_elements(&self) -> Result<u32> {
        let elems = self.len().try_into()?;
        Ok(elems)
    }
    fn cumulative_size(&self) -> Result<u32> {
        let mut total: u32 = 0;
        for elem in self.iter() {
            let elem_bytes = elem.as_bytes_with_nul().len().try_into()?;
            total = total.checked_add(elem_bytes).ok_or(Error::Overflow)?;
        }
        Ok(total)
    }
    fn write_to_guest<'a>(
        &self,
        buffer: &GuestPtr<'a, u8>,
        element_heads: &GuestPtr<'a, GuestPtr<'a, u8>>,
    ) -> Result<()> {
        let element_heads = element_heads.as_array(self.number_elements()?);
        let buffer = buffer.as_array(self.cumulative_size()?);
        let mut cursor = 0;
        for (elem, head) in self.iter().zip(element_heads.iter()) {
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
