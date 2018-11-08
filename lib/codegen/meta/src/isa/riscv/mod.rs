use cdsl::isa::{TargetIsa, TargetIsaBuilder};
use cdsl::regs::{RegBankBuilder, RegClassBuilder};

pub fn define() -> TargetIsa {
    let mut isa = TargetIsaBuilder::new("riscv");

    let builder = RegBankBuilder::new("IntRegs", "x")
        .units(32)
        .track_pressure(true);
    let int_regs = isa.add_reg_bank(builder);

    let builder = RegBankBuilder::new("FloatRegs", "f")
        .units(32)
        .track_pressure(true);
    let float_regs = isa.add_reg_bank(builder);

    let builder = RegClassBuilder::new_toplevel("GPR", int_regs);
    isa.add_reg_class(builder);

    let builder = RegClassBuilder::new_toplevel("FPR", float_regs);
    isa.add_reg_class(builder);

    isa.finish()
}
