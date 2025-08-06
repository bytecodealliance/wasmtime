//! A DSL for describing x64 CPU features.

use core::fmt;
use std::ops::{BitAnd, BitOr};

/// A boolean term of CPU features.
///
/// An instruction is valid when the boolean term (a recursive tree of `AND` and
/// `OR` terms) is satisfied.
///
/// ```
/// # use cranelift_assembler_x64_meta::dsl::{Features, Feature};
/// let fs = Feature::_64b | Feature::compat;
/// assert_eq!(fs.to_string(), "(_64b | compat)");
/// ```
#[derive(PartialEq)]
pub enum Features {
    And(Box<Features>, Box<Features>),
    Or(Box<Features>, Box<Features>),
    Feature(Feature),
}

impl Features {
    pub(crate) fn is_sse(&self) -> bool {
        use Feature::*;
        match self {
            Features::And(lhs, rhs) => lhs.is_sse() || rhs.is_sse(),
            Features::Or(lhs, rhs) => lhs.is_sse() || rhs.is_sse(),
            Features::Feature(feature) => {
                matches!(feature, sse | sse2 | sse3 | ssse3 | sse41 | sse42)
            }
        }
    }
}

impl fmt::Display for Features {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Features::And(lhs, rhs) => write!(f, "({lhs} & {rhs})"),
            Features::Or(lhs, rhs) => write!(f, "({lhs} | {rhs})"),
            Features::Feature(feature) => write!(f, "{feature:#?}"),
        }
    }
}

impl<T> BitOr<T> for Features
where
    T: Into<Features>,
{
    type Output = Features;
    fn bitor(self, rhs: T) -> Self::Output {
        Features::Or(Box::new(self), Box::new(rhs.into()))
    }
}

impl<T> BitAnd<T> for Features
where
    T: Into<Features>,
{
    type Output = Features;
    fn bitand(self, rhs: T) -> Self::Output {
        Features::And(Box::new(self), Box::new(rhs.into()))
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
    sse2,
    sse3,
    ssse3,
    sse41,
    sse42,
    bmi1,
    bmi2,
    lzcnt,
    popcnt,
    avx,
    avx2,
    avx512f,
    avx512vl,
    avx512dq,
    avx512bitalg,
    avx512vbmi,
    cmpxchg16b,
    fma,
}

/// List all CPU features.
///
/// It is critical that this list contains _all_ variants of the [`Feature`]
/// `enum`. We use this list here in the `meta` level so that we can accurately
/// transcribe each variant to an `enum` available in the generated layer above.
/// If this list is incomplete, we will (fortunately) see compile errors for
/// generated functions that use the missing variants.
pub const ALL_FEATURES: &[Feature] = &[
    Feature::_64b,
    Feature::compat,
    Feature::sse,
    Feature::sse2,
    Feature::sse3,
    Feature::ssse3,
    Feature::sse41,
    Feature::sse42,
    Feature::bmi1,
    Feature::bmi2,
    Feature::lzcnt,
    Feature::popcnt,
    Feature::avx,
    Feature::avx2,
    Feature::avx512f,
    Feature::avx512vl,
    Feature::avx512dq,
    Feature::avx512bitalg,
    Feature::avx512vbmi,
    Feature::cmpxchg16b,
    Feature::fma,
];

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl From<Feature> for Features {
    fn from(f: Feature) -> Self {
        Features::Feature(f)
    }
}

impl<T> BitAnd<T> for Feature
where
    T: Into<Features>,
{
    type Output = Features;
    fn bitand(self, rhs: T) -> Self::Output {
        Features::from(self) & rhs.into()
    }
}

impl<T> BitOr<T> for Feature
where
    T: Into<Features>,
{
    type Output = Features;
    fn bitor(self, rhs: T) -> Self::Output {
        Features::from(self) | rhs.into()
    }
}
