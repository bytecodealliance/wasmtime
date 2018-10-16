use cdsl::isa::TargetIsa;
use cdsl::regs::{RegBank, RegClass};
use cranelift_entity::EntityRef;
use error;
use srcgen::Formatter;

fn gen_regbank(fmt: &mut Formatter, reg_bank: &RegBank) {
    let names = if reg_bank.names.len() > 0 {
        format!(r#""{}""#, reg_bank.names.join(r#"", ""#))
    } else {
        "".to_string()
    };
    fmt.line("RegBank {");
    fmt.indent(|fmt| {
        fmt.line(&format!(r#"name: "{}","#, reg_bank.name));
        fmt.line(&format!("first_unit: {},", reg_bank.first_unit));
        fmt.line(&format!("units: {},", reg_bank.units));
        fmt.line(&format!("names: &[{}],", names));
        fmt.line(&format!(r#"prefix: "{}","#, reg_bank.prefix));
        fmt.line(&format!("first_toprc: {},", reg_bank.toprcs[0].index()));
        fmt.line(&format!("num_toprcs: {},", reg_bank.toprcs.len()));
        fmt.line(&format!(
            "pressure_tracking: {},",
            if reg_bank.pressure_tracking {
                "true"
            } else {
                "false"
            }
        ));
    });
    fmt.line("},");
}

fn gen_regclass(isa: &TargetIsa, reg_class: &RegClass, fmt: &mut Formatter) {
    let reg_bank = isa.reg_banks.get(reg_class.bank).unwrap();

    let mask: Vec<String> = reg_class
        .mask(reg_bank.first_unit)
        .iter()
        .map(|x| format!("0x{:08x}", x))
        .collect();
    let mask = mask.join(", ");

    fmt.line(&format!(
        "pub static {}_DATA: RegClassData = RegClassData {{",
        reg_class.name
    ));
    fmt.indent(|fmt| {
        fmt.line(&format!(r#"name: "{}","#, reg_class.name));
        fmt.line(&format!("index: {},", reg_class.index.index()));
        fmt.line(&format!("width: {},", reg_class.width));
        fmt.line(&format!("bank: {},", reg_class.bank.index()));
        fmt.line(&format!("toprc: {},", reg_class.toprc.index()));
        fmt.line(&format!(
            "first: {},",
            reg_bank.first_unit + reg_class.start
        ));
        fmt.line(&format!("subclasses: {:#x},", reg_class.subclass_mask()));
        fmt.line(&format!("mask: [{}],", mask));
        fmt.line("info: &INFO,");
    });
    fmt.line("};");
    fmt.line("#[allow(dead_code)]");
    fmt.line(&format!(
        "pub static {}: RegClass = &{}_DATA;",
        reg_class.name, reg_class.name
    ));
}

fn gen_regbank_units(reg_bank: &RegBank, fmt: &mut Formatter) {
    for unit in 0..reg_bank.units {
        let v = unit + reg_bank.first_unit;
        if (unit as usize) < reg_bank.names.len() {
            fmt.line(&format!("{} = {},", reg_bank.names[unit as usize], v));
            continue;
        }
        fmt.line(&format!("{}{} = {},", reg_bank.prefix, unit, v));
    }
}

fn gen_isa(isa: &TargetIsa, fmt: &mut Formatter) -> Result<(), error::Error> {
    // Emit RegInfo.
    fmt.line("pub static INFO: RegInfo = RegInfo {");

    fmt.indent(|fmt| {
        fmt.line("banks: &[");
        // Bank descriptors.
        fmt.indent(|fmt| {
            for reg_bank in isa.reg_banks.values() {
                gen_regbank(fmt, &reg_bank);
            }
        });
        fmt.line("],");
        // References to register classes.
        fmt.line("classes: &[");
        fmt.indent(|fmt| {
            for reg_class in isa.reg_classes.values() {
                fmt.line(&format!("&{}_DATA,", reg_class.name));
            }
        });
        fmt.line("],");
    });
    fmt.line("};");

    // Register class descriptors.
    for rc in isa.reg_classes.values() {
        gen_regclass(&isa, rc, fmt);
    }

    // Emit constants for all the register units.
    fmt.line("#[allow(dead_code, non_camel_case_types)]");
    fmt.line("#[derive(Clone, Copy)]");
    fmt.line("pub enum RU {");
    fmt.indent(|fmt| {
        for reg_bank in isa.reg_banks.values() {
            gen_regbank_units(reg_bank, fmt);
        }
    });
    fmt.line("}");

    // Emit Into conversion for the RU class.
    fmt.line("impl Into<RegUnit> for RU {");
    fmt.indent(|fmt| {
        fmt.line("fn into(self) -> RegUnit {");
        fmt.indent(|fmt| {
            fmt.line("self as RegUnit");
        });
        fmt.line("}")
    });
    fmt.line("}");

    Ok(())
}

pub fn generate(isa: TargetIsa, base_filename: &str, out_dir: &str) -> Result<(), error::Error> {
    let mut fmt = Formatter::new();
    gen_isa(&isa, &mut fmt)?;
    fmt.update_file(&format!("{}-{}.rs", base_filename, isa.name), out_dir)?;
    Ok(())
}
