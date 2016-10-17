//! Shared settings module.
//!
//! This module defines data structures to access the settings defined in the meta language.
//!
//! Each settings group is translated to a `Flags` struct either in this module or in its
//! ISA-specific `settings` module. The struct provides individual getter methods for all of the
//! settings as well as computed predicate flags.
//!
//! The `Flags` struct is immutable once it has been created. A `Builder` instance is used to
//! create it.
//!
//! # Example
//! ```
//! use cretonne::settings::{self, Configurable};
//!
//! let mut b = settings::builder();
//! b.set("opt_level", "fastest");
//!
//! let f = settings::Flags::new(&b);
//! assert_eq!(f.opt_level(), settings::OptLevel::Fastest);
//! ```

use std::fmt;
use std::result;

use constant_hash::{probe, simple_hash};

/// A string-based configurator for settings groups.
///
/// The `Configurable` protocol allows settings to be modified by name before a finished `Flags`
/// struct is created.
pub trait Configurable {
    /// Set the string value of any setting by name.
    ///
    /// This can set any type of setting whether it is numeric, boolean, or enumerated.
    fn set(&mut self, name: &str, value: &str) -> Result<()>;

    /// Set the value of a boolean setting by name.
    ///
    /// If the identified setting isn't a boolean, a `BadType` error is returned.
    fn set_bool(&mut self, name: &str, value: bool) -> Result<()>;
}

/// Collect settings values based on a template.
pub struct Builder {
    template: &'static detail::Template,
    bytes: Vec<u8>,
}

impl Builder {
    /// Create a new builder with defaults and names from the given template.
    pub fn new(tmpl: &'static detail::Template) -> Builder {
        Builder {
            template: tmpl,
            bytes: tmpl.defaults.into(),
        }
    }

    /// Extract contents of builder once everything is configured.
    pub fn state_for(&self, name: &str) -> &[u8] {
        assert_eq!(name, self.template.name);
        &self.bytes[..]
    }

    /// Set the value of a single bit.
    fn set_bit(&mut self, offset: usize, bit: u8, value: bool) {
        let byte = &mut self.bytes[offset];
        let mask = 1 << bit;
        if value {
            *byte |= mask;
        } else {
            *byte &= !mask;
        }
    }

    /// Look up a descriptor by name.
    fn lookup(&self, name: &str) -> Result<(usize, detail::Detail)> {
        match probe(self.template, name, simple_hash(name)) {
            None => Err(Error::BadName),
            Some(entry) => {
                let d = &self.template.descriptors[self.template.hash_table[entry] as usize];
                Ok((d.offset as usize, d.detail))
            }
        }
    }
}

fn parse_bool_value(value: &str) -> Result<bool> {
    match value {
        "true" | "on" | "yes" | "1" => Ok(true),
        "false" | "off" | "no" | "0" => Ok(false),
        _ => Err(Error::BadValue),
    }
}

fn parse_enum_value(value: &str, choices: &[&str]) -> Result<u8> {
    match choices.iter().position(|&tag| tag == value) {
        Some(idx) => Ok(idx as u8),
        None => Err(Error::BadValue),
    }
}

impl Configurable for Builder {
    fn set_bool(&mut self, name: &str, value: bool) -> Result<()> {
        use self::detail::Detail;
        let (offset, detail) = try!(self.lookup(name));
        if let Detail::Bool { bit } = detail {
            self.set_bit(offset, bit, value);
            Ok(())
        } else {
            Err(Error::BadType)
        }
    }

    fn set(&mut self, name: &str, value: &str) -> Result<()> {
        use self::detail::Detail;
        let (offset, detail) = try!(self.lookup(name));
        match detail {
            Detail::Bool { bit } => {
                self.set_bit(offset, bit, try!(parse_bool_value(value)));
            }
            Detail::Num => {
                self.bytes[offset] = try!(value.parse().map_err(|_| Error::BadValue));
            }
            Detail::Enum { last, enumerators } => {
                self.bytes[offset] = try!(parse_enum_value(value,
                                                           self.template.enums(last, enumerators)));
            }
        }
        Ok(())
    }
}

/// An error produced when changing a setting.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// No setting by this name exists.
    BadName,

    /// Type mismatch for setting (e.g., setting an enum setting as a bool).
    BadType,

    /// This is not a valid value for this setting.
    BadValue,
}

pub type Result<T> = result::Result<T, Error>;

/// Implementation details for generated code.
///
/// This module holds definitions that need to be public so the can be instantiated by generated
/// code in other modules.
pub mod detail {
    use std::fmt;
    use constant_hash;

