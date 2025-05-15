//! Generate the Cranelift-specific integration of the x64 assembler.

use cranelift_assembler_x64_meta::dsl::{
    Format, Inst, Location, Mutability, Operand, OperandKind, RegClass,
};
use cranelift_srcgen::{Formatter, fmtln};

/// This factors out use of the assembler crate name.
const ASM: &str = "cranelift_assembler_x64";

/// Returns the Rust type used for the `IsleConstructorRaw` variants.
pub fn rust_param_raw(op: &Operand) -> String {
    match op.location.kind() {
        OperandKind::Imm(loc) => {
            let bits = loc.bits();
            if op.extension.is_sign_extended() {
                format!("i{bits}")
            } else {
                format!("u{bits}")
            }
        }
        OperandKind::RegMem(rm) => {
            let reg = rm.reg_class().unwrap();
            let aligned = if op.align { "Aligned" } else { "" };
            format!("&{reg}Mem{aligned}")
        }
        OperandKind::Mem(_) => {
            format!("&Amode")
        }
        OperandKind::Reg(r) | OperandKind::FixedReg(r) => r.reg_class().unwrap().to_string(),
    }
}

/// Returns the conversion function, if any, when converting the ISLE type for
/// this parameter to the assembler type for this parameter. Effectively
/// converts `self.rust_param_raw()` to the assembler type.
pub fn rust_convert_isle_to_assembler(op: &Operand) -> String {
    match op.location.kind() {
        OperandKind::Imm(loc) => {
            let bits = loc.bits();
            let ty = if op.extension.is_sign_extended() {
                "Simm"
            } else {
                "Imm"
            };
            format!("{ASM}::{ty}{bits}::new({loc})")
        }
        OperandKind::FixedReg(r) => {
            let reg = r.reg_class().unwrap().to_string().to_lowercase();
            match op.mutability {
                Mutability::Read => format!("{ASM}::Fixed({r})"),
                Mutability::Write => {
                    format!("{ASM}::Fixed(self.temp_writable_{reg}())")
                }
                Mutability::ReadWrite => {
                    format!("self.convert_{reg}_to_assembler_fixed_read_write_{reg}({r})")
                }
            }
        }
        OperandKind::Reg(r) => {
            let reg = r.reg_class().unwrap();
            let reg_lower = reg.to_string().to_lowercase();
            match op.mutability {
                Mutability::Read => {
                    format!("{ASM}::{reg}::new({r})")
                }
                Mutability::Write => {
                    format!("{ASM}::{reg}::new(self.temp_writable_{reg_lower}())")
                }
                Mutability::ReadWrite => {
                    format!("self.convert_{reg_lower}_to_assembler_read_write_{reg_lower}({r})")
                }
            }
        }
        OperandKind::RegMem(rm) => {
            let reg = rm.reg_class().unwrap().to_string().to_lowercase();
            let mut_ = op.mutability.generate_snake_case();
            let align = if op.align { "_aligned" } else { "" };
            format!("self.convert_{reg}_mem_to_assembler_{mut_}_{reg}_mem{align}({rm})")
        }
        OperandKind::Mem(mem) => format!("self.convert_amode_to_assembler_amode({mem})"),
    }
}

