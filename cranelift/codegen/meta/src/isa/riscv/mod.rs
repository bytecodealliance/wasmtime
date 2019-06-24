use crate::cdsl::cpu_modes::CpuMode;
use crate::cdsl::instructions::InstructionGroupBuilder;
use crate::cdsl::isa::TargetIsa;
use crate::cdsl::regs::{IsaRegs, IsaRegsBuilder, RegBankBuilder, RegClassBuilder};
use crate::cdsl::settings::{PredicateNode, SettingGroup, SettingGroupBuilder};

use crate::shared::types::Float::{F32, F64};
use crate::shared::types::Int::{I32, I64};
use crate::shared::Definitions as SharedDefinitions;

mod encodings;
mod recipes;

fn define_settings(shared: &SettingGroup) -> SettingGroup {
    let mut setting = SettingGroupBuilder::new("riscv");

    let supports_m = setting.add_bool(
        "supports_m",
        "CPU supports the 'M' extension (mul/div)",
        false,
    );
    let supports_a = setting.add_bool(
        "supports_a",
        "CPU supports the 'A' extension (atomics)",
        false,
    );
    let supports_f = setting.add_bool(
        "supports_f",
        "CPU supports the 'F' extension (float)",
        false,
    );
    let supports_d = setting.add_bool(
        "supports_d",
        "CPU supports the 'D' extension (double)",
        false,
    );

    let enable_m = setting.add_bool(
        "enable_m",
        "Enable the use of 'M' instructions if available",
        true,
    );

    setting.add_bool(
        "enable_e",
        "Enable the 'RV32E' instruction set with only 16 registers",
        false,
    );

    let shared_enable_atomics = shared.get_bool("enable_atomics");
    let shared_enable_float = shared.get_bool("enable_float");
    let shared_enable_simd = shared.get_bool("enable_simd");

    setting.add_predicate("use_m", predicate!(supports_m && enable_m));
    setting.add_predicate("use_a", predicate!(supports_a && shared_enable_atomics));
    setting.add_predicate("use_f", predicate!(supports_f && shared_enable_float));
    setting.add_predicate("use_d", predicate!(supports_d && shared_enable_float));
    setting.add_predicate(
        "full_float",
        predicate!(shared_enable_simd && supports_f && supports_d),
    );

    setting.build()
}

fn define_registers() -> IsaRegs {
    let mut regs = IsaRegsBuilder::new();

    let builder = RegBankBuilder::new("IntRegs", "x")
        .units(32)
        .track_pressure(true);
    let int_regs = regs.add_bank(builder);

    let builder = RegBankBuilder::new("FloatRegs", "f")
        .units(32)
        .track_pressure(true);
    let float_regs = regs.add_bank(builder);

    let builder = RegClassBuilder::new_toplevel("GPR", int_regs);
    regs.add_class(builder);

    let builder = RegClassBuilder::new_toplevel("FPR", float_regs);
    regs.add_class(builder);

    regs.build()
}

pub fn define(shared_defs: &mut SharedDefinitions) -> TargetIsa {
    let settings = define_settings(&shared_defs.settings);
    let regs = define_registers();

    let inst_group = InstructionGroupBuilder::new(
        "riscv",
        "riscv specific instruction set",
        &mut shared_defs.all_instructions,
        &shared_defs.format_registry,
    )
    .build();

    // CPU modes for 32-bit and 64-bit operation.
    let mut rv_32 = CpuMode::new("RV32");
    let mut rv_64 = CpuMode::new("RV64");

    let expand = shared_defs.transform_groups.by_name("expand");
    let narrow = shared_defs.transform_groups.by_name("narrow");
    rv_32.legalize_monomorphic(expand);
    rv_32.legalize_default(narrow);
    rv_32.legalize_type(I32, expand);
    rv_32.legalize_type(F32, expand);
    rv_32.legalize_type(F64, expand);

    rv_64.legalize_monomorphic(expand);
    rv_64.legalize_default(narrow);
    rv_64.legalize_type(I32, expand);
    rv_64.legalize_type(I64, expand);
    rv_64.legalize_type(F32, expand);
    rv_64.legalize_type(F64, expand);

    let recipes = recipes::define(shared_defs, &regs);

    let encodings = encodings::define(shared_defs, &settings, &recipes);
    rv_32.set_encodings(encodings.enc32);
    rv_64.set_encodings(encodings.enc64);
    let encodings_predicates = encodings.inst_pred_reg.extract();

    let recipes = recipes.collect();

    let cpu_modes = vec![rv_32, rv_64];

    TargetIsa::new(
        "riscv",
        inst_group,
        settings,
        regs,
        recipes,
        cpu_modes,
        encodings_predicates,
    )
}
