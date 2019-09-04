use crate::cdsl::cpu_modes::CpuMode;
use crate::cdsl::isa::TargetIsa;

use crate::shared::types::Bool::B1;
use crate::shared::types::Float::{F32, F64};
use crate::shared::types::Int::{I16, I32, I64, I8};
use crate::shared::Definitions as SharedDefinitions;

mod encodings;
mod instructions;
mod legalize;
mod recipes;
mod registers;
mod settings;

pub(crate) fn define(shared_defs: &mut SharedDefinitions) -> TargetIsa {
    let settings = settings::define(&shared_defs.settings);
    let regs = registers::define();

    let inst_group = instructions::define(
        &mut shared_defs.all_instructions,
        &shared_defs.format_registry,
        &shared_defs.imm,
    );
    legalize::define(shared_defs, &inst_group);

    // CPU modes for 32-bit and 64-bit operations.
    let mut x86_64 = CpuMode::new("I64");
    let mut x86_32 = CpuMode::new("I32");

    let expand_flags = shared_defs.transform_groups.by_name("expand_flags");
    let narrow = shared_defs.transform_groups.by_name("narrow");
    let widen = shared_defs.transform_groups.by_name("widen");
    let x86_narrow = shared_defs.transform_groups.by_name("x86_narrow");
    let x86_expand = shared_defs.transform_groups.by_name("x86_expand");

    x86_32.legalize_monomorphic(expand_flags);
    x86_32.legalize_default(narrow);
    x86_32.legalize_type(B1, expand_flags);
    x86_32.legalize_type(I8, widen);
    x86_32.legalize_type(I16, widen);
    x86_32.legalize_type(I32, x86_expand);
    x86_32.legalize_type(F32, x86_expand);
    x86_32.legalize_type(F64, x86_expand);

    x86_64.legalize_monomorphic(expand_flags);
    x86_64.legalize_default(x86_narrow);
    x86_64.legalize_type(B1, expand_flags);
    x86_64.legalize_type(I8, widen);
    x86_64.legalize_type(I16, widen);
    x86_64.legalize_type(I32, x86_expand);
    x86_64.legalize_type(I64, x86_expand);
    x86_64.legalize_type(F32, x86_expand);
    x86_64.legalize_type(F64, x86_expand);

    let recipes = recipes::define(shared_defs, &settings, &regs);

    let encodings = encodings::define(shared_defs, &settings, &inst_group, &recipes);
    x86_32.set_encodings(encodings.enc32);
    x86_64.set_encodings(encodings.enc64);
    let encodings_predicates = encodings.inst_pred_reg.extract();

    let recipes = encodings.recipes;

    let cpu_modes = vec![x86_64, x86_32];

    TargetIsa::new(
        "x86",
        inst_group,
        settings,
        regs,
        recipes,
        cpu_modes,
        encodings_predicates,
    )
}
