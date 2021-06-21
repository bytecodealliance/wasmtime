use crate::cdsl::instructions::{InstructionGroupBuilder, InstructionPredicateMap};
use crate::cdsl::isa::TargetIsa;
use crate::cdsl::recipes::Recipes;
use crate::cdsl::regs::{IsaRegs, IsaRegsBuilder, RegBankBuilder, RegClassBuilder};
use crate::cdsl::settings::{SettingGroup, SettingGroupBuilder};

use crate::shared::Definitions as SharedDefinitions;

fn define_settings(_shared: &SettingGroup) -> SettingGroup {
    let setting = SettingGroupBuilder::new("arm32");
    setting.build()
}

fn define_regs() -> IsaRegs {
    let mut regs = IsaRegsBuilder::new();

    let builder = RegBankBuilder::new("FloatRegs", "s")
        .units(64)
        .track_pressure(true);
    let float_regs = regs.add_bank(builder);

    let builder = RegBankBuilder::new("IntRegs", "r")
        .units(16)
        .track_pressure(true);
    let int_regs = regs.add_bank(builder);

    let builder = RegBankBuilder::new("FlagRegs", "")
        .units(1)
        .names(vec!["nzcv"])
        .track_pressure(false);
    let flag_reg = regs.add_bank(builder);

    let builder = RegClassBuilder::new_toplevel("S", float_regs).count(32);
    regs.add_class(builder);

    let builder = RegClassBuilder::new_toplevel("D", float_regs).width(2);
    regs.add_class(builder);

    let builder = RegClassBuilder::new_toplevel("Q", float_regs).width(4);
    regs.add_class(builder);

    let builder = RegClassBuilder::new_toplevel("GPR", int_regs);
    regs.add_class(builder);

    let builder = RegClassBuilder::new_toplevel("FLAG", flag_reg);
    regs.add_class(builder);

    regs.build()
}

pub(crate) fn define(shared_defs: &mut SharedDefinitions) -> TargetIsa {
    let settings = define_settings(&shared_defs.settings);
    let regs = define_regs();

    let inst_group = InstructionGroupBuilder::new(&mut shared_defs.all_instructions).build();

    let cpu_modes = vec![];

    // TODO implement arm32 recipes.
    let recipes = Recipes::new();

    // TODO implement arm32 encodings and predicates.
    let encodings_predicates = InstructionPredicateMap::new();

    TargetIsa::new(
        "arm32",
        inst_group,
        settings,
        regs,
        recipes,
        cpu_modes,
        encodings_predicates,
    )
}
