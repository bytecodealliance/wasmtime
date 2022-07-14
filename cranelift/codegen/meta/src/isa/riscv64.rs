use crate::cdsl::isa::TargetIsa;
use crate::cdsl::settings::{SettingGroup, SettingGroupBuilder};

use crate::shared::Definitions as SharedDefinitions;

fn define_settings(_shared: &SettingGroup) -> SettingGroup {
    let mut setting = SettingGroupBuilder::new("riscv64");

    setting.add_bool("has_m", "has extension M?", "", false);
    setting.add_bool("has_a", "has extension A?", "", false);
    setting.add_bool("has_f", "has extension F?", "", false);
    setting.add_bool("has_d", "has extension D?", "", false);

    setting.add_bool("has_v", "has extension V?", "", false);

    setting.add_bool("has_b", "has extension B?", "", false);

    setting.add_bool("has_zbkb", "has extension zbkb?", "", false);
    setting.build()
}

pub(crate) fn define(shared_defs: &mut SharedDefinitions) -> TargetIsa {
    let settings = define_settings(&shared_defs.settings);
    TargetIsa::new("riscv64", settings)
}
