//! Generate feature-related Rust code.

use super::{fmtln, Formatter};
use crate::{dsl, generate::generate_derive};

impl dsl::Feature {
    /// `pub enum Feature { ... }`
    ///
    /// This function recreates the `Feature` struct itself in the generated
    /// code.
    pub fn generate_enum(f: &mut Formatter) {
        fmtln!(f, "#[doc(hidden)]");
        generate_derive(f);
        fmtln!(f, "#[derive(Copy, PartialEq)]"); // Add a couple more helpful derives.
        fmtln!(f, "pub enum Feature {{");
        f.indent(|f| {
            for feature in dsl::ALL_FEATURES {
                fmtln!(f, "{feature},");
            }
        });
        fmtln!(f, "}}");
    }
}
