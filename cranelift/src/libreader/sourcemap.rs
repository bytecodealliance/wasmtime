//! Source map for translating source entity names to parsed entities.
//!
//! When the parser reads in a source file, entities like instructions, EBBs, and values get new
//! entity numbers. The parser maintains a mapping from the entity names in the source to the final
//! entity references.
//!
//! The `SourceMap` struct defined in this module makes the same mapping available to parser
//! clients.

use std::collections::HashMap;
use cretonne::ir::{StackSlot, JumpTable, Ebb, Value};
use cretonne::ir::entities::AnyEntity;

/// Mapping from source entity names to entity references that are valid in the parsed function.
#[derive(Debug)]
pub struct SourceMap {
    values: HashMap<Value, Value>, // vNN, vxNN
    ebbs: HashMap<Ebb, Ebb>, // ebbNN
    stack_slots: HashMap<u32, StackSlot>, // ssNN
    jump_tables: HashMap<u32, JumpTable>, // jtNN
}

/// Read-only interface which is exposed outside the parser crate.
impl SourceMap {
    /// Look up an entity by source name.
    /// Returns the entity reference corresponding to `name`, if it exists.
    pub fn lookup_str(&self, name: &str) -> Option<AnyEntity> {
        split_entity_name(name).and_then(|(ent, num)| {
            match ent {
                "v" => {
                    Value::direct_with_number(num)
                        .and_then(|v| self.values.get(&v).cloned())
                        .map(AnyEntity::Value)
                }
                "vx" => {
                    Value::table_with_number(num)
                        .and_then(|v| self.values.get(&v).cloned())
                        .map(AnyEntity::Value)
                }
                "ebb" => {
                    Ebb::with_number(num)
                        .and_then(|e| self.ebbs.get(&e).cloned())
                        .map(AnyEntity::Ebb)
                }
                "ss" => self.stack_slots.get(&num).cloned().map(AnyEntity::StackSlot),
                "jt" => self.jump_tables.get(&num).cloned().map(AnyEntity::JumpTable),
                _ => None,
            }
        })
    }
}

/// Get the number of decimal digits at the end of `s`.
fn trailing_digits(s: &str) -> usize {
    // It's faster to iterate backwards over bytes, and we're only counting ASCII digits.
    s.as_bytes().iter().rev().cloned().take_while(|&b| b'0' <= b && b <= b'9').count()
}

/// Pre-parse a supposed entity name by splitting it into two parts: A head of lowercase ASCII
/// letters and numeric tail.
fn split_entity_name(name: &str) -> Option<(&str, u32)> {
    let (head, tail) = name.split_at(name.len() - trailing_digits(name));
    if tail.len() > 1 && tail.starts_with('0') {
        None
    } else {
        tail.parse().ok().map(|n| (head, n))
    }
}

/// Create a new SourceMap from all the individual mappings.
pub fn new(values: HashMap<Value, Value>,
           ebbs: HashMap<Ebb, Ebb>,
           stack_slots: HashMap<u32, StackSlot>,
           jump_tables: HashMap<u32, JumpTable>)
           -> SourceMap {
    SourceMap {
        values: values,
        ebbs: ebbs,
        stack_slots: stack_slots,
        jump_tables: jump_tables,
    }
}

#[cfg(test)]
mod tests {
    use super::{trailing_digits, split_entity_name};
    use parse_test;

    #[test]
    fn digits() {
        assert_eq!(trailing_digits(""), 0);
        assert_eq!(trailing_digits("x"), 0);
        assert_eq!(trailing_digits("0x"), 0);
        assert_eq!(trailing_digits("x1"), 1);
        assert_eq!(trailing_digits("1x1"), 1);
        assert_eq!(trailing_digits("1x01"), 2);
    }

    #[test]
    fn entity_name() {
        assert_eq!(split_entity_name(""), None);
        assert_eq!(split_entity_name("x"), None);
        assert_eq!(split_entity_name("x+"), None);
        assert_eq!(split_entity_name("x+1"), Some(("x+", 1)));
        assert_eq!(split_entity_name("x-1"), Some(("x-", 1)));
        assert_eq!(split_entity_name("1"), Some(("", 1)));
        assert_eq!(split_entity_name("x1"), Some(("x", 1)));
        assert_eq!(split_entity_name("xy0"), Some(("xy", 0)));
        // Reject this non-canonical form.
        assert_eq!(split_entity_name("inst01"), None);
    }

    #[test]
    fn details() {
        let tf = parse_test("function detail() {
                               ss10 = stack_slot 13
                               jt10 = jump_table ebb0
                             ebb0(v4: i32, vx7: i32):
                               v10 = iadd v4, vx7
                             }")
            .unwrap();
        let map = &tf.functions[0].1.map;

        assert_eq!(map.lookup_str("v0"), None);
        assert_eq!(map.lookup_str("ss1"), None);
        assert_eq!(map.lookup_str("ss10").unwrap().to_string(), "ss0");
        assert_eq!(map.lookup_str("jt10").unwrap().to_string(), "jt0");
        assert_eq!(map.lookup_str("ebb0").unwrap().to_string(), "ebb0");
        assert_eq!(map.lookup_str("v4").unwrap().to_string(), "vx0");
        assert_eq!(map.lookup_str("vx7").unwrap().to_string(), "vx1");
        assert_eq!(map.lookup_str("v10").unwrap().to_string(), "v0");
    }
}
