use crate::cdsl::isa::TargetIsa;
use crate::cdsl::settings::SettingGroupBuilder;

pub (crate) fn define() -> TargetIsa {
    let mut settings: SettingGroupBuilder = SettingGroupBuilder::new("wasm32");

    
    TargetIsa::new("wasm32", settings.build())
}