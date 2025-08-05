//! Generate feature-related Rust code.

use super::{Formatter, fmtln};
use crate::dsl;

impl dsl::Feature {
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

    /// E.g., `Features::Or(Features::Feature(compat), Features::Feature(64b))`
    ///
    /// Generate a Rust constructor expression that contains the feature
    /// boolean term.
    pub(crate) fn generate_constructor_expr(&self, f: &mut Formatter) {
        let mut index = 0;
        let name = self.generate_inner_constructor_expr(f, &mut index);
        fmtln!(f, "{name}");
    }

    fn generate_inner_constructor_expr(&self, f: &mut Formatter, index: &mut u32) -> String {
        use dsl::Features::*;

        let name = format!("F{index}");
        *index += 1;

        let const_expr = format!("const {name}: &'static Features");
        match self {
            And(lhs, rhs) => {
                let lhs = lhs.generate_inner_constructor_expr(f, index);
                let rhs = rhs.generate_inner_constructor_expr(f, index);
                fmtln!(f, "{const_expr} = &Features::And({lhs}, {rhs});");
            }
            Or(lhs, rhs) => {
                let lhs = lhs.generate_inner_constructor_expr(f, index);
                let rhs = rhs.generate_inner_constructor_expr(f, index);
                fmtln!(f, "{const_expr} = &Features::Or({lhs}, {rhs});");
            }
            Feature(feature) => {
                fmtln!(f, "{const_expr} = &Features::Feature(Feature::{feature});");
            }
        }
        name
    }
}
