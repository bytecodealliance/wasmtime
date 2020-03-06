//! Register diversions.
//!
//! Normally, a value is assigned to a single register or stack location by the register allocator.
//! Sometimes, it is necessary to move register values to a different register in order to satisfy
//! instruction constraints.
//!
//! These register diversions are local to a block. No values can be diverted when entering a new
//! block.

use crate::fx::FxHashMap;
use crate::hash_map::{Entry, Iter};
use crate::ir::{Block, StackSlot, Value, ValueLoc, ValueLocations};
use crate::ir::{InstructionData, Opcode};
use crate::isa::{RegInfo, RegUnit};
use core::fmt;
use cranelift_entity::{SparseMap, SparseMapValue};

/// A diversion of a value from its original location to a new register or stack location.
///
/// In IR, a diversion is represented by a `regmove` instruction, possibly a chain of them for the
/// same value.
///
/// When tracking diversions, the `from` field is the original assigned value location, and `to` is
/// the current one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Diversion {
    /// The original value location.
    pub from: ValueLoc,
    /// The current value location.
    pub to: ValueLoc,
}

impl Diversion {
    /// Make a new diversion.
    pub fn new(from: ValueLoc, to: ValueLoc) -> Self {
        debug_assert!(from.is_assigned() && to.is_assigned());
        Self { from, to }
    }
}

/// Keep track of diversions in a block.
#[derive(Clone)]
pub struct RegDiversions {
    current: FxHashMap<Value, Diversion>,
}

/// Keep track of diversions at the entry of block.
#[derive(Clone)]
struct EntryRegDiversionsValue {
    key: Block,
    divert: RegDiversions,
}

/// Map block to their matching RegDiversions at basic blocks entry.
pub struct EntryRegDiversions {
    map: SparseMap<Block, EntryRegDiversionsValue>,
}

impl RegDiversions {
    /// Create a new empty diversion tracker.
    pub fn new() -> Self {
        Self {
            current: FxHashMap::default(),
        }
    }

    /// Clear the content of the diversions, to reset the state of the compiler.
    pub fn clear(&mut self) {
        self.current.clear()
    }

    /// Are there any diversions?
    pub fn is_empty(&self) -> bool {
        self.current.is_empty()
    }

    /// Get the current diversion of `value`, if any.
    pub fn diversion(&self, value: Value) -> Option<&Diversion> {
        self.current.get(&value)
    }

