//! Pretty-printing for machine code (virtual-registerized or final).

use regalloc::{RealRegUniverse, Reg, Writable};

use std::fmt::Debug;
use std::hash::Hash;
use std::string::{String, ToString};

// FIXME: Should this go into regalloc.rs instead?

/// A trait for printing instruction bits and pieces, with the the ability to
/// take a contextualising RealRegUniverse that is used to give proper names to
/// registers.
pub trait ShowWithRRU {
    /// Return a string that shows the implementing object in context of the
    /// given `RealRegUniverse`, if provided.
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String;

    /// The same as |show_rru|, but with an optional hint giving a size in
    /// bytes.  Its interpretation is object-dependent, and it is intended to
    /// pass around enough information to facilitate printing sub-parts of
    /// real registers correctly.  Objects may ignore size hints that are
    /// irrelevant to them.
    fn show_rru_sized(&self, mb_rru: Option<&RealRegUniverse>, _size: u8) -> String {
        // Default implementation is to ignore the hint.
        self.show_rru(mb_rru)
    }
}

impl ShowWithRRU for Reg {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        if self.is_real() {
            if let Some(rru) = mb_rru {
                let reg_ix = self.get_index();
                if reg_ix < rru.regs.len() {
                    return rru.regs[reg_ix].1.to_string();
                } else {
                    // We have a real reg which isn't listed in the universe.
                    // Per the regalloc.rs interface requirements, this is
                    // Totally Not Allowed.  Print it generically anyway, so
                    // we have something to debug.
                    return format!("!!{:?}!!", self);
                }
            }
        }
        // The reg is virtual, or we have no universe.  Be generic.
        format!("%{:?}", self)
    }

    fn show_rru_sized(&self, _mb_rru: Option<&RealRegUniverse>, _size: u8) -> String {
        // For the specific case of Reg, we demand not to have a size hint,
        // since interpretation of the size is target specific, but this code
        // is used by all targets.
        panic!("Reg::show_rru_sized: impossible to implement");
    }
}

impl<R: ShowWithRRU + Copy + Ord + Hash + Eq + Debug> ShowWithRRU for Writable<R> {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        self.to_reg().show_rru(mb_rru)
    }

    fn show_rru_sized(&self, mb_rru: Option<&RealRegUniverse>, size: u8) -> String {
        self.to_reg().show_rru_sized(mb_rru, size)
    }
}
