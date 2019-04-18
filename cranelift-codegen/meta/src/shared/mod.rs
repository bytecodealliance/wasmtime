//! Shared definitions for the Cranelift intermediate language.

pub mod entities;
pub mod formats;
pub mod immediates;
pub mod instructions;
pub mod legalize;
pub mod settings;
pub mod types;

use crate::cdsl::formats::FormatRegistry;
use crate::cdsl::inst::InstructionGroup;
use crate::cdsl::operands::OperandKind;
use crate::cdsl::settings::SettingGroup;
use crate::cdsl::xform::TransformGroups;

pub struct Definitions {
    pub settings: SettingGroup,
    pub instructions: InstructionGroup,
    pub operand_kinds: OperandKinds,
    pub format_registry: FormatRegistry,
    pub transform_groups: TransformGroups,
}

pub struct OperandKinds(Vec<OperandKind>);

impl OperandKinds {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn by_name(&self, name: &'static str) -> &OperandKind {
        self.0
            .iter()
            .find(|op| op.name == name)
            .expect(&format!("unknown Operand name: {}", name))
    }

    pub fn push(&mut self, operand_kind: OperandKind) {
        assert!(
            self.0
                .iter()
                .find(|existing| existing.name == operand_kind.name)
                .is_none(),
            "trying to insert operand kind '{}' for the second time",
            operand_kind.name
        );
        self.0.push(operand_kind);
    }
}

pub fn define() -> Definitions {
    let immediates = OperandKinds(immediates::define());
    let entities = OperandKinds(entities::define());
    let format_registry = formats::define(&immediates, &entities);
    let instructions = instructions::define(&format_registry, &immediates, &entities);
    let transform_groups = legalize::define(&instructions, &immediates);

    Definitions {
        settings: settings::define(),
        instructions,
        operand_kinds: immediates,
        format_registry,
        transform_groups,
    }
}
