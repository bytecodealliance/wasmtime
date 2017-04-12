//! Source map for translating source entity names to parsed entities.
//!
//! When the parser reads in a source file, entities like instructions, EBBs, and values get new
//! entity numbers. The parser maintains a mapping from the entity names in the source to the final
//! entity references.
//!
//! The `SourceMap` struct defined in this module makes the same mapping available to parser
//! clients.

use std::collections::HashMap;
use cretonne::ir::{StackSlot, JumpTable, Ebb, Value, SigRef, FuncRef};
use cretonne::ir::entities::AnyEntity;
use error::{Result, Location};
use lexer::split_entity_name;

/// Mapping from source entity names to entity references that are valid in the parsed function.
#[derive(Debug)]
pub struct SourceMap {
    values: HashMap<Value, Value>, // vNN
    ebbs: HashMap<Ebb, Ebb>, // ebbNN
    stack_slots: HashMap<u32, StackSlot>, // ssNN
    signatures: HashMap<u32, SigRef>, // sigNN
    functions: HashMap<u32, FuncRef>, // fnNN
    jump_tables: HashMap<u32, JumpTable>, // jtNN

    // Store locations for entities, including instructions.
    locations: HashMap<AnyEntity, Location>,
}

/// Read-only interface which is exposed outside the parser crate.
impl SourceMap {
    /// Look up a value entity by its source number.
    pub fn get_value(&self, src: Value) -> Option<Value> {
        self.values.get(&src).cloned()
    }

    /// Look up a EBB entity by its source number.
    pub fn get_ebb(&self, src: Ebb) -> Option<Ebb> {
        self.ebbs.get(&src).cloned()
    }

    /// Look up a stack slot entity by its source number.
    pub fn get_ss(&self, src_num: u32) -> Option<StackSlot> {
        self.stack_slots.get(&src_num).cloned()
    }

    /// Look up a signature entity by its source number.
    pub fn get_sig(&self, src_num: u32) -> Option<SigRef> {
        self.signatures.get(&src_num).cloned()
    }

    /// Look up a function entity by its source number.
    pub fn get_fn(&self, src_num: u32) -> Option<FuncRef> {
        self.functions.get(&src_num).cloned()
    }

    /// Look up a jump table entity by its source number.
    pub fn get_jt(&self, src_num: u32) -> Option<JumpTable> {
        self.jump_tables.get(&src_num).cloned()
    }

    /// Look up an entity by source name.
    /// Returns the entity reference corresponding to `name`, if it exists.
    pub fn lookup_str(&self, name: &str) -> Option<AnyEntity> {
        split_entity_name(name).and_then(|(ent, num)| match ent {
                                             "v" => {
                                                 Value::with_number(num)
                                                     .and_then(|v| self.get_value(v))
                                                     .map(AnyEntity::Value)
                                             }
                                             "ebb" => {
                                                 Ebb::with_number(num)
                                                     .and_then(|e| self.get_ebb(e))
                                                     .map(AnyEntity::Ebb)
                                             }
                                             "ss" => self.get_ss(num).map(AnyEntity::StackSlot),
                                             "sig" => self.get_sig(num).map(AnyEntity::SigRef),
                                             "fn" => self.get_fn(num).map(AnyEntity::FuncRef),
                                             "jt" => self.get_jt(num).map(AnyEntity::JumpTable),
                                             _ => None,
                                         })
    }

    /// Get the source location where an entity was defined.
    /// This looks up entities in the parsed function, not the source entity numbers.
    pub fn location(&self, entity: AnyEntity) -> Option<Location> {
        self.locations.get(&entity).cloned()
    }

    /// Rewrite an Ebb reference.
    pub fn rewrite_ebb(&self, ebb: &mut Ebb, loc: AnyEntity) -> Result<()> {
        match self.get_ebb(*ebb) {
            Some(new) => {
                *ebb = new;
                Ok(())
            }
            None => {
                err!(self.location(loc).unwrap_or_default(),
                     "undefined reference: {}",
                     ebb)
            }
        }
    }

    /// Rewrite a value reference.
    pub fn rewrite_value(&self, val: &mut Value, loc: AnyEntity) -> Result<()> {
        match self.get_value(*val) {
            Some(new) => {
                *val = new;
                Ok(())
            }
            None => {
                err!(self.location(loc).unwrap_or_default(),
                     "undefined reference: {}",
                     val)
            }
        }
    }

    /// Rewrite a slice of value references.
    pub fn rewrite_values(&self, vals: &mut [Value], loc: AnyEntity) -> Result<()> {
        for val in vals {
            self.rewrite_value(val, loc)?;
        }
        Ok(())
    }
}


