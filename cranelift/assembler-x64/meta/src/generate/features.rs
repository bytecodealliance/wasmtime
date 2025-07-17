//! Generate feature-related Rust code.

use super::{Formatter, fmtln};
use crate::dsl;

impl dsl::Feature {
    /// `pub trait Features { ... }`
    ///
    /// This function generates a `Features` trait that users can implement to
    /// query if instructions are available on a target CPU.
    pub(crate) fn generate_trait(f: &mut Formatter) {
        fmtln!(f, "#[doc(hidden)]");
        f.add_block("pub trait Features", |f| {
            for feature in dsl::ALL_FEATURES {
                fmtln!(f, "fn {feature}(&self) -> bool;");
            }
        });
    }

    /// `macro_rules! for_each_feature { ... }`
    ///
    /// This function generates a macro to allow generating code for each CPU
    /// feature.
    pub(crate) fn generate_macro(f: &mut Formatter) {
        fmtln!(f, "#[doc(hidden)]");
        fmtln!(f, "#[macro_export]");
        f.add_block("macro_rules! for_each_feature", |f| {
            f.add_block("($m:ident) =>", |f| {
                f.add_block("$m!", |f| {
                    for feature in dsl::ALL_FEATURES {
                        fmtln!(f, "{feature}");
                    }
                });
            });
        });
    }
}

impl dsl::Features {
    /// E.g., `features.is_sse2() && features.is_64b()`
    ///
    /// Generate a boolean expression that checks if the features are available.
    pub(crate) fn generate_boolean_expr(&self, name: &str) -> String {
        use dsl::Features::*;
        match self {
            And(lhs, rhs) => {
                let lhs = lhs.generate_inner_boolean_expr(name);
                let rhs = rhs.generate_inner_boolean_expr(name);
                format!("{lhs} && {rhs}")
            }
            Or(lhs, rhs) => {
                let lhs = lhs.generate_inner_boolean_expr(name);
                let rhs = rhs.generate_inner_boolean_expr(name);
                format!("{lhs} || {rhs}")
            }
            Feature(feature) => {
                format!("{name}.{feature}()")
            }
        }
    }

    // This adds parentheses for inner terms.
    fn generate_inner_boolean_expr(&self, name: &str) -> String {
        use dsl::Features::*;
        match self {
            And(lhs, rhs) => {
                let lhs = lhs.generate_inner_boolean_expr(name);
                let rhs = rhs.generate_inner_boolean_expr(name);
                format!("({lhs} && {rhs})")
            }
            Or(lhs, rhs) => {
                let lhs = lhs.generate_inner_boolean_expr(name);
                let rhs = rhs.generate_inner_boolean_expr(name);
                format!("({lhs} || {rhs})")
            }
            Feature(feature) => format!("{name}.{feature}()"),
        }
    }
}
