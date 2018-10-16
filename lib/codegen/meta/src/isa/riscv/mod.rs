use cdsl::regs::{RegBankBuilder, RegClassBuilder};
use isa;

pub fn define() -> isa::TargetIsa {
    let mut isa = isa::TargetIsa::new("riscv");

    let builder = RegBankBuilder::new("IntRegs", "x")
        .units(32)
        .track_pressure(true);
    let int_regs = isa.add_reg_bank(builder);

    let builder = RegBankBuilder::new("FloatRegs", "f")
        .units(32)
        .track_pressure(true);
    let float_regs = isa.add_reg_bank(builder);

    let builder = RegClassBuilder::new_toplevel(&mut isa, "GPR", int_regs);
    isa.add_reg_class(builder);

    let builder = RegClassBuilder::new_toplevel(&mut isa, "FPR", float_regs);
    isa.add_reg_class(builder);

    isa
}
