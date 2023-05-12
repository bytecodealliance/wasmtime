use crate::{Error, ErrorExt};
use wiggle::GuestPtr;

pub struct StringArray {
    elems: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum StringArrayError {
    #[error("Number of elements exceeds 2^32")]
    NumberElements,
    #[error("Element size exceeds 2^32")]
    ElementSize,
    #[error("Cumulative size exceeds 2^32")]
    CumulativeSize,
}

impl StringArray {
    pub fn new() -> Self {
        StringArray { elems: Vec::new() }
    }

    pub fn push(&mut self, elem: String) -> Result<(), StringArrayError> {
        if self.elems.len() + 1 > std::u32::MAX as usize {
            return Err(StringArrayError::NumberElements);
        }
        if elem.as_bytes().len() + 1 > std::u32::MAX as usize {
            return Err(StringArrayError::ElementSize);
        }
        if self.cumulative_size() as usize + elem.as_bytes().len() + 1 > std::u32::MAX as usize {
            return Err(StringArrayError::CumulativeSize);
        }
        self.elems.push(elem);
        Ok(())
    }

    pub fn number_elements(&self) -> u32 {
        self.elems.len() as u32
    }

    pub fn cumulative_size(&self) -> u32 {
        self.elems
            .iter()
            .map(|e| e.as_bytes().len() + 1)
            .sum::<usize>() as u32
    }

    pub fn write_to_guest<'a>(
        &self,
        buffer: &GuestPtr<'a, u8>,
        element_heads: &GuestPtr<'a, GuestPtr<'a, u8>>,
    ) -> Result<(), Error> {
        let element_heads = element_heads.as_array(self.number_elements());
        let buffer = buffer.as_array(self.cumulative_size());
        let mut cursor = 0;
        for (elem, head) in self.elems.iter().zip(element_heads.iter()) {
            let bytes = elem.as_bytes();
            let len = bytes.len() as u32;
            {
                let elem_buffer = buffer
                    .get_range(cursor..(cursor + len))
                    .ok_or(Error::invalid_argument())?; // Elements don't fit in buffer provided
                elem_buffer.copy_from_slice(bytes)?;
            }
            buffer
                .get(cursor + len)
                .ok_or(Error::invalid_argument())?
                .write(0)?; // 0 terminate
            head?.write(buffer.get(cursor).expect("already validated"))?;
            cursor += len + 1;
        }
        Ok(())
    }
}
