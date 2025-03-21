//! Generate the Cranelift-specific integration of the x64 assembler.

use cranelift_assembler_x64_meta::dsl::{Format, Inst, Mutability, Operand, OperandKind};
use cranelift_srcgen::{fmtln, Formatter};

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
            let reg = match rm.bits() {
                128 => "Xmm",
                _ => "Gpr",
            };
            let aligned = if op.align { "Aligned" } else { "" };
            format!("&{reg}Mem{aligned}")
        }
        OperandKind::Reg(r) => match r.bits() {
            128 => "Xmm".to_string(),
            _ => "Gpr".to_string(),
        },
        OperandKind::FixedReg(_) => "Gpr".to_string(),
    }
}

/// Returns the conversion function, if any, when converting the ISLE type for
/// this parameter to the assembler type for this parameter. Effectively
/// converts `self.rust_param_raw()` to the assembler type.
pub fn rust_convert_isle_to_assembler(op: &Operand) -> Option<&'static str> {
    match op.location.kind() {
        OperandKind::Reg(r) => Some(match (r.bits(), op.mutability) {
            (128, Mutability::Read) => "cranelift_assembler_x64::Xmm::new",
            (128, Mutability::ReadWrite) => "self.convert_xmm_to_assembler_read_write_xmm",
            (_, Mutability::Read) => "cranelift_assembler_x64::Gpr::new",
            (_, Mutability::ReadWrite) => "self.convert_gpr_to_assembler_read_write_gpr",
        }),
        OperandKind::RegMem(r) => Some(match (r.bits(), op.mutability) {
            (128, Mutability::Read) => "self.convert_xmm_mem_to_assembler_read_xmm_mem",
            (128, Mutability::ReadWrite) => "self.convert_xmm_mem_to_assembler_read_write_xmm_mem",
            (_, Mutability::Read) => "self.convert_gpr_mem_to_assembler_read_gpr_mem",
            (_, Mutability::ReadWrite) => "self.convert_gpr_mem_to_assembler_read_write_gpr_mem",
        }),
        OperandKind::Imm(loc) => match (op.extension.is_sign_extended(), loc.bits()) {
            (true, 8) => Some("cranelift_assembler_x64::Simm8::new"),
            (true, 16) => Some("cranelift_assembler_x64::Simm16::new"),
            (true, 32) => Some("cranelift_assembler_x64::Simm32::new"),
            (false, 8) => Some("cranelift_assembler_x64::Imm8::new"),
            (false, 16) => Some("cranelift_assembler_x64::Imm16::new"),
            (false, 32) => Some("cranelift_assembler_x64::Imm32::new"),
            _ => None,
        },
        OperandKind::FixedReg(_) => None,
    }
}