    /// An instruction group template.
    pub struct Template {
        pub name: &'static str,
        pub descriptors: &'static [Descriptor],
        pub enumerators: &'static [&'static str],
        pub hash_table: &'static [u16],
        pub defaults: &'static [u8],
    }

    impl Template {
        /// Get enumerators corresponding to a `Details::Enum`.
        pub fn enums(&self, last: u8, enumerators: u16) -> &[&'static str] {
            let from = enumerators as usize;
            let len = last as usize + 1;
            &self.enumerators[from..from + len]
        }

        /// Format a setting value as a TOML string. This is mostly for use by the generated
        /// `Display` implementation.
        pub fn format_toml_value(&self,
                                 detail: Detail,
                                 byte: u8,
                                 f: &mut fmt::Formatter)
                                 -> fmt::Result {
            match detail {
                Detail::Bool { bit } => write!(f, "{}", (byte & (1 << bit)) != 0),
                Detail::Num => write!(f, "{}", byte),
                Detail::Enum { last, enumerators } => {
                    if byte <= last {
                        let tags = self.enums(last, enumerators);
                        write!(f, "\"{}\"", tags[byte as usize])
                    } else {
                        write!(f, "{}", byte)
                    }
                }
            }
        }
    }

    /// The template contains a hash table for by-name lookup.
    impl<'a> constant_hash::Table<&'a str> for Template {
        fn len(&self) -> usize {
            self.hash_table.len()
        }

        fn key(&self, idx: usize) -> Option<&'a str> {
            let e = self.hash_table[idx] as usize;
            if e < self.descriptors.len() {
                Some(self.descriptors[e].name)
            } else {
                None
            }
        }
    }

    /// A setting descriptor holds the information needed to generically set and print a setting.
    ///
    /// Each settings group will be represented as a constant DESCRIPTORS array.
    pub struct Descriptor {
        /// Lower snake-case name of setting as defined in meta.
        pub name: &'static str,

        /// Offset of byte containing this setting.
        pub offset: u32,

        /// Additional details, depending on the kind of setting.
        pub detail: Detail,
    }

    /// The different kind of settings along with descriptor bits that depend on the kind.
    #[derive(Clone, Copy)]
    pub enum Detail {
        /// A boolean setting only uses one bit, numbered from LSB.
        Bool { bit: u8 },

        /// A numerical setting uses the whole byte.
        Num,

        /// An Enum setting uses a range of enumerators.
        Enum {
            /// Numerical value of last enumerator, allowing for 1-256 enumerators.
            last: u8,

            /// First enumerator in the ENUMERATORS table.
            enumerators: u16,
        },
    }
}

// Include code generated by `meta/gen_settings.py`. This file contains a public `Flags` struct
// with an impl for all of the settings defined in `meta/cretonne/settings.py`.
include!(concat!(env!("OUT_DIR"), "/settings.rs"));

#[cfg(test)]
mod tests {
    use super::{builder, Flags};
    use super::Error::*;
    use super::Configurable;

    #[test]
    fn display_default() {
        let b = builder();
        let f = Flags::new(&b);
        assert_eq!(f.to_string(),
                   "[shared]\n\
                    opt_level = \"default\"\n\
                    is_64bit = false\n\
                    enable_float = true\n\
                    enable_simd = true\n\
                    enable_atomics = true\n");
        assert_eq!(f.opt_level(), super::OptLevel::Default);
        assert_eq!(f.enable_simd(), true);
    }

    #[test]
    fn modify_bool() {
        let mut b = builder();
        assert_eq!(b.set_bool("not_there", true), Err(BadName));
        assert_eq!(b.set_bool("enable_simd", true), Ok(()));
        assert_eq!(b.set_bool("enable_simd", false), Ok(()));

        let f = Flags::new(&b);
        assert_eq!(f.enable_simd(), false);
    }

    #[test]
    fn modify_string() {
        let mut b = builder();
        assert_eq!(b.set("not_there", "true"), Err(BadName));
        assert_eq!(b.set("enable_simd", ""), Err(BadValue));
        assert_eq!(b.set("enable_simd", "best"), Err(BadValue));
        assert_eq!(b.set("opt_level", "true"), Err(BadValue));
        assert_eq!(b.set("opt_level", "best"), Ok(()));
        assert_eq!(b.set("enable_simd", "0"), Ok(()));

        let f = Flags::new(&b);
        assert_eq!(f.enable_simd(), false);
        assert_eq!(f.opt_level(), super::OptLevel::Best);
    }
}
