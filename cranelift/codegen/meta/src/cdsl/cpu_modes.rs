use crate::cdsl::types::LaneType;
use crate::cdsl::xform::{TransformGroup, TransformGroupIndex, TransformGroups};

use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

pub struct CpuMode {
    _name: &'static str,
    default_legalize: Option<TransformGroupIndex>,
    monomorphic_legalize: Option<TransformGroupIndex>,
    typed_legalize: HashMap<String, TransformGroupIndex>,
}

impl CpuMode {
    pub fn new(name: &'static str) -> Self {
        Self {
            _name: name,
            default_legalize: None,
            monomorphic_legalize: None,
            typed_legalize: HashMap::new(),
        }
    }
    pub fn legalize_monomorphic(&mut self, group: &TransformGroup) {
        assert!(self.monomorphic_legalize.is_none());
        self.monomorphic_legalize = Some(group.id);
    }
    pub fn legalize_default(&mut self, group: &TransformGroup) {
        assert!(self.default_legalize.is_none());
        self.default_legalize = Some(group.id);
    }
    pub fn legalize_type(&mut self, lane_type: impl Into<LaneType>, group: &TransformGroup) {
        assert!(self
            .typed_legalize
            .insert(lane_type.into().to_string(), group.id)
            .is_none());
    }

    /// Returns a deterministically ordered, deduplicated list of TransformGroupIndex for the directly
    /// reachable set of TransformGroup this TargetIsa uses.
    pub fn direct_transform_groups(&self) -> Vec<TransformGroupIndex> {
        let mut set = HashSet::new();
        if let Some(i) = &self.default_legalize {
            set.insert(*i);
        }
        if let Some(i) = &self.monomorphic_legalize {
            set.insert(*i);
        }
        set.extend(self.typed_legalize.values().cloned());
        let mut ret = Vec::from_iter(set);
        ret.sort();
        ret
    }
}