/// `fn x64_<inst>(&mut self, <params>) -> Inst<R> { ... }`
///
/// # Panics
///
/// This function panics if the instruction has no operands.
pub fn generate_macro_inst_fn(f: &mut Formatter, inst: &Inst) {
    let struct_name = inst.name();
    let operands = inst.format.operands.iter().cloned().collect::<Vec<_>>();
    let results = operands
        .iter()
        .filter(|o| o.mutability.is_write())
        .collect::<Vec<_>>();
    let rust_params = operands
        .iter()
        .filter(|o| o.mutability.is_read())
        .map(|o| format!("{}: {}", o.location, rust_param_raw(o)))
        .collect::<Vec<_>>()
        .join(", ");
    f.add_block(
        &format!("fn x64_{struct_name}_raw(&mut self, {rust_params}) -> AssemblerOutputs"),
        |f| {
            f.comment("Convert ISLE types to assembler types.");
            for op in operands.iter() {
                let loc = op.location;
                let cvt = rust_convert_isle_to_assembler(op);
                fmtln!(f, "let {loc} = {cvt};");
            }
            let args = operands
                .iter()
                .map(|o| format!("{}.clone()", o.location))
                .collect::<Vec<_>>();
            let args = args.join(", ");
            f.empty_line();

            f.comment("Build the instruction.");
            fmtln!(
                f,
                "let inst = {ASM}::inst::{struct_name}::new({args}).into();"
            );
            fmtln!(f, "let inst = MInst::External {{ inst }};");
            f.empty_line();

            // When an instruction writes to an operand, Cranelift expects a
            // returned value to use in other instructions: we return this
            // information in the `AssemblerOutputs` struct defined in ISLE
            // (below). The general rule here is that memory stores will create
            // a `SideEffect` whereas for write or read-write registers we will
            // return some form of `Ret*`.
            f.comment("Return a type ISLE can work with.");
            let access_reg = |op: &Operand| match op.mutability {
                Mutability::Read => unreachable!(),
                Mutability::Write => "to_reg()",
                Mutability::ReadWrite => "write.to_reg()",
            };
            let ty_var_of_reg = |loc: Location| {
                let ty = loc.reg_class().unwrap().to_string();
                let var = ty.to_lowercase();
                (ty, var)
            };
            match results.as_slice() {
                [] => fmtln!(f, "SideEffectNoResult::Inst(inst)"),
                [op] => match op.location.kind() {
                    OperandKind::Imm(_) => unreachable!(),
                    OperandKind::Reg(r) | OperandKind::FixedReg(r) => {
                        let (ty, var) = ty_var_of_reg(r);
                        fmtln!(f, "let {var} = {r}.as_ref().{};", access_reg(op));
                        fmtln!(f, "AssemblerOutputs::Ret{ty} {{ inst, {var} }}");
                    }
                    OperandKind::Mem(_) => {
                        fmtln!(f, "AssemblerOutputs::SideEffect {{ inst }}")
                    }
                    OperandKind::RegMem(rm) => {
                        let (ty, var) = ty_var_of_reg(rm);
                        f.add_block(&format!("match {rm}"), |f| {
                            f.add_block(&format!("{ASM}::{ty}Mem::{ty}(reg) => "), |f| {
                                fmtln!(f, "let {var} = reg.{};", access_reg(op));
                                fmtln!(f, "AssemblerOutputs::Ret{ty} {{ inst, {var} }} ");
                            });
                            f.add_block(&format!("{ASM}::{ty}Mem::Mem(_) => "), |f| {
                                fmtln!(f, "AssemblerOutputs::SideEffect {{ inst }} ");
                            });
                        });
                    }
                },
                // For now, we assume that if there are two results, they are
                // coming from a register-writing instruction like `mul`. The
                // `match` below can be expanded as needed.
                [op1, op2] => match (op1.location.kind(), op2.location.kind()) {
                    (OperandKind::FixedReg(loc1), OperandKind::FixedReg(loc2)) => {
                        fmtln!(f, "let one = {loc1}.as_ref().{}.to_reg();", access_reg(op1));
                        fmtln!(f, "let two = {loc2}.as_ref().{}.to_reg();", access_reg(op2));
                        fmtln!(f, "let regs = ValueRegs::two(one, two);");
                        fmtln!(f, "AssemblerOutputs::RetValueRegs {{ inst, regs }}");
                    }
                    _ => unimplemented!("unhandled results: {results:?}"),
                },
                _ => panic!("instruction has more than one result"),
            }
        },
    );
}

/// Generate the `isle_assembler_methods!` macro.
pub fn generate_rust_macro(f: &mut Formatter, insts: &[Inst]) {
    fmtln!(f, "#[doc(hidden)]");
    fmtln!(f, "macro_rules! isle_assembler_methods {{");
    f.indent(|f| {
        fmtln!(f, "() => {{");
        f.indent(|f| {
            for inst in insts {
                generate_macro_inst_fn(f, inst);
            }
        });
        fmtln!(f, "}};");
    });
    fmtln!(f, "}}");
}

