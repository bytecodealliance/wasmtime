use crate::cdsl::isa::TargetIsa;
use crate::cdsl::settings::SettingGroupBuilder;

pub(crate) fn define() -> TargetIsa {
    let settings = SettingGroupBuilder::new("arm32");

    // ARM32-specific settings can be added here in the future.
    // For now, we start with an empty settings group.

    TargetIsa::new("arm32", settings.build())
}
