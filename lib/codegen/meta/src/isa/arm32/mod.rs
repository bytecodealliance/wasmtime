use cdsl::regs::{RegBankBuilder, RegClassBuilder};
use isa;

pub fn define() -> isa::TargetIsa {
    let mut isa = isa::TargetIsa::new("arm32");

    let builder = RegBankBuilder::new("FloatRegs", "s")
        .units(64)
        .track_pressure(true);
    let float_regs = isa.add_reg_bank(builder);

    let builder = RegBankBuilder::new("IntRegs", "r")
        .units(16)
        .track_pressure(true);
    let int_regs = isa.add_reg_bank(builder);

    let builder = RegBankBuilder::new("FlagRegs", "")
        .units(1)
        .names(vec!["nzcv"])
        .track_pressure(false);
    let flag_reg = isa.add_reg_bank(builder);

    let builder = RegClassBuilder::new_toplevel(&mut isa, "S", float_regs).count(32);
    isa.add_reg_class(builder);

    let builder = RegClassBuilder::new_toplevel(&mut isa, "D", float_regs).width(2);
    isa.add_reg_class(builder);

    let builder = RegClassBuilder::new_toplevel(&mut isa, "Q", float_regs).width(4);
    isa.add_reg_class(builder);

    let builder = RegClassBuilder::new_toplevel(&mut isa, "GPR", int_regs);
    isa.add_reg_class(builder);

    let builder = RegClassBuilder::new_toplevel(&mut isa, "FLAG", flag_reg);
    isa.add_reg_class(builder);

    isa
}
