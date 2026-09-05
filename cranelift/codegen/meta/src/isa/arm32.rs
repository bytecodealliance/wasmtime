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

    settings.add_bool(
        "has_idiv",
        "Has hardware integer divide (the `sdiv`/`udiv` instructions, present \
         on ARMv7-R/M and ARMv7VE); when disabled, division must go through a \
         runtime library call.",
        "",
        false,
    );

    TargetIsa::new("arm32", settings.build())
}
