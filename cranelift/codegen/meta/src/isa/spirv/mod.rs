use crate::cdsl::cpu_modes::CpuMode;
use crate::cdsl::instructions::{InstructionGroupBuilder, InstructionPredicateMap};
use crate::cdsl::isa::TargetIsa;
use crate::cdsl::recipes::Recipes;
use crate::cdsl::regs::{IsaRegs, IsaRegsBuilder, RegBankBuilder, RegClassBuilder};
use crate::cdsl::settings::SettingGroupBuilder;

use crate::shared::Definitions as SharedDefinitions;

pub(crate) fn define_recipes(_shared_defs: &SharedDefinitions, _regs: &IsaRegs) -> Recipes {
    // Register classes shorthands.
    // let formats = &shared_defs.formats;
    // let gpr = regs.class_by_name("GPR");

    Recipes::new()
}

fn define_registers() -> IsaRegs {
    let mut regs = IsaRegsBuilder::new();

    let builder = RegBankBuilder::new("VirtualRegs", "")
        .units(255) // jb-todo: spirv's registers are virtual...
        .track_pressure(true);
    let int_regs = regs.add_bank(builder);

    let builder = RegClassBuilder::new_toplevel("GPR", int_regs);
    regs.add_class(builder);

    regs.build()
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
