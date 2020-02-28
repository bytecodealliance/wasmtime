use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::{Arc, RwLock};
use wasmtime_environ::ir;

/// The registry maintains descriptions of traps in currently allocated functions.
#[derive(Default)]
pub struct TrapRegistry {
    // This data structure is intended to be safe to use across many threads
    // since this is stored inside of a `Compiler` which, eventually, will be
    // used across many threads. To that end this is internally use an `Arc`
    // plus an `RwLock`.
    //
    // The problem that this data structure is solving is that when a
    // segfault/illegal instruction happens we need to answer "given this
    // hardware program counter what is the wasm reason this trap is being
    // raised"?
    //
    // The way this is answered here is done to minimize the amount of
    // synchronization (in theory) and have something like so:
    //
    // * Each module bulk-registers a list of in-memory pc addresses that have
    //   traps. We assume that the range of traps for each module are always
    //   disjoint.
    // * Each key in this `BTreeMap` is the highest trapping address and the
    //   value contains the lowest address as well as all the individual
    //   addresses in their own `HashMap`.
    // * Registration then looks by calculating the start/end and inserting
    //   into this map (with some assertions about disjointed-ness)
    // * Lookup is done in two layers. First we find the corresponding entry
    //   in the map and verify that a program counter falls in the start/end
    //   range. Next we look up the address in the `traps` hash map below.
    //
    // The `register_traps` function works by returning an RAII guard that owns
    // a handle to this `Arc` as well, and when that type is dropped it will
    // automatically remove all trap information from this `ranges` list.
    ranges: Arc<RwLock<BTreeMap<usize, TrapGroup>>>,
}

#[derive(Debug)]
struct TrapGroup {
    /// The lowest key in the `trap` field.
    ///
    /// This represents the start of the range of this group of traps, and the
    /// end of the range for this group of traps is stored as the key in the
    /// `ranges` struct above in `TrapRegistry`.
    start: usize,

    /// All known traps in this group, mapped from program counter to the
    /// description of the trap itself.
    traps: HashMap<usize, TrapDescription>,
}

/// RAII structure returned from `TrapRegistry::register_trap` to unregister
/// trap information on drop.
#[derive(Clone)]
pub struct TrapRegistration {
    ranges: Arc<RwLock<BTreeMap<usize, TrapGroup>>>,
    end: Option<usize>,
}

/// Description of a trap.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TrapDescription {
    /// Location of the trap in source binary module.
    pub source_loc: ir::SourceLoc,
    /// Code of the trap.
    pub trap_code: ir::TrapCode,
}

impl fmt::Display for TrapDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "wasm trap: {}, source location: {}",
            trap_code_to_expected_string(self.trap_code),
            self.source_loc
        )
    }
}

fn trap_code_to_expected_string(trap_code: ir::TrapCode) -> String {
    use ir::TrapCode::*;
    match trap_code {
        StackOverflow => "call stack exhausted".to_string(),
        HeapOutOfBounds => "out of bounds memory access".to_string(),
        TableOutOfBounds => "undefined element: out of bounds".to_string(),
        OutOfBounds => "out of bounds".to_string(), // Note: not covered by the test suite
        IndirectCallToNull => "uninitialized element".to_string(),
        BadSignature => "indirect call type mismatch".to_string(),
        IntegerOverflow => "integer overflow".to_string(),
        IntegerDivisionByZero => "integer divide by zero".to_string(),
        BadConversionToInteger => "invalid conversion to integer".to_string(),
        UnreachableCodeReached => "unreachable".to_string(),
        Interrupt => "interrupt".to_string(), // Note: not covered by the test suite
        User(x) => format!("user trap {}", x), // Note: not covered by the test suite
    }
}

impl TrapRegistry {
    /// Registers a list of traps.
    ///
    /// Returns a RAII guard that deregisters all traps when dropped.
    pub fn register_traps(
        &self,
        list: impl IntoIterator<Item = (usize, ir::SourceLoc, ir::TrapCode)>,
    ) -> TrapRegistration {
        let mut start = usize::max_value();
        let mut end = 0;
        let mut traps = HashMap::new();
        for (addr, source_loc, trap_code) in list.into_iter() {
            traps.insert(
                addr,
                TrapDescription {
                    source_loc,
                    trap_code,
                },
            );
            if addr < start {
                start = addr;
            }
            if addr > end {
                end = addr;
            }
        }
        if traps.len() == 0 {
            return TrapRegistration {
                ranges: self.ranges.clone(),
                end: None,
            };
        }
        let mut ranges = self.ranges.write().unwrap();

        // Sanity check that no other group of traps overlaps with our
        // registration...
        if let Some((_, prev)) = ranges.range(end..).next() {
            assert!(prev.start > end);
        }
        if let Some((prev_end, _)) = ranges.range(..=start).next_back() {
            assert!(*prev_end < start);
        }

        // ... and then register ourselves
        assert!(ranges.insert(end, TrapGroup { start, traps }).is_none());
        TrapRegistration {
            ranges: self.ranges.clone(),
            end: Some(end),
        }
    }
}

impl TrapRegistration {
    /// Gets a trap description at given address.
    pub fn get_trap(&self, address: usize) -> Option<TrapDescription> {
        let ranges = self.ranges.read().ok()?;
        let (end, group) = ranges.range(address..).next()?;
        if group.start <= address && address <= *end {
            group.traps.get(&address).copied()
        } else {
            None
        }
    }
}

impl Drop for TrapRegistration {
    fn drop(&mut self) {
        if let Some(end) = self.end {
            if let Ok(mut ranges) = self.ranges.write() {
                ranges.remove(&end);
            }
        }
    }
}
