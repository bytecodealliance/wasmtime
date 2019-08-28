use crate::cdsl::regs::{IsaRegs, IsaRegsBuilder, RegBankBuilder, RegClassBuilder};

pub fn define() -> IsaRegs {
    let mut regs = IsaRegsBuilder::new();

    let builder = RegBankBuilder::new("IntRegs", "r")
        .units(16)
        .names(vec!["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi"])
        .track_pressure(true)
        .pinned_reg(15);
    let int_regs = regs.add_bank(builder);

    let builder = RegBankBuilder::new("FloatRegs", "xmm")
        .units(16)
        .track_pressure(true);
    let float_regs = regs.add_bank(builder);

    let builder = RegBankBuilder::new("FlagRegs", "")
        .units(1)
        .names(vec!["rflags"])
        .track_pressure(false);
    let flag_reg = regs.add_bank(builder);

    let builder = RegClassBuilder::new_toplevel("GPR", int_regs);
    let gpr = regs.add_class(builder);

    let builder = RegClassBuilder::new_toplevel("FPR", float_regs);
    let fpr = regs.add_class(builder);

    let builder = RegClassBuilder::new_toplevel("FLAG", flag_reg);
    regs.add_class(builder);

    let builder = RegClassBuilder::subclass_of("GPR8", gpr, 0, 8);
    let gpr8 = regs.add_class(builder);

    let builder = RegClassBuilder::subclass_of("ABCD", gpr8, 0, 4);
    regs.add_class(builder);

    let builder = RegClassBuilder::subclass_of("FPR8", fpr, 0, 8);
    regs.add_class(builder);

    regs.build()
}
