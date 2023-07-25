use crate::cdsl::isa::TargetIsa;
use crate::cdsl::settings::SettingGroupBuilder;

pub(crate) fn define() -> TargetIsa {
    let mut settings = SettingGroupBuilder::new("s390x");

    // The baseline architecture for cranelift is z14 (arch12),
    // so we list only facilities of later processors here.

    // z15 (arch13) facilities
    let has_mie2 = settings.add_bool(
        "has_mie2",
        "Has Miscellaneous-Instruction-Extensions Facility 2 support.",
        "",
        false,
    );
    let has_vxrs_ext2 = settings.add_bool(
        "has_vxrs_ext2",
        "Has Vector-Enhancements Facility 2 support.",
        "",
        false,
    );

    // Architecture level presets
    settings.add_preset(
        "arch13",
        "Thirteenth Edition of the z/Architecture.",
        preset!(has_mie2 && has_vxrs_ext2),
    );

    // Processor presets
    settings.add_preset(
        "z15",
        "IBM z15 processor.",
        preset!(has_mie2 && has_vxrs_ext2),
    );

    TargetIsa::new("s390x", settings.build())
}