/// Returns the type of this operand in ISLE as a part of the ISLE "raw"
/// constructors.
pub fn isle_param_raw(op: &Operand) -> String {
    match op.location.kind() {
        OperandKind::Imm(loc) => {
            let bits = loc.bits();
            if op.extension.is_sign_extended() {
                format!("i{bits}")
            } else {
                format!("u{bits}")
            }
        }
        OperandKind::Reg(r) | OperandKind::FixedReg(r) => r.reg_class().unwrap().to_string(),
        OperandKind::Mem(_) => {
            if op.align {
                unimplemented!("no way yet to mark an Amode as aligned")
            } else {
                "Amode".to_string()
            }
        }
        OperandKind::RegMem(rm) => {
            let reg = rm.reg_class().unwrap();
            let aligned = if op.align { "Aligned" } else { "" };
            format!("{reg}Mem{aligned}")
        }
    }
}

/// Different kinds of ISLE constructors generated for a particular instruction.
///
/// One instruction may generate a single constructor or multiple constructors.
/// For example an instruction that writes its result to a register will
/// generate only a single constructor. An instruction where the destination
/// read/write operand is `GprMem` will generate two constructors though, one
/// for memory and one for in registers.
#[derive(Copy, Clone, Debug)]
pub enum IsleConstructor {
    /// This constructor only produces a side effect, meaning that the
    /// instruction does not produce results in registers. This may produce
    /// a result in memory, however.
    RetMemorySideEffect,

    /// This constructor produces a `Gpr` value, meaning that the instruction
    /// will write its result to a single GPR register.
    RetGpr,

    /// This is similar to `RetGpr`, but for XMM registers.
    RetXmm,

    /// This "special" constructor captures multiple written-to registers (e.g.
    /// `mul`).
    RetValueRegs,
}

impl IsleConstructor {
    /// Returns the result type, in ISLE, that this constructor generates.
    pub fn result_ty(&self) -> &'static str {
        match self {
            IsleConstructor::RetMemorySideEffect => "SideEffectNoResult",
            IsleConstructor::RetGpr => "Gpr",
            IsleConstructor::RetXmm => "Xmm",
            IsleConstructor::RetValueRegs => "ValueRegs",
        }
    }

    /// Returns the constructor used to convert an `AssemblerOutput` into the
    /// type returned by [`Self::result_ty`].
    pub fn conversion_constructor(&self) -> &'static str {
        match self {
            IsleConstructor::RetMemorySideEffect => "defer_side_effect",
            IsleConstructor::RetGpr => "emit_ret_gpr",
            IsleConstructor::RetXmm => "emit_ret_xmm",
            IsleConstructor::RetValueRegs => "emit_ret_value_regs",
        }
    }

    /// Returns the suffix used in the ISLE constructor name.
    pub fn suffix(&self) -> &'static str {
        match self {
            IsleConstructor::RetMemorySideEffect => "_mem",
            IsleConstructor::RetGpr | IsleConstructor::RetXmm | IsleConstructor::RetValueRegs => "",
        }
    }
}

/// Returns the parameter type used for the `IsleConstructor` variant
/// provided.
pub fn isle_param_for_ctor(op: &Operand, ctor: IsleConstructor) -> String {
    match op.location.kind() {
        // Writable `RegMem` operands are special here: in one constructor
        // it's operating on memory so the argument is `Amode` and in the
        // other constructor it's operating on registers so the argument is
        // a `Gpr`.
        OperandKind::RegMem(_) if op.mutability.is_write() => match ctor {
            IsleConstructor::RetMemorySideEffect => "Amode".to_string(),
            IsleConstructor::RetGpr => "Gpr".to_string(),
            IsleConstructor::RetXmm => "Xmm".to_string(),
            IsleConstructor::RetValueRegs => "ValueRegs".to_string(),
        },

        // everything else is the same as the "raw" variant
        _ => isle_param_raw(op),
    }
}