/// Interface for mutating a source map.
///
/// This interface is provided for the parser itself, it is not made available outside the crate.
pub trait MutableSourceMap {
    fn new() -> Self;

    /// Define a value mapping from the source name `src` to the final `entity`.
    fn def_value(&mut self, src: Value, entity: Value, loc: &Location) -> Result<()>;
    fn def_ebb(&mut self, src: Ebb, entity: Ebb, loc: &Location) -> Result<()>;
    fn def_ss(&mut self, src_num: u32, entity: StackSlot, loc: &Location) -> Result<()>;
    fn def_sig(&mut self, src_num: u32, entity: SigRef, loc: &Location) -> Result<()>;
    fn def_fn(&mut self, src_num: u32, entity: FuncRef, loc: &Location) -> Result<()>;
    fn def_jt(&mut self, src_num: u32, entity: JumpTable, loc: &Location) -> Result<()>;

    /// Define an entity without an associated source number. This can be used for instructions
    /// whose numbers never appear in source, or implicitly defined signatures.
    fn def_entity(&mut self, entity: AnyEntity, loc: &Location) -> Result<()>;
}

impl MutableSourceMap for SourceMap {
    fn new() -> SourceMap {
        SourceMap {
            values: HashMap::new(),
            ebbs: HashMap::new(),
            stack_slots: HashMap::new(),
            signatures: HashMap::new(),
            functions: HashMap::new(),
            jump_tables: HashMap::new(),
            locations: HashMap::new(),
        }
    }

    fn def_value(&mut self, src: Value, entity: Value, loc: &Location) -> Result<()> {
        if self.values.insert(src, entity).is_some() {
            err!(loc, "duplicate value: {}", src)
        } else {
            self.def_entity(entity.into(), loc)
        }
    }

    fn def_ebb(&mut self, src: Ebb, entity: Ebb, loc: &Location) -> Result<()> {
        if self.ebbs.insert(src, entity).is_some() {
            err!(loc, "duplicate EBB: {}", src)
        } else {
            self.def_entity(entity.into(), loc)
        }
    }

    fn def_ss(&mut self, src_num: u32, entity: StackSlot, loc: &Location) -> Result<()> {
        if self.stack_slots.insert(src_num, entity).is_some() {
            err!(loc, "duplicate stack slot: ss{}", src_num)
        } else {
            self.def_entity(entity.into(), loc)
        }
    }

    fn def_sig(&mut self, src_num: u32, entity: SigRef, loc: &Location) -> Result<()> {
        if self.signatures.insert(src_num, entity).is_some() {
            err!(loc, "duplicate signature: sig{}", src_num)
        } else {
            self.def_entity(entity.into(), loc)
        }
    }

    fn def_fn(&mut self, src_num: u32, entity: FuncRef, loc: &Location) -> Result<()> {
        if self.functions.insert(src_num, entity).is_some() {
            err!(loc, "duplicate function: fn{}", src_num)
        } else {
            self.def_entity(entity.into(), loc)
        }
    }

    fn def_jt(&mut self, src_num: u32, entity: JumpTable, loc: &Location) -> Result<()> {
        if self.jump_tables.insert(src_num, entity).is_some() {
            err!(loc, "duplicate jump table: jt{}", src_num)
        } else {
            self.def_entity(entity.into(), loc)
        }
    }

    fn def_entity(&mut self, entity: AnyEntity, loc: &Location) -> Result<()> {
        if self.locations.insert(entity, loc.clone()).is_some() {
            err!(loc, "duplicate entity: {}", entity)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use parse_test;

    #[test]
    fn details() {
        let tf = parse_test("function detail() {
                               ss10 = stack_slot 13
                               jt10 = jump_table ebb0
                             ebb0(v4: i32, v7: i32):
                               v10 = iadd v4, v7
                             }")
                .unwrap();
        let map = &tf.functions[0].1.map;

        assert_eq!(map.lookup_str("v0"), None);
        assert_eq!(map.lookup_str("ss1"), None);
        assert_eq!(map.lookup_str("ss10").unwrap().to_string(), "ss0");
        assert_eq!(map.lookup_str("jt10").unwrap().to_string(), "jt0");
        assert_eq!(map.lookup_str("ebb0").unwrap().to_string(), "ebb0");
        assert_eq!(map.lookup_str("v4").unwrap().to_string(), "v0");
        assert_eq!(map.lookup_str("v7").unwrap().to_string(), "v1");
        assert_eq!(map.lookup_str("v10").unwrap().to_string(), "v2");
    }
}
