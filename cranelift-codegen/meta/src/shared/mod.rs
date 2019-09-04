//! Shared definitions for the Cranelift intermediate language.

mod entities;
pub mod formats;
pub mod immediates;
pub mod instructions;
pub mod legalize;
pub mod settings;
pub mod types;

use crate::cdsl::formats::FormatRegistry;
use crate::cdsl::instructions::{AllInstructions, InstructionGroup};
use crate::cdsl::settings::SettingGroup;
use crate::cdsl::xform::TransformGroups;

use crate::shared::entities::EntityRefs;
use crate::shared::immediates::Immediates;

pub(crate) struct Definitions {
    pub settings: SettingGroup,
    pub all_instructions: AllInstructions,
    pub instructions: InstructionGroup,
    pub imm: Immediates,
    pub format_registry: FormatRegistry,
    pub transform_groups: TransformGroups,
}

pub(crate) fn define() -> Definitions {
    let mut all_instructions = AllInstructions::new();

    let immediates = Immediates::new();
    let entities = EntityRefs::new();;
    let format_registry = formats::define(&immediates, &entities);
    let instructions = instructions::define(
        &mut all_instructions,
        &format_registry,
        &immediates,
        &entities,
    );
    let transform_groups = legalize::define(&instructions, &immediates);

    Definitions {
        settings: settings::define(),
        all_instructions,
        instructions,
        imm: immediates,
        format_registry,
        transform_groups,
    }
}
