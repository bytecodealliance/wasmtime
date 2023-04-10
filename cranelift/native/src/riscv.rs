use cranelift_codegen::settings::Configurable;

pub fn hwcap_detect(isa_builder: &mut dyn Configurable) -> Result<(), &'static str> {
    let v = unsafe { libc::getauxval(libc::AT_HWCAP) };

    const HWCAP_RISCV_EXT_A: libc::c_ulong = 1 << (b'a' - b'a');
    const HWCAP_RISCV_EXT_C: libc::c_ulong = 1 << (b'c' - b'a');
    const HWCAP_RISCV_EXT_D: libc::c_ulong = 1 << (b'd' - b'a');
    const HWCAP_RISCV_EXT_F: libc::c_ulong = 1 << (b'f' - b'a');
    const HWCAP_RISCV_EXT_M: libc::c_ulong = 1 << (b'm' - b'a');
    const HWCAP_RISCV_EXT_V: libc::c_ulong = 1 << (b'v' - b'a');

    if (v & HWCAP_RISCV_EXT_A) != 0 {
        isa_builder.enable("has_a").unwrap();
    }

    if (v & HWCAP_RISCV_EXT_C) != 0 {
        isa_builder.enable("has_c").unwrap();
    }

    if (v & HWCAP_RISCV_EXT_D) != 0 {
        isa_builder.enable("has_d").unwrap();
    }

    if (v & HWCAP_RISCV_EXT_F) != 0 {
        isa_builder.enable("has_f").unwrap();

        // TODO: There doesn't seem to be a bit associated with this extension
        // rust enables it with the `f` extension:
        // https://github.com/rust-lang/stdarch/blob/790411f93c4b5eada3c23abb4c9a063fb0b24d99/crates/std_detect/src/detect/os/linux/riscv.rs#L43
        isa_builder.enable("has_zicsr").unwrap();
    }

    if (v & HWCAP_RISCV_EXT_M) != 0 {
        isa_builder.enable("has_m").unwrap();
    }

    if (v & HWCAP_RISCV_EXT_V) != 0 {
        isa_builder.enable("has_v").unwrap();
    }

    // In general extensions that are longer than one letter
    // won't have a bit associated with them. The Linux kernel
    // is currently working on a new way to query the extensions.
    Ok(())
}
