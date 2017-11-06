//! External names.
//!
//! These are identifiers for declaring entities defined outside the current
//! function. The name of an external declaration doesn't have any meaning to
//! Cretonne, which compiles functions independently.

use std::fmt::{self, Write};
use std::ascii::AsciiExt;

/// The name of an external can be any sequence of bytes.
///
/// External names are primarily used as keys by code using Cretonne to map
/// from a cretonne::ir::FuncRef or similar to additional associated data.
///
/// External names can also serve as a primitive testing and debugging tool.
/// In particular, many `.cton` test files use function names to identify
/// functions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct ExternalName(NameRepr);

impl ExternalName {
    /// Creates a new external name from a sequence of bytes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cretonne::ir::ExternalName;
    /// // Create `ExternalName` from a string.
    /// let name = ExternalName::new("hello");
    /// assert_eq!(name.to_string(), "%hello");
    ///
    /// // Create `ExternalName` from a sequence of bytes.
    /// let bytes: &[u8] = &[10, 9, 8];
    /// let name = ExternalName::new(bytes);
    /// assert_eq!(name.to_string(), "#0a0908");
    /// ```
    pub fn new<T>(v: T) -> ExternalName
    where
        T: Into<Vec<u8>>,
    {
        let vec = v.into();
        if vec.len() <= NAME_LENGTH_THRESHOLD {
            let mut bytes = [0u8; NAME_LENGTH_THRESHOLD];
            for (i, &byte) in vec.iter().enumerate() {
                bytes[i] = byte;
            }
            ExternalName(NameRepr::Short {
                length: vec.len() as u8,
                bytes: bytes,
            })
        } else {
            ExternalName(NameRepr::Long(vec))
        }
    }
}

/// Tries to interpret bytes as ASCII alphanumerical characters and `_`.
fn try_as_name(bytes: &[u8]) -> Option<String> {
    let mut name = String::with_capacity(bytes.len());
    for c in bytes.iter().map(|&b| b as char) {
        if c.is_ascii() && c.is_alphanumeric() || c == '_' {
            name.push(c);
        } else {
            return None;
        }
    }
    Some(name)
}

const NAME_LENGTH_THRESHOLD: usize = 22;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NameRepr {
    Short {
        length: u8,
        bytes: [u8; NAME_LENGTH_THRESHOLD],
    },
    Long(Vec<u8>),
}

impl AsRef<[u8]> for NameRepr {
    fn as_ref(&self) -> &[u8] {
        match *self {
            NameRepr::Short { length, ref bytes } => &bytes[0..length as usize],
            NameRepr::Long(ref vec) => vec.as_ref(),
        }
    }
}

impl AsRef<[u8]> for ExternalName {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Default for NameRepr {
    fn default() -> Self {
        NameRepr::Short {
            length: 0,
            bytes: [0; NAME_LENGTH_THRESHOLD],
        }
    }
}

impl fmt::Display for ExternalName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = try_as_name(self.0.as_ref()) {
            write!(f, "%{}", name)
        } else {
            f.write_char('#')?;
            for byte in self.0.as_ref() {
                write!(f, "{:02x}", byte)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ExternalName;

    #[test]
    fn displaying() {
        assert_eq!(ExternalName::new("").to_string(), "%");
        assert_eq!(ExternalName::new("x").to_string(), "%x");
        assert_eq!(ExternalName::new("x_1").to_string(), "%x_1");
        assert_eq!(ExternalName::new(" ").to_string(), "#20");
        assert_eq!(
            ExternalName::new("кретон").to_string(),
            "#d0bad180d0b5d182d0bed0bd"
        );
        assert_eq!(
            ExternalName::new("印花棉布").to_string(),
            "#e58db0e88ab1e6a389e5b883"
        );
        assert_eq!(
            ExternalName::new(vec![0, 1, 2, 3, 4, 5]).to_string(),
            "#000102030405"
        );
    }
}
