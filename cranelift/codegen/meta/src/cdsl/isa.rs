use crate::cdsl::inst::InstructionGroup;
use crate::cdsl::regs::IsaRegs;
use crate::cdsl::settings::SettingGroup;

pub struct TargetIsa {
    pub name: &'static str,
    pub instructions: InstructionGroup,
    pub settings: SettingGroup,
    pub regs: IsaRegs,
}

impl TargetIsa {
    pub fn new(
        name: &'static str,
        instructions: InstructionGroup,
        settings: SettingGroup,
        regs: IsaRegs,
    ) -> Self {
        Self {
            name,
            instructions,
            settings,
            regs,
        }
    }
}