    /// Get all current diversions.
    pub fn iter(&self) -> Iter<'_, Value, Diversion> {
        self.current.iter()
    }

    /// Get the current location for `value`. Fall back to the assignment map for non-diverted
    /// values
    pub fn get(&self, value: Value, locations: &ValueLocations) -> ValueLoc {
        match self.diversion(value) {
            Some(d) => d.to,
            None => locations[value],
        }
    }

    /// Get the current register location for `value`, or panic if `value` isn't in a register.
    pub fn reg(&self, value: Value, locations: &ValueLocations) -> RegUnit {
        self.get(value, locations).unwrap_reg()
    }

    /// Get the current stack location for `value`, or panic if `value` isn't in a stack slot.
    pub fn stack(&self, value: Value, locations: &ValueLocations) -> StackSlot {
        self.get(value, locations).unwrap_stack()
    }

    /// Record any kind of move.
    ///
    /// The `from` location must match an existing `to` location, if any.
    fn divert(&mut self, value: Value, from: ValueLoc, to: ValueLoc) {
        debug_assert!(from.is_assigned() && to.is_assigned());
        match self.current.entry(value) {
            Entry::Occupied(mut e) => {
                // TODO: non-lexical lifetimes should allow removal of the scope and early return.
                {
                    let d = e.get_mut();
                    debug_assert_eq!(d.to, from, "Bad regmove chain for {}", value);
                    if d.from != to {
                        d.to = to;
                        return;
                    }
                }
                e.remove();
            }
            Entry::Vacant(e) => {
                e.insert(Diversion::new(from, to));
            }
        }
    }

    /// Record a register -> register move.
    pub fn regmove(&mut self, value: Value, from: RegUnit, to: RegUnit) {
        self.divert(value, ValueLoc::Reg(from), ValueLoc::Reg(to));
    }

    /// Record a register -> stack move.
    pub fn regspill(&mut self, value: Value, from: RegUnit, to: StackSlot) {
        self.divert(value, ValueLoc::Reg(from), ValueLoc::Stack(to));
    }

    /// Record a stack -> register move.
    pub fn regfill(&mut self, value: Value, from: StackSlot, to: RegUnit) {
        self.divert(value, ValueLoc::Stack(from), ValueLoc::Reg(to));
    }

    /// Apply the effect of `inst`.
    ///
    /// If `inst` is a `regmove`, `regfill`, or `regspill` instruction, update the diversions to
    /// match.
    pub fn apply(&mut self, inst: &InstructionData) {
        match *inst {
            InstructionData::RegMove {
                opcode: Opcode::Regmove,
                arg,
                src,
                dst,
            } => self.regmove(arg, src, dst),
            InstructionData::RegSpill {
                opcode: Opcode::Regspill,
                arg,
                src,
                dst,
            } => self.regspill(arg, src, dst),
            InstructionData::RegFill {
                opcode: Opcode::Regfill,
                arg,
                src,
                dst,
            } => self.regfill(arg, src, dst),
            _ => {}
        }
    }

    /// Drop any recorded move for `value`.
    ///
    /// Returns the `to` location of the removed diversion.
    pub fn remove(&mut self, value: Value) -> Option<ValueLoc> {
        self.current.remove(&value).map(|d| d.to)
    }

    /// Resets the state of the current diversions to the recorded diversions at the entry of the
    /// given `block`. The recoded diversions is available after coloring on `func.entry_diversions`
    /// field.
    pub fn at_block(&mut self, entry_diversions: &EntryRegDiversions, block: Block) {
        self.clear();
        if let Some(entry_divert) = entry_diversions.map.get(block) {
            let iter = entry_divert.divert.current.iter();
            self.current.extend(iter);
        }
    }

    /// Copy the current state of the diversions, and save it for the entry of the `block` given as
    /// argument.
    ///
    /// Note: This function can only be called once on a `Block` with a given `entry_diversions`
    /// argument, otherwise it would panic.
    pub fn save_for_block(&mut self, entry_diversions: &mut EntryRegDiversions, target: Block) {
        // No need to save anything if there is no diversions to be recorded.
        if self.is_empty() {
            return;
        }
        debug_assert!(!entry_diversions.map.contains_key(target));
        let iter = self.current.iter();
        let mut entry_divert = Self::new();
        entry_divert.current.extend(iter);
        entry_diversions.map.insert(EntryRegDiversionsValue {
            key: target,
            divert: entry_divert,
        });
    }

    /// Check that the recorded entry for a given `block` matches what is recorded in the
    /// `entry_diversions`.
    pub fn check_block_entry(&self, entry_diversions: &EntryRegDiversions, target: Block) -> bool {
        let entry_divert = match entry_diversions.map.get(target) {
            Some(entry_divert) => entry_divert,
            None => return self.is_empty(),
        };

        if entry_divert.divert.current.len() != self.current.len() {
            return false;
        }

        for (val, _) in entry_divert.divert.current.iter() {
            if !self.current.contains_key(val) {
                return false;
            }
        }
        true
    }

    /// Return an object that can display the diversions.
    pub fn display<'a, R: Into<Option<&'a RegInfo>>>(&'a self, regs: R) -> DisplayDiversions<'a> {
        DisplayDiversions(&self, regs.into())
    }
}

impl EntryRegDiversions {
    /// Create a new empty entry diversion, to associate diversions to each block entry.
    pub fn new() -> Self {
        Self {
            map: SparseMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}

impl Clone for EntryRegDiversions {
    /// The Clone trait is required by `ir::Function`.
    fn clone(&self) -> Self {
        let mut tmp = Self::new();
        for v in self.map.values() {
            tmp.map.insert(v.clone());
        }
        tmp
    }
}

/// Implement `SparseMapValue`, as required to make use of a `SparseMap` for mapping the entry
/// diversions for each block.
impl SparseMapValue<Block> for EntryRegDiversionsValue {
    fn key(&self) -> Block {
        self.key
    }
}

/// Object that displays register diversions.
pub struct DisplayDiversions<'a>(&'a RegDiversions, Option<&'a RegInfo>);

impl<'a> fmt::Display for DisplayDiversions<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{")?;
        for (value, div) in self.0.current.iter() {
            write!(
                f,
                " {}: {} -> {}",
                value,
                div.from.display(self.1),
                div.to.display(self.1)
            )?
        }
        write!(f, " }}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityRef;
    use crate::ir::Value;

    #[test]
    fn inserts() {
        let mut divs = RegDiversions::new();
        let v1 = Value::new(1);
        let v2 = Value::new(2);

        divs.regmove(v1, 10, 12);
        assert_eq!(
            divs.diversion(v1),
            Some(&Diversion {
                from: ValueLoc::Reg(10),
                to: ValueLoc::Reg(12),
            })
        );
        assert_eq!(divs.diversion(v2), None);

        divs.regmove(v1, 12, 11);
        assert_eq!(divs.diversion(v1).unwrap().to, ValueLoc::Reg(11));
        divs.regmove(v1, 11, 10);
        assert_eq!(divs.diversion(v1), None);
    }
}
