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
}
