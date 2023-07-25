use crate::cdsl::isa::TargetIsa;
use crate::cdsl::settings::SettingGroupBuilder;

macro_rules! define_zvl_ext {
    (DEF: $settings:expr, $size:expr) => {{
        let name = concat!("has_zvl", $size, "b");
        let desc = concat!("has extension Zvl", $size, "b?");
        let comment = concat!(
            "Zvl",
            $size,
            "b: Vector register has a minimum of ",
            $size,
            " bits"
        );
        $settings.add_bool(&name, &desc, &comment, false)
    }};
    ($settings:expr, $size:expr $(, $implies:expr)*) => {{
        let has_feature = define_zvl_ext!(DEF: $settings, $size);

        let name = concat!("zvl", $size, "b");
        let desc = concat!("Has a vector register size of at least ", $size, " bits");

        let preset = $settings.add_preset(&name, &desc, preset!(has_feature $( && $implies )*));
        (has_feature, preset)
    }};
}

pub(crate) fn define() -> TargetIsa {
    let mut setting = SettingGroupBuilder::new("riscv64");

    let _has_m = setting.add_bool("has_m", "has extension M?", "", false);
    let _has_a = setting.add_bool("has_a", "has extension A?", "", false);
    let _has_f = setting.add_bool("has_f", "has extension F?", "", false);
    let _has_d = setting.add_bool("has_d", "has extension D?", "", false);
    let _has_v = setting.add_bool("has_v", "has extension V?", "", false);
    let _has_c = setting.add_bool("has_c", "has extension C?", "", false);
    let _has_zbkb = setting.add_bool(
        "has_zbkb",
        "has extension zbkb?",
        "Zbkb: Bit-manipulation for Cryptography",
        false,
    );
    let _has_zba = setting.add_bool(
        "has_zba",
        "has extension zba?",
        "Zba: Address Generation",
        false,
    );
    let _has_zbb = setting.add_bool(
        "has_zbb",
        "has extension zbb?",
        "Zbb: Basic bit-manipulation",
        false,
    );
    let _has_zbc = setting.add_bool(
        "has_zbc",
        "has extension zbc?",
        "Zbc: Carry-less multiplication",
        false,
    );
    let _has_zbs = setting.add_bool(
        "has_zbs",
        "has extension zbs?",
        "Zbs: Single-bit instructions",
        false,
    );

    let _has_zicsr = setting.add_bool(
        "has_zicsr",
        "has extension zicsr?",
        "Zicsr: Control and Status Register (CSR) Instructions",
        false,
    );
    let _has_zifencei = setting.add_bool(
        "has_zifencei",
        "has extension zifencei?",
        "Zifencei: Instruction-Fetch Fence",
        false,
    );

    // Zvl*: Minimum Vector Length Standard Extensions
    // These extension specifiy the minimum number of bits in a vector register.
    // Since it is a minimum, Zvl64b implies Zvl32b, Zvl128b implies Zvl64b, etc.
    // The V extension supports a maximum of 64K bits in a single register.
    //
    // See: https://github.com/riscv/riscv-v-spec/blob/master/v-spec.adoc#181-zvl-minimum-vector-length-standard-extensions
    let (_, zvl32b) = define_zvl_ext!(setting, 32);
    let (_, zvl64b) = define_zvl_ext!(setting, 64, zvl32b);
    let (_, zvl128b) = define_zvl_ext!(setting, 128, zvl64b);
    let (_, zvl256b) = define_zvl_ext!(setting, 256, zvl128b);
    let (_, zvl512b) = define_zvl_ext!(setting, 512, zvl256b);
    let (_, zvl1024b) = define_zvl_ext!(setting, 1024, zvl512b);
    let (_, zvl2048b) = define_zvl_ext!(setting, 2048, zvl1024b);
    let (_, zvl4096b) = define_zvl_ext!(setting, 4096, zvl2048b);
    let (_, zvl8192b) = define_zvl_ext!(setting, 8192, zvl4096b);
    let (_, zvl16384b) = define_zvl_ext!(setting, 16384, zvl8192b);
    let (_, zvl32768b) = define_zvl_ext!(setting, 32768, zvl16384b);
    let (_, _zvl65536b) = define_zvl_ext!(setting, 65536, zvl32768b);

    TargetIsa::new("riscv64", setting.build())
}
