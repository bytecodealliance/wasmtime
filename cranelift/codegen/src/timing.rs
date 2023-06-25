//! Pass timing.
//!
//! This modules provides facilities for timing the execution of individual compilation passes.

use core::fmt;
use std::any::Any;
use std::boxed::Box;
use std::cell::RefCell;

use crate::default_profiler::DefaultProfiler;

// Each pass that can be timed is predefined with the `define_passes!` macro. Each pass has a
// snake_case name and a plain text description used when printing out the timing report.
//
// This macro defines:
//
// - A C-style enum containing all the pass names and a `None` variant.
// - A usize constant with the number of defined passes.
// - A const array of pass descriptions.
// - A public function per pass used to start the timing of that pass.
macro_rules! define_passes {
    ($($pass:ident: $desc:expr,)+) => {
        /// A single profiled pass.
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Pass {
            $(#[doc=$desc] $pass,)+
            /// No active pass.
            None,
        }

        /// The amount of profiled passes.
        pub const NUM_PASSES: usize = Pass::None as usize;

        const DESCRIPTIONS: [&str; NUM_PASSES] = [ $($desc),+ ];

        $(
            #[doc=$desc]
            #[must_use]
            pub fn $pass() -> Box<dyn Any> {
                start_pass(Pass::$pass)
            }
        )+

        impl Pass {
            /// Turn an index back into a pass identifier.
            #[allow(non_upper_case_globals)]
            pub fn from_idx(idx: usize) -> Self {
                $(
                    const $pass: usize = Pass::$pass as usize;
                )+
                match idx {
                    $(
                        $pass => Pass::$pass,
                    )+
                    _ => panic!("Invalid index {idx}"),
                }
            }
        }
    }
}

// Pass definitions.
define_passes! {
    // All these are used in other crates but defined here so they appear in the unified
    // `PassTimes` output.
    process_file: "Processing test file",
    parse_text: "Parsing textual Cranelift IR",
    wasm_translate_module: "Translate WASM module",
    wasm_translate_function: "Translate WASM function",

    verifier: "Verify Cranelift IR",

    compile: "Compilation passes",
    try_incremental_cache: "Try loading from incremental cache",
    store_incremental_cache: "Store in incremental cache",
    flowgraph: "Control flow graph",
    domtree: "Dominator tree",
    loop_analysis: "Loop analysis",
    preopt: "Pre-legalization rewriting",
    dce: "Dead code elimination",
    egraph: "Egraph based optimizations",
    gvn: "Global value numbering",
    licm: "Loop invariant code motion",
    unreachable_code: "Remove unreachable blocks",
    remove_constant_phis: "Remove constant phi-nodes",

    vcode_lower: "VCode lowering",
    vcode_emit: "VCode emission",
    vcode_emit_finish: "VCode emission finalization",

    regalloc: "Register allocation",
    regalloc_checker: "Register allocation symbolic verification",
    layout_renumber: "Layout full renumbering",

    canonicalize_nans: "Canonicalization of NaNs",
}

impl Pass {
    /// A number that can be used to index into a dense array of pass timings.
    ///
    /// The exact return value is not guaranteed to be stable.
    pub fn idx(self) -> usize {
        self as usize
    }

    /// Description of the pass.
    pub fn description(self) -> &'static str {
        match DESCRIPTIONS.get(self.idx()) {
            Some(s) => s,
            None => "<no pass>",
        }
    }
}

impl fmt::Display for Pass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}

/// A profiler.
pub trait Profiler {
    /// Start a profiling pass.
    ///
    /// Will return a token which when dropped indicates the end of the pass.
    ///
    /// Multiple passes can be active at the same time, but they must be started and stopped in a
    /// LIFO fashion.
    fn start_pass(&self, pass: Pass) -> Box<dyn Any>;
}

// Information about passes in a single thread.
thread_local! {
    static PROFILER: RefCell<Box<dyn Profiler>> = RefCell::new(Box::new(DefaultProfiler));
}

/// Set the profiler for the current thread.
///
/// Returns the old profiler.
pub fn set_thread_profiler(new_profiler: Box<dyn Profiler>) -> Box<dyn Profiler> {
    PROFILER.with(|profiler| std::mem::replace(&mut *profiler.borrow_mut(), new_profiler))
}

/// Start timing `pass` as a child of the currently running pass, if any.
///
/// This function is called by the publicly exposed pass functions.
fn start_pass(pass: Pass) -> Box<dyn Any> {
    PROFILER.with(|profiler| profiler.borrow().start_pass(pass))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn display() {
        assert_eq!(Pass::None.to_string(), "<no pass>");
        assert_eq!(Pass::regalloc.to_string(), "Register allocation");
    }
}