/// Returns the ISLE constructors that are going to be used when generating
/// this instruction.
///
/// Note that one instruction might need multiple constructors, such as one
/// for operating on memory and one for operating on registers.
pub fn isle_constructors(format: &Format) -> Vec<IsleConstructor> {
    use Mutability::*;
    use OperandKind::*;

    let write_operands = format
        .operands
        .iter()
        .filter(|o| o.mutability.is_write())
        .collect::<Vec<_>>();
    match &write_operands[..] {
        [] => unimplemented!(
            "if you truly need this (and not a `SideEffect*`), add a `NoReturn` variant to `AssemblerOutputs`"
        ),
        [one] => match one.mutability {
            Read => unreachable!(),
            ReadWrite | Write => match one.location.kind() {
                Imm(_) => unreachable!(),
                // One read/write register output? Output the instruction
                // and that register.
                Reg(r) | FixedReg(r) => match r.reg_class().unwrap() {
                    RegClass::Xmm => vec![IsleConstructor::RetXmm],
                    RegClass::Gpr => vec![IsleConstructor::RetGpr],
                },
                // One read/write memory operand? Output a side effect.
                Mem(_) => vec![IsleConstructor::RetMemorySideEffect],
                // One read/write reg-mem output? We need constructors for
                // both variants.
                RegMem(rm) => match rm.reg_class().unwrap() {
                    RegClass::Xmm => vec![
                        IsleConstructor::RetXmm,
                        IsleConstructor::RetMemorySideEffect,
                    ],
                    RegClass::Gpr => vec![
                        IsleConstructor::RetGpr,
                        IsleConstructor::RetMemorySideEffect,
                    ],
                },
            },
        },
        [one, two] => {
            // For now, we assume that if there are two results, they are coming
            // from a register-writing instruction like `mul`. This can be
            // expanded as needed.
            assert!(matches!(one.location.kind(), FixedReg(_)));
            assert!(matches!(two.location.kind(), FixedReg(_)));
            vec![IsleConstructor::RetValueRegs]
        }
        other => panic!("unsupported number of write operands {other:?}"),
    }
}

