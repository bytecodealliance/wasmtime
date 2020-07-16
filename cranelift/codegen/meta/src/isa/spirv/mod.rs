use crate::cdsl::cpu_modes::CpuMode;
use crate::cdsl::instructions::{InstructionGroupBuilder, InstructionPredicateMap};
use crate::cdsl::isa::TargetIsa;
use crate::cdsl::recipes::Recipes;
use crate::cdsl::regs::{IsaRegs, IsaRegsBuilder};
use crate::cdsl::settings::SettingGroupBuilder;

use crate::shared::Definitions as SharedDefinitions;

pub(crate) fn define_recipes(_shared_defs: &SharedDefinitions, _regs: &IsaRegs) -> Recipes {
    // Register classes shorthands.
    // let formats = &shared_defs.formats;
    // let gpr = regs.class_by_name("GPR");

    Recipes::new()
}

fn define_registers() -> IsaRegs {
    IsaRegsBuilder::new().build()
}

pub(crate) fn define(shared_defs: &mut SharedDefinitions) -> TargetIsa {
    let settings = SettingGroupBuilder::new("spirv").build();

    let regs = define_registers();

    let inst_group = InstructionGroupBuilder::new(&mut shared_defs.all_instructions).build();

    let glcompute = CpuMode::new("GLCompute");

    let recipes = define_recipes(shared_defs, &regs);

    //let encodings = encodings::define(shared_defs, &settings, &recipes);
    //let encodings_predicates = encodings.inst_pred_reg.extract();

    //let recipes = recipes.collect();

    let cpu_modes = vec![glcompute];

    TargetIsa::new(
        "spirv",
        inst_group,
        settings,
        regs,
        recipes,
        cpu_modes,
        InstructionPredicateMap::new(),
    )
}
