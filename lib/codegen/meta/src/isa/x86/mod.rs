use cdsl::regs::{RegBankBuilder, RegClassBuilder};
use isa;

pub fn define() -> isa::TargetIsa {
    let mut isa = isa::TargetIsa::new("x86");

    let builder = RegBankBuilder::new("IntRegs", "r")
        .units(16)
        .names(vec!["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi"])
        .track_pressure(true);
    let int_regs = isa.add_reg_bank(builder);

    let builder = RegBankBuilder::new("FloatRegs", "xmm")
        .units(16)
        .track_pressure(true);
    let float_regs = isa.add_reg_bank(builder);

    let builder = RegBankBuilder::new("FlagRegs", "")
        .units(1)
        .names(vec!["rflags"])
        .track_pressure(false);
    let flag_reg = isa.add_reg_bank(builder);

    let builder = RegClassBuilder::new_toplevel(&mut isa, "GPR", int_regs);
    let gpr = isa.add_reg_class(builder);

    let builder = RegClassBuilder::new_toplevel(&mut isa, "FPR", float_regs);
    let fpr = isa.add_reg_class(builder);

    let builder = RegClassBuilder::new_toplevel(&mut isa, "FLAG", flag_reg);
    isa.add_reg_class(builder);

    let builder = RegClassBuilder::subclass_of(&mut isa, "GPR8", gpr, 0, 8);
    let gpr8 = isa.add_reg_class(builder);

    let builder = RegClassBuilder::subclass_of(&mut isa, "ABCD", gpr8, 0, 4);
    isa.add_reg_class(builder);

    let builder = RegClassBuilder::subclass_of(&mut isa, "FPR8", fpr, 0, 8);
    isa.add_reg_class(builder);

    isa
}
