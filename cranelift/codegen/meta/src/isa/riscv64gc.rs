use crate::cdsl::isa::TargetIsa;
use crate::cdsl::settings::{SettingGroup, SettingGroupBuilder};

use crate::shared::Definitions as SharedDefinitions;

fn define_settings(_shared: &SettingGroup) -> SettingGroup {
    let mut setting = SettingGroupBuilder::new("riscv64gc");

    setting.add_bool("has_extension_m", "has extension M?", "", false);
    setting.add_bool("has_extension_a", "has extension A?", "", false);
    setting.add_bool("has_extension_f", "has extension F?", "", false);
    setting.add_bool("has_extension_d", "has extension D?", "", false);

    setting.add_bool("has_extension_v", "has extension V?", "", false);

    setting.add_bool("has_extension_zba", "has extension zba?", "", false);
    setting.add_bool("has_extension_zbb", "has extension zbb?", "", false);
    setting.add_bool("has_extension_zbc", "has extension zbc?", "", false);
    setting.add_bool("has_extension_zbs", "has extension zbs?", "", false);
    setting.add_bool("has_extension_zbkb", "has extension zbkb?", "", false);
    setting.build()
}

pub(crate) fn define(shared_defs: &mut SharedDefinitions) -> TargetIsa {
    let settings = define_settings(&shared_defs.settings);
    TargetIsa::new("riscv64gc", settings)
}
