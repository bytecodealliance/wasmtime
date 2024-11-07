//! Generate feature-related Rust code.

use super::{fmtln, Formatter};
use crate::{dsl, generate::generate_derive};

impl dsl::Feature {
    /// `pub enum Feature { ... }`
    ///
    /// This function recreates the `Feature` struct itself in the generated
    /// code.
    pub fn generate_enum(f: &mut Formatter) {
        use dsl::Feature::*;
        generate_derive(f);
        fmtln!(f, "pub enum Feature {{");
        f.indent(|f| {
            // N.B.: it is critical that this list contains _all_ variants of
            // the `Flag` enumeration here at the `meta` level so that we can
            // accurately transcribe them to a structure available in the
            // generated layer above. If this list is incomplete, we will
            // (fortunately) see compile errors for generated functions that use
            // the missing variants.
            const ALL: &[dsl::Feature] = &[_64b, compat];
            for flag in ALL {
                fmtln!(f, "{flag},");
            }
        });
        fmtln!(f, "}}");
    }
}