/// Generate a "raw" constructor that simply constructs, but does not emit
/// the assembly instruction:
///
/// ```text
/// (decl x64_<inst>_raw (<params>) AssemblerOutputs)
/// (extern constructor x64_<inst>_raw x64_<inst>_raw)
/// ```
///
/// Using the "raw" constructor, we also generate "emitter" constructors
/// (see [`IsleConstructor`]). E.g., instructions that write to a register
/// will return the register:
///
/// ```text
/// (decl x64_<inst> (<params>) Gpr)
/// (rule (x64_<inst> <params>) (emit_ret_gpr (x64_<inst>_raw <params>)))
/// ```
///
/// For instructions that write to memory, we also generate an "emitter"
/// constructor with the `_mem` suffix:
///
/// ```text
/// (decl x64_<inst>_mem (<params>) SideEffectNoResult)
/// (rule (x64_<inst>_mem <params>) (defer_side_effect (x64_<inst>_raw <params>)))
/// ```
///
/// # Panics
///
/// This function panics if the instruction has no operands.
pub fn generate_isle_inst_decls(f: &mut Formatter, inst: &Inst) {
    // First declare the "raw" constructor which is implemented in Rust
    // with `generate_isle_macro` above. This is an "extern" constructor
    // with relatively raw types. This is not intended to be used by
    // general lowering rules in ISLE.
    let struct_name = inst.name();
    let raw_name = format!("x64_{struct_name}_raw");
    let params = inst
        .format
        .operands
        .iter()
        .filter(|o| o.mutability.is_read())
        .collect::<Vec<_>>();
    let raw_param_tys = params
        .iter()
        .map(|o| isle_param_raw(o))
        .collect::<Vec<_>>()
        .join(" ");
    fmtln!(f, "(decl {raw_name} ({raw_param_tys}) AssemblerOutputs)");
    fmtln!(f, "(extern constructor {raw_name} {raw_name})");

    // Next, for each "emitter" ISLE constructor being generated, synthesize
    // a pure-ISLE constructor which delegates appropriately to the `*_raw`
    // constructor above.
    //
    // The main purpose of these constructors is to have faithful type
    // signatures for the SSA nature of VCode/ISLE, effectively translating
    // x64's type system to ISLE/VCode's type system.
    for ctor in isle_constructors(&inst.format) {
        let suffix = ctor.suffix();
        let rule_name = format!("x64_{struct_name}{suffix}");
        let result_ty = ctor.result_ty();
        let param_tys = params
            .iter()
            .map(|o| isle_param_for_ctor(o, ctor))
            .collect::<Vec<_>>()
            .join(" ");
        let param_names = params
            .iter()
            .map(|o| o.location.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let convert = ctor.conversion_constructor();

        fmtln!(f, "(decl {rule_name} ({param_tys}) {result_ty})");
        fmtln!(
            f,
            "(rule ({rule_name} {param_names}) ({convert} ({raw_name} {param_names})))"
        );
    }
}

/// Generate the ISLE definitions that match the `isle_assembler_methods!` macro
/// above.
pub fn generate_isle(f: &mut Formatter, insts: &[Inst]) {
    fmtln!(f, "(type AssemblerOutputs (enum");
    fmtln!(f, "    ;; Used for instructions that have ISLE");
    fmtln!(f, "    ;; `SideEffect`s (memory stores, traps,");
    fmtln!(f, "    ;; etc.) and do not return a `Value`.");
    fmtln!(f, "    (SideEffect (inst MInst))");
    fmtln!(f, "    ;; Used for instructions that return a");
    fmtln!(f, "    ;; GPR (including `GprMem` variants with");
    fmtln!(f, "    ;; a GPR as the first argument).");
    fmtln!(f, "    (RetGpr (inst MInst) (gpr Gpr))");
    fmtln!(f, "    ;; Used for instructions that return an");
    fmtln!(f, "    ;; XMM register.");
    fmtln!(f, "    (RetXmm (inst MInst) (xmm Xmm))");
    fmtln!(f, "    ;; Used for multi-return instructions.");
    fmtln!(f, "    (RetValueRegs (inst MInst) (regs ValueRegs))");
    fmtln!(
        f,
        "    ;; https://github.com/bytecodealliance/wasmtime/pull/10276"
    );
    fmtln!(f, "))");
    f.empty_line();

    fmtln!(f, ";; Directly emit instructions that return a GPR.");
    fmtln!(f, "(decl emit_ret_gpr (AssemblerOutputs) Gpr)");
    fmtln!(f, "(rule (emit_ret_gpr (AssemblerOutputs.RetGpr inst gpr))");
    fmtln!(f, "    (let ((_ Unit (emit inst))) gpr))");
    f.empty_line();

    fmtln!(f, ";; Directly emit instructions that return an");
    fmtln!(f, ";; XMM register.");
    fmtln!(f, "(decl emit_ret_xmm (AssemblerOutputs) Xmm)");
    fmtln!(f, "(rule (emit_ret_xmm (AssemblerOutputs.RetXmm inst xmm))");
    fmtln!(f, "    (let ((_ Unit (emit inst))) xmm))");
    f.empty_line();

    fmtln!(f, ";; Directly emit instructions that return multiple");
    fmtln!(f, ";; registers (e.g. `mul`).");
    fmtln!(f, "(decl emit_ret_value_regs (AssemblerOutputs) ValueRegs)");
    fmtln!(
        f,
        "(rule (emit_ret_value_regs (AssemblerOutputs.RetValueRegs inst regs))"
    );
    fmtln!(f, "    (let ((_ Unit (emit inst))) regs))");
    f.empty_line();

    fmtln!(f, ";; Pass along the side-effecting instruction");
    fmtln!(f, ";; for later emission.");
    fmtln!(
        f,
        "(decl defer_side_effect (AssemblerOutputs) SideEffectNoResult)"
    );
    fmtln!(
        f,
        "(rule (defer_side_effect (AssemblerOutputs.SideEffect inst))"
    );
    fmtln!(f, "    (SideEffectNoResult.Inst inst))");
    f.empty_line();

    for inst in insts {
        generate_isle_inst_decls(f, inst);
        f.empty_line();
    }
}
