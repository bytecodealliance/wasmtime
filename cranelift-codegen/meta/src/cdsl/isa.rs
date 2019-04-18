use crate::cdsl::cpu_modes::CpuMode;
use crate::cdsl::inst::InstructionGroup;
use crate::cdsl::regs::IsaRegs;
use crate::cdsl::settings::SettingGroup;
use crate::cdsl::xform::{TransformGroupIndex, TransformGroups};

use std::collections::HashSet;
use std::iter::FromIterator;

pub struct TargetIsa {
    pub name: &'static str,
    pub instructions: InstructionGroup,
    pub settings: SettingGroup,
    pub regs: IsaRegs,
    pub cpu_modes: Vec<CpuMode>,
}

impl TargetIsa {
    pub fn new(
        name: &'static str,
        instructions: InstructionGroup,
        settings: SettingGroup,
        regs: IsaRegs,
        cpu_modes: Vec<CpuMode>,
    ) -> Self {
        Self {
            name,
            instructions,
            settings,
            regs,
            cpu_modes,
        }
    }

    /// Returns a deterministically ordered, deduplicated list of TransformGroupIndex for the
    /// transitive set of TransformGroup this TargetIsa uses.
    pub fn transitive_transform_groups(
        &self,
        all_groups: &TransformGroups,
    ) -> Vec<TransformGroupIndex> {
        let mut set = HashSet::new();
        for cpu_mode in &self.cpu_modes {
            set.extend(cpu_mode.transitive_transform_groups(all_groups));
        }
        let mut vec = Vec::from_iter(set);
        vec.sort();
        vec
    }

    /// Returns a deterministically ordered, deduplicated list of TransformGroupIndex for the directly
    /// reachable set of TransformGroup this TargetIsa uses.
    pub fn direct_transform_groups(&self) -> Vec<TransformGroupIndex> {
        let mut set = HashSet::new();
        for cpu_mode in &self.cpu_modes {
            set.extend(cpu_mode.direct_transform_groups());
        }
        let mut vec = Vec::from_iter(set);
        vec.sort();
        vec
    }
}
