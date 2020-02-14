//! Generate the ISA-specific registers.
use crate::cdsl::isa::TargetIsa;
use crate::cdsl::regs::{RegBank, RegClass};
use crate::error;
use crate::srcgen::Formatter;
use cranelift_entity::EntityRef;

fn gen_regbank(fmt: &mut Formatter, reg_bank: &RegBank) {
    let names = if !reg_bank.names.is_empty() {
        format!(r#""{}""#, reg_bank.names.join(r#"", ""#))
    } else {
        "".to_string()
    };
    fmtln!(fmt, "RegBank {");
    fmt.indent(|fmt| {
        fmtln!(fmt, r#"name: "{}","#, reg_bank.name);
        fmtln!(fmt, "first_unit: {},", reg_bank.first_unit);
        fmtln!(fmt, "units: {},", reg_bank.units);
        fmtln!(fmt, "names: &[{}],", names);
        fmtln!(fmt, r#"prefix: "{}","#, reg_bank.prefix);
        fmtln!(fmt, "first_toprc: {},", reg_bank.toprcs[0].index());
        fmtln!(fmt, "num_toprcs: {},", reg_bank.toprcs.len());
        fmtln!(
            fmt,
            "pressure_tracking: {},",
            if reg_bank.pressure_tracking {
                "true"
            } else {
                "false"
            }
        );
    });
    fmtln!(fmt, "},");
}

fn gen_regclass(isa: &TargetIsa, reg_class: &RegClass, fmt: &mut Formatter) {
    let reg_bank = isa.regs.banks.get(reg_class.bank).unwrap();

    let mask: Vec<String> = reg_class
        .mask(reg_bank.first_unit)
        .iter()
        .map(|x| format!("0x{:08x}", x))
        .collect();
    let mask = mask.join(", ");

    fmtln!(
        fmt,
        "pub static {}_DATA: RegClassData = RegClassData {{",
        reg_class.name
    );
    fmt.indent(|fmt| {
        fmtln!(fmt, r#"name: "{}","#, reg_class.name);
        fmtln!(fmt, "index: {},", reg_class.index.index());
        fmtln!(fmt, "width: {},", reg_class.width);
        fmtln!(fmt, "bank: {},", reg_class.bank.index());
        fmtln!(fmt, "toprc: {},", reg_class.toprc.index());
        fmtln!(fmt, "first: {},", reg_bank.first_unit + reg_class.start);
        fmtln!(fmt, "subclasses: {:#x},", reg_class.subclass_mask());
        fmtln!(fmt, "mask: [{}],", mask);
        fmtln!(
            fmt,
            "pinned_reg: {:?},",
            reg_bank
                .pinned_reg
                .map(|index| index + reg_bank.first_unit as u16 + reg_class.start as u16)
        );
        fmtln!(fmt, "info: &INFO,");
    });
    fmtln!(fmt, "};");

    fmtln!(fmt, "#[allow(dead_code)]");
    fmtln!(
        fmt,
        "pub static {}: RegClass = &{}_DATA;",
        reg_class.name,
        reg_class.name
    );
}

fn gen_regbank_units(reg_bank: &RegBank, fmt: &mut Formatter) {
    for unit in 0..reg_bank.units {
        let v = unit + reg_bank.first_unit;
        if (unit as usize) < reg_bank.names.len() {
            fmtln!(fmt, "{} = {},", reg_bank.names[unit as usize], v);
            continue;
        }
        fmtln!(fmt, "{}{} = {},", reg_bank.prefix, unit, v);
    }
}

fn gen_isa(isa: &TargetIsa, fmt: &mut Formatter) {
    // Emit RegInfo.
    fmtln!(fmt, "pub static INFO: RegInfo = RegInfo {");

    fmt.indent(|fmt| {
        fmtln!(fmt, "banks: &[");
        // Bank descriptors.
        fmt.indent(|fmt| {
            for reg_bank in isa.regs.banks.values() {
                gen_regbank(fmt, &reg_bank);
            }
        });
        fmtln!(fmt, "],");
        // References to register classes.
        fmtln!(fmt, "classes: &[");
        fmt.indent(|fmt| {
            for reg_class in isa.regs.classes.values() {
                fmtln!(fmt, "&{}_DATA,", reg_class.name);
            }
        });
        fmtln!(fmt, "],");
    });
    fmtln!(fmt, "};");

    // Register class descriptors.
    for rc in isa.regs.classes.values() {
        gen_regclass(&isa, rc, fmt);
    }

    // Emit constants for all the register units.
    fmtln!(fmt, "#[allow(dead_code, non_camel_case_types)]");
    fmtln!(fmt, "#[derive(Clone, Copy)]");
    fmtln!(fmt, "pub enum RU {");
    fmt.indent(|fmt| {
        for reg_bank in isa.regs.banks.values() {
            gen_regbank_units(reg_bank, fmt);
        }
    });
    fmtln!(fmt, "}");

    // Emit Into conversion for the RU class.
    fmtln!(fmt, "impl Into<RegUnit> for RU {");
    fmt.indent(|fmt| {
        fmtln!(fmt, "fn into(self) -> RegUnit {");
        fmt.indent(|fmt| {
            fmtln!(fmt, "self as RegUnit");
        });
        fmtln!(fmt, "}");
    });
    fmtln!(fmt, "}");
}

pub(crate) fn generate(isa: &TargetIsa, filename: &str, out_dir: &str) -> Result<(), error::Error> {
    let mut fmt = Formatter::new();
    gen_isa(&isa, &mut fmt);
    fmt.update_file(filename, out_dir)?;
    Ok(())
}