/// `fn x64_<inst>(&mut self, <params>) -> Inst<R> { ... }`
///
/// # Panics
///
/// This function panics if the instruction has no operands.
pub fn generate_macro_inst_fn(f: &mut Formatter, inst: &Inst) {
    let struct_name = inst.name();
    let params = inst
        .format
        .operands
        .iter()
        .filter(|o| o.mutability.is_read())
        // FIXME(#10238) don't filter out fixed regs here
        .filter(|o| !matches!(o.location.kind(), OperandKind::FixedReg(_)))
        .collect::<Vec<_>>();
    let results = inst
        .format
        .operands
        .iter()
        .filter(|o| o.mutability.is_write())
        .collect::<Vec<_>>();
    let rust_params = params
        .iter()
        .map(|o| format!("{}: {}", o.location, rust_param_raw(o)))
        .collect::<Vec<_>>()
        .join(", ");
    f.add_block(
        &format!("fn x64_{struct_name}_raw(&mut self, {rust_params}) -> AssemblerOutputs"),
        |f| {
            for o in params.iter() {
                let l = o.location;
                match rust_convert_isle_to_assembler(o) {
                    Some(cvt) => fmtln!(f, "let {l} = {cvt}({l});"),
                    None => fmtln!(f, "let {l} = {l}.clone();"),
                }
            }
            let args = params
                .iter()
                .map(|o| format!("{}.clone()", o.location))
                .collect::<Vec<_>>();
            let args = args.join(", ");
            fmtln!(
                f,
                "let inst = cranelift_assembler_x64::inst::{struct_name}::new({args}).into();"
            );
            if let Some(OperandKind::FixedReg(_)) = results.first().map(|o| o.location.kind()) {
                fmtln!(f, "#[allow(unused_variables, reason = \"FIXME(#10238): fixed register instructions have TODOs\")]");
            }
            fmtln!(f, "let inst = MInst::External {{ inst }};");

            use cranelift_assembler_x64_meta::dsl::Mutability::*;
            match results.as_slice() {
                [] => fmtln!(f, "SideEffectNoResult::Inst(inst)"),
                [one] => match one.mutability {
                    Read => unreachable!(),
                    ReadWrite => match one.location.kind() {
                        OperandKind::Imm(_) => unreachable!(),
                        // FIXME(#10238)
                        OperandKind::FixedReg(_) => fmtln!(f, "todo!()"),
                        // One read/write register output? Output the instruction
                        // and that register.
                        OperandKind::Reg(r) => match r.bits() {
                            128 => {
                                fmtln!(
                                    f,
                                    "let xmm = {}.as_ref().write.to_reg();",
                                    results[0].location
                                );
                                fmtln!(f, "AssemblerOutputs::RetXmm {{ inst, xmm }}")
                            }
                            _ => {
                                fmtln!(
                                    f,
                                    "let gpr = {}.as_ref().write.to_reg();",
                                    results[0].location
                                );
                                fmtln!(f, "AssemblerOutputs::RetGpr {{ inst, gpr }}")
                            }
                        },
                        // One read/write regmem output? We need to output
                        // everything and it'll internally disambiguate which was
                        // emitted (e.g. the mem variant or the register variant).
                        OperandKind::RegMem(_) => {
                            assert_eq!(results.len(), 1);
                            let l = results[0].location;
                            f.add_block(&format!("match {l}"), |f| match l.bits() {
                                128 => {
                                    f.add_block("asm::XmmMem::Xmm(reg) => ", |f| {
                                        fmtln!(f, "let xmm = reg.write.to_reg();");
                                        fmtln!(f, "AssemblerOutputs::RetXmm {{ inst, xmm }} ");
                                    });
                                    f.add_block("asm::XmmMem::Mem(_) => ", |f| {
                                        fmtln!(f, "AssemblerOutputs::SideEffect {{ inst }} ");
                                    });
                                }
                                _ => {
                                    f.add_block("asm::GprMem::Gpr(reg) => ", |f| {
                                        fmtln!(f, "let gpr = reg.write.to_reg();");
                                        fmtln!(f, "AssemblerOutputs::RetGpr {{ inst, gpr }} ")
                                    });
                                    f.add_block("asm::GprMem::Mem(_) => ", |f| {
                                        fmtln!(f, "AssemblerOutputs::SideEffect {{ inst }} ");
                                    });
                                }
                            });
                        }
                    },
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
        OperandKind::Reg(r) => match r.bits() {
            128 => "Xmm".to_string(),
            _ => "Gpr".to_string(),
        },
        OperandKind::FixedReg(_) => "Gpr".to_string(),
        OperandKind::RegMem(rm) => {
            let reg = match rm.bits() {
                128 => "Xmm",
                _ => "Gpr",
            };
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

    /// This constructor produces a `Gpr` value, meaning that it will write the
    /// result to a `Gpr`.
    RetGpr,

    /// This constructor produces an `Xmm` value, meaning that it will write the
    /// result to an `Xmm`.
    RetXmm,
}

impl IsleConstructor {
    /// Returns the result type, in ISLE, that this constructor generates.
    pub fn result_ty(&self) -> &'static str {
        match self {
            IsleConstructor::RetMemorySideEffect => "SideEffectNoResult",
            IsleConstructor::RetGpr => "Gpr",
            IsleConstructor::RetXmm => "Xmm",
        }
    }

    /// Returns the constructor used to convert an `AssemblerOutput` into the
    /// type returned by [`Self::result_ty`].
    pub fn conversion_constructor(&self) -> &'static str {
        match self {
            IsleConstructor::RetMemorySideEffect => "defer_side_effect",
            IsleConstructor::RetGpr => "emit_ret_gpr",
            IsleConstructor::RetXmm => "emit_ret_xmm",
        }
    }

    /// Returns the suffix used in the ISLE constructor name.
    pub fn suffix(&self) -> &'static str {
        match self {
            IsleConstructor::RetMemorySideEffect => "_mem",
            IsleConstructor::RetGpr => "",
            IsleConstructor::RetXmm => "",
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
            [] => unimplemented!("if you truly need this (and not a `SideEffect*`), add a `NoReturn` variant to `AssemblerOutputs`"),
            [one] => match one.mutability {
                Read => unreachable!(),
                ReadWrite => match one.location.kind() {
                    Imm(_) => unreachable!(),
                    FixedReg(_) => vec![IsleConstructor::RetGpr],
                    // One read/write register output? Output the instruction
                    // and that register.
                    Reg(r) => match r.bits() {
                        128 => vec![IsleConstructor::RetXmm],
                        _ => vec![IsleConstructor::RetGpr],
                    },
                    // One read/write reg-mem output? We need constructors for
                    // both variants.
                    RegMem(rm) => match rm.bits() {
                        128 => vec![IsleConstructor::RetXmm, IsleConstructor::RetMemorySideEffect],
                        _ => vec![IsleConstructor::RetGpr, IsleConstructor::RetMemorySideEffect],
                    },
                }
            },
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
        // FIXME(#10238) don't filter out fixed regs here
        .filter(|o| !matches!(o.location.kind(), OperandKind::FixedReg(_)))
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
    fmtln!(f, "    ;; TODO: eventually add more variants for");
    fmtln!(f, "    ;; multi-return, XMM, etc.; see");
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
