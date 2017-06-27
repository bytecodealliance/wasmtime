//! Register diversions.
//!
//! Normally, a value is assigned to a single register or stack location by the register allocator.
//! Sometimes, it is necessary to move register values to a different register in order to satisfy
//! instruction constraints.
//!
//! These register diversions are local to an EBB. No values can be diverted when entering a new
//! EBB.

use entity_map::EntityMap;
use ir::{Value, ValueLoc};
use isa::RegUnit;

/// A diversion of a value from its original register location to a new register.
///
/// In IL, a diversion is represented by a `regmove` instruction, possibly a chain of them for the
/// same value.
///
/// When tracking diversions, the `from` field is the original assigned value location, and `to` is
/// the current one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Diversion {
    /// The value that is diverted.
    pub value: Value,
    /// The original register value location.
    pub from: RegUnit,
    /// The current register value location.
    pub to: RegUnit,
}

impl Diversion {
    /// Make a new register diversion.
    pub fn new(value: Value, from: RegUnit, to: RegUnit) -> Diversion {
        Diversion { value, from, to }
    }
}

/// Keep track of register diversions in an EBB.
pub struct RegDiversions {
    current: Vec<Diversion>,
}

impl RegDiversions {
    /// Create a new empty diversion tracker.
    pub fn new() -> RegDiversions {
        RegDiversions { current: Vec::new() }
    }

    /// Clear the tracker, preparing for a new EBB.
    pub fn clear(&mut self) {
        self.current.clear()
    }

    /// Are there any diversions?
    pub fn is_empty(&self) -> bool {
        self.current.is_empty()
    }

    /// Get the current diversion of `value`, if any.
    pub fn diversion(&self, value: Value) -> Option<&Diversion> {
        self.current.iter().find(|d| d.value == value)
    }

    /// Get all current diversions.
    pub fn all(&self) -> &[Diversion] {
        self.current.as_slice()
    }

    /// Get the current register location for `value`. Fall back to the assignment map for
    /// non-diverted values.
    pub fn reg(&self, value: Value, locations: &EntityMap<Value, ValueLoc>) -> RegUnit {
        match self.diversion(value) {
            Some(d) => d.to,
            None => locations[value].unwrap_reg(),
        }
    }

    /// Record a register move.
    pub fn regmove(&mut self, value: Value, from: RegUnit, to: RegUnit) {
        if let Some(i) = self.current.iter().position(|d| d.value == value) {
            debug_assert_eq!(self.current[i].to, from, "Bad regmove chain for {}", value);
            if self.current[i].from != to {
                self.current[i].to = to;
            } else {
                self.current.swap_remove(i);
            }
        } else {
            self.current.push(Diversion::new(value, from, to));
        }
    }

    /// Drop any recorded register move for `value`.
    ///
    /// Returns the `to` register of the removed diversion.
    pub fn remove(&mut self, value: Value) -> Option<RegUnit> {
        self.current
            .iter()
            .position(|d| d.value == value)
            .map(|i| self.current.swap_remove(i).to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir::Value;
    use entity_ref::EntityRef;

    #[test]
    fn inserts() {
        let mut divs = RegDiversions::new();
        let v1 = Value::new(1);
        let v2 = Value::new(2);

        divs.regmove(v1, 10, 12);
        assert_eq!(divs.diversion(v1),
                   Some(&Diversion {
                            value: v1,
                            from: 10,
                            to: 12,
                        }));
        assert_eq!(divs.diversion(v2), None);

        divs.regmove(v1, 12, 11);
        assert_eq!(divs.diversion(v1).unwrap().to, 11);
        divs.regmove(v1, 11, 10);
        assert_eq!(divs.diversion(v1), None);
    }
}
