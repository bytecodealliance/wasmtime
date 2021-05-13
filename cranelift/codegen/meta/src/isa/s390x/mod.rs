use crate::cdsl::cpu_modes::CpuMode;
use crate::cdsl::instructions::{InstructionGroupBuilder, InstructionPredicateMap};
use crate::cdsl::isa::TargetIsa;
use crate::cdsl::recipes::Recipes;
use crate::cdsl::regs::IsaRegsBuilder;
use crate::cdsl::settings::SettingGroupBuilder;

use crate::shared::Definitions as SharedDefinitions;

pub(crate) fn define(shared_defs: &mut SharedDefinitions) -> TargetIsa {
    let inst_group = InstructionGroupBuilder::new(&mut shared_defs.all_instructions).build();
    let settings = SettingGroupBuilder::new("s390x").build();
    let regs = IsaRegsBuilder::new().build();
    let recipes = Recipes::new();
    let encodings_predicates = InstructionPredicateMap::new();

    let mut mode = CpuMode::new("s390x");
    let expand = shared_defs.transform_groups.by_name("expand");
    mode.legalize_default(expand);
    let cpu_modes = vec![mode];

    TargetIsa::new(
        "s390x",
        inst_group,
        settings,
        regs,
        recipes,
        cpu_modes,
        encodings_predicates,
    )
}
