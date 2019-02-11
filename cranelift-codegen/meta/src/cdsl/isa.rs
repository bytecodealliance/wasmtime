use super::regs::IsaRegs;
use super::settings::SettingGroup;

pub struct TargetIsa {
    pub name: &'static str,
    pub settings: SettingGroup,
    pub regs: IsaRegs,
}

impl TargetIsa {
    pub fn new(name: &'static str, settings: SettingGroup, regs: IsaRegs) -> Self {
        Self {
            name,
            settings,
            regs,
        }
    }
}
