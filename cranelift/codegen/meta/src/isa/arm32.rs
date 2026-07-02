use crate::cdsl::isa::TargetIsa;
use crate::cdsl::settings::SettingGroupBuilder;

pub(crate) fn define() -> TargetIsa {
    let mut settings = SettingGroupBuilder::new("arm32");

    settings.add_bool(
        "has_neon",
        "Has Advanced SIMD (NEON) support; does not have an effect on code \
         generation by itself yet, reserved for future use.",
        "",
        false,
    );

    TargetIsa::new("arm32", settings.build())
}
