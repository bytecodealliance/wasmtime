//! A DSL for describing x64 CPU features.

use core::fmt;
use std::ops::BitOr;

/// A collection of CPU features.
///
/// An instruction is valid when _any_ of the features in the collection are
/// enabled; i.e., the collection is an `OR` expression.
///
/// ```
/// # use cranelift_assembler_x64_meta::dsl::{Features, Feature};
/// let fs = Feature::_64b | Feature::compat;
/// assert_eq!(fs.to_string(), "_64b | compat");
/// ```
///
/// Duplicate features are not allowed and will cause a panic.
///
/// ```should_panic
/// # use cranelift_assembler_x64_meta::dsl::Feature;
/// let fs = Feature::_64b | Feature::_64b;
/// ```
#[derive(PartialEq)]
pub struct Features(Vec<Feature>);

impl Features {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Feature> {
        self.0.iter()
    }
}

impl fmt::Display for Features {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.iter().map(ToString::to_string).collect::<Vec<_>>().join(" | "))
    }
}

/// A CPU feature.
///
/// IA-32e mode is the typical mode of operation for modern 64-bit x86
/// processors. It consists of two sub-modes:
/// - __64-bit mode__: uses the full 64-bit address space
/// - __compatibility mode__: allows use of legacy 32-bit code
///
/// Other features listed here should match the __CPUID Feature Flags__ column
/// of the instruction tables of the x64 reference manual.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(non_camel_case_types, reason = "makes DSL definitions easier to read")]
pub enum Feature {
    _64b,
    compat,
    sse,
}

/// List all CPU features.
///
/// It is critical that this list contains _all_ variants of the [`Feature`]
/// `enum`. We use this list here in the `meta` level so that we can accurately
/// transcribe each variant to an `enum` available in the generated layer above.
/// If this list is incomplete, we will (fortunately) see compile errors for
/// generated functions that use the missing variants.
pub const ALL_FEATURES: &[Feature] = &[Feature::_64b, Feature::compat, Feature::sse];

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Feature::_64b => write!(f, "_64b"),
            Feature::compat => write!(f, "compat"),
            Feature::sse => write!(f, "sse"),
        }
    }
}

impl From<Feature> for Features {
    fn from(flag: Feature) -> Self {
        Features(vec![flag])
    }
}

impl From<Option<Feature>> for Features {
    fn from(flag: Option<Feature>) -> Self {
        Features(flag.into_iter().collect())
    }
}

impl BitOr for Feature {
    type Output = Features;
    fn bitor(self, rhs: Self) -> Self::Output {
        assert_ne!(self, rhs, "duplicate feature: {self:?}");
        Features(vec![self, rhs])
    }
}

impl BitOr<Feature> for Features {
    type Output = Features;
    fn bitor(mut self, rhs: Feature) -> Self::Output {
        assert!(!self.0.contains(&rhs), "duplicate feature: {rhs:?}");
        self.0.push(rhs);
        self
    }
}
