//! CPU features.
//!
//! Across generations, CPUs add features that include new instructions, e.g.,
//! [`Feature::sse`], [`Feature::avx`], etc. x64 instructions are governed by
//! boolean terms of these CPU features: e.g., `(_64b | compat) & ssse3`. These
//! terms are defined per instruction in the `meta` crate and are exposed to
//! users in two ways:
//! - via the [`Inst::is_available`] function, which uses the
//!   [`AvailableFeatures`] trait below to query "is this instruction currently
//!   allowed?"; use this for fast checks during compilation
//! - via the [`Inst::features`] function, which returns a fully-constructed
//!   [`Features`] term; use this for time-insensitive analysis or
//!   pretty-printing.
//!
//! ```rust
//! # use cranelift_assembler_x64::{Registers, inst};
//! # pub struct Regs;
//! # impl Registers for Regs {
//! #     type ReadGpr = u8;
//! #     type ReadWriteGpr = u8;
//! #     type WriteGpr = u8;
//! #     type ReadXmm = u8;
//! #     type ReadWriteXmm = u8;
//! #     type WriteXmm = u8;
//! # }
//! let xmm0: u8 = 0;
//! let andps = inst::andps_a::<Regs>::new(xmm0, xmm0);
//! assert_eq!(andps.features().to_string(), "((_64b | compat) & sse)");
//! ```
//!
//! [`Inst::is_available`]: crate::inst::Inst::is_available
//! [`Inst::features`]: crate::inst::Inst::features

use crate::inst::for_each_feature;
use std::fmt;

// Helpfully generate `enum Feature`.
macro_rules! create_feature_enum {
    ($($f:ident)+) => {
        /// A CPU feature.
        ///
        /// IA-32e mode is the typical mode of operation for modern 64-bit x86
        /// processors. It consists of two sub-modes:
        /// - __64-bit mode__: uses the full 64-bit address space
        /// - __compatibility mode__: allows use of legacy 32-bit code
        ///
        /// Other features listed here should match the __CPUID Feature Flags__
        /// column of the instruction tables of the x64 reference manual.
        ///
        /// This is generated from the `dsl::Feature` enumeration defined in the
        /// `meta` crate; see [`for_each_feature`].
        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        pub enum Feature {
            $($f,)+
        }
    };
}
for_each_feature!(create_feature_enum);

// Helpfully generate trait functions in `AvailableFeatures`.
macro_rules! add_func {
    ($($f:ident)+) => {
        $(fn $f(&self) -> bool;)+
    };
}

/// A trait for querying CPU features.
///
/// This is generated from the `dsl::Feature` enumeration defined in the `meta`
/// crate. It allows querying the CPUID features required by an instruction; see
/// [`Inst::is_available`] and [`for_each_feature`].
///
/// [`Inst::is_available`]: crate::inst::Inst::is_available
pub trait AvailableFeatures {
    for_each_feature!(add_func);
}

/// A boolean term of CPU features.
///
/// An instruction is valid when the boolean term (a recursive tree of `AND` and
/// `OR` terms) is satisfied; see [`Inst::features`].
///
/// [`Inst::features`]: crate::inst::Inst::features
pub enum Features {
    And(&'static Features, &'static Features),
    Or(&'static Features, &'static Features),
    Feature(Feature),
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
