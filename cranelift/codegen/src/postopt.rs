//! A post-legalization rewriting pass.

#![allow(non_snake_case)]

use crate::cursor::{Cursor, EncCursor};
use crate::ir::condcodes::{CondCode, FloatCC, IntCC};
use crate::ir::dfg::ValueDef;
use crate::ir::immediates::{Imm64, Offset32};
use crate::ir::instructions::{Opcode, ValueList};
use crate::ir::{Block, Function, Inst, InstBuilder, InstructionData, MemFlags, Type, Value};
use crate::isa::TargetIsa;
use crate::timing;

/// Information collected about a compare+branch sequence.
struct CmpBrInfo {
    /// The branch instruction.
    br_inst: Inst,
    /// The icmp, icmp_imm, or fcmp instruction.
    cmp_inst: Inst,
    /// The destination of the branch.
    destination: Block,
    /// The arguments of the branch.
    args: ValueList,
    /// The first argument to the comparison. The second is in the `kind` field.
    cmp_arg: Value,
    /// If the branch is `brz` rather than `brnz`, we need to invert the condition
    /// before the branch.
    invert_branch_cond: bool,
    /// The kind of comparison, and the second argument.
    kind: CmpBrKind,
}

enum CmpBrKind {
    Icmp { cond: IntCC, arg: Value },
    IcmpImm { cond: IntCC, imm: Imm64 },
    Fcmp { cond: FloatCC, arg: Value },
}

/// Optimize comparisons to use flags values, to avoid materializing conditions
/// in integer registers.
///
/// For example, optimize icmp/fcmp brz/brnz sequences into ifcmp/ffcmp brif/brff
/// sequences.
fn optimize_cpu_flags(
    pos: &mut EncCursor,
    inst: Inst,
    last_flags_clobber: Option<Inst>,
    isa: &dyn TargetIsa,
) {
    // Look for compare and branch patterns.
    // This code could be considerably simplified with non-lexical lifetimes.
    let info = match pos.func.dfg[inst] {
        InstructionData::Branch {
            opcode,
            destination,
            ref args,
        } => {
            let first_arg = args.first(&pos.func.dfg.value_lists).unwrap();
            let invert_branch_cond = match opcode {
                Opcode::Brz => true,
                Opcode::Brnz => false,
                _ => panic!(),
            };
            if let ValueDef::Result(cond_inst, _) = pos.func.dfg.value_def(first_arg) {
                match pos.func.dfg[cond_inst] {
                    InstructionData::IntCompare {
                        cond,
                        args: cmp_args,
                        ..
                    } => CmpBrInfo {
                        br_inst: inst,
                        cmp_inst: cond_inst,
                        destination,
                        args: args.clone(),
                        cmp_arg: cmp_args[0],
                        invert_branch_cond,
                        kind: CmpBrKind::Icmp {
                            cond,
                            arg: cmp_args[1],
                        },
                    },
                    InstructionData::IntCompareImm {
                        cond,
                        arg: cmp_arg,
                        imm: cmp_imm,
                        ..
                    } => CmpBrInfo {
                        br_inst: inst,
                        cmp_inst: cond_inst,
                        destination,
                        args: args.clone(),
                        cmp_arg,
                        invert_branch_cond,
                        kind: CmpBrKind::IcmpImm { cond, imm: cmp_imm },
                    },
                    InstructionData::FloatCompare {
                        cond,
                        args: cmp_args,
                        ..
                    } => CmpBrInfo {
                        br_inst: inst,
                        cmp_inst: cond_inst,
                        destination,
                        args: args.clone(),
                        cmp_arg: cmp_args[0],
                        invert_branch_cond,
                        kind: CmpBrKind::Fcmp {
                            cond,
                            arg: cmp_args[1],
                        },
                    },
                    _ => return,
                }
            } else {
                return;
            }
        }
        // TODO: trapif, trueif, selectif, and their ff counterparts.
        _ => return,
    };

    // If any instructions clobber the flags between the comparison and the branch,
    // don't optimize them.
    if last_flags_clobber != Some(info.cmp_inst) {
        return;
    }

    // We found a compare+branch pattern. Transform it to use flags.
    let args = info.args.as_slice(&pos.func.dfg.value_lists)[1..].to_vec();
    pos.goto_inst(info.cmp_inst);
    pos.use_srcloc(info.cmp_inst);
    match info.kind {
        CmpBrKind::Icmp { mut cond, arg } => {
            let flags = pos.ins().ifcmp(info.cmp_arg, arg);
            pos.func.dfg.replace(info.cmp_inst).trueif(cond, flags);
            if info.invert_branch_cond {
                cond = cond.inverse();
            }
            pos.func
                .dfg
                .replace(info.br_inst)
                .brif(cond, flags, info.destination, &args);
        }
        CmpBrKind::IcmpImm { mut cond, imm } => {
            let flags = pos.ins().ifcmp_imm(info.cmp_arg, imm);
            pos.func.dfg.replace(info.cmp_inst).trueif(cond, flags);
            if info.invert_branch_cond {
                cond = cond.inverse();
            }
            pos.func
                .dfg
                .replace(info.br_inst)
                .brif(cond, flags, info.destination, &args);
        }
        CmpBrKind::Fcmp { mut cond, arg } => {
            let flags = pos.ins().ffcmp(info.cmp_arg, arg);
            pos.func.dfg.replace(info.cmp_inst).trueff(cond, flags);
            if info.invert_branch_cond {
                cond = cond.inverse();
            }
            pos.func
                .dfg
                .replace(info.br_inst)
                .brff(cond, flags, info.destination, &args);
        }
    }
    let ok = pos.func.update_encoding(info.cmp_inst, isa).is_ok();
    debug_assert!(ok);
    let ok = pos.func.update_encoding(info.br_inst, isa).is_ok();
    debug_assert!(ok);
}

struct MemOpInfo {
    opcode: Opcode,
    itype: Type,
    arg: Value,
    st_arg: Option<Value>,
    flags: MemFlags,
    offset: Offset32,
}

fn optimize_complex_addresses(pos: &mut EncCursor, inst: Inst, isa: &dyn TargetIsa) {
    // Look for simple loads and stores we can optimize.
    let info = match pos.func.dfg[inst] {
        InstructionData::Load {
            opcode,
            arg,
            flags,
            offset,
        } => MemOpInfo {
            opcode,
            itype: pos.func.dfg.ctrl_typevar(inst),
            arg,
            st_arg: None,
            flags,
            offset,
        },
        InstructionData::Store {
            opcode,
            args,
            flags,
            offset,
        } => MemOpInfo {
            opcode,
            itype: pos.func.dfg.ctrl_typevar(inst),
            arg: args[1],
            st_arg: Some(args[0]),
            flags,
            offset,
        },
        _ => return,
    };

    // Examine the instruction that defines the address operand.
    if let ValueDef::Result(result_inst, _) = pos.func.dfg.value_def(info.arg) {
        match pos.func.dfg[result_inst] {
            InstructionData::Binary {
                opcode: Opcode::Iadd,
                args,
            } => match info.opcode {
                // Operand is an iadd. Fold it into a memory address with a complex address mode.
                Opcode::Load => {
                    pos.func.dfg.replace(inst).load_complex(
                        info.itype,
                        info.flags,
                        &args,
                        info.offset,
                    );
                }
                Opcode::Uload8 => {
                    pos.func.dfg.replace(inst).uload8_complex(
                        info.itype,
                        info.flags,
                        &args,
                        info.offset,
                    );
                }
                Opcode::Sload8 => {
                    pos.func.dfg.replace(inst).sload8_complex(
                        info.itype,
                        info.flags,
                        &args,
                        info.offset,
                    );
                }
                Opcode::Uload16 => {
                    pos.func.dfg.replace(inst).uload16_complex(
                        info.itype,
                        info.flags,
                        &args,
                        info.offset,
                    );
                }
                Opcode::Sload16 => {
                    pos.func.dfg.replace(inst).sload16_complex(
                        info.itype,
                        info.flags,
                        &args,
                        info.offset,
                    );
                }
                Opcode::Uload32 => {
                    pos.func
                        .dfg
                        .replace(inst)
                        .uload32_complex(info.flags, &args, info.offset);
                }
                Opcode::Sload32 => {
                    pos.func
                        .dfg
                        .replace(inst)
                        .sload32_complex(info.flags, &args, info.offset);
                }
                Opcode::Store => {
                    pos.func.dfg.replace(inst).store_complex(
                        info.flags,
                        info.st_arg.unwrap(),
                        &args,
                        info.offset,
                    );
                }
                Opcode::Istore8 => {
                    pos.func.dfg.replace(inst).istore8_complex(
                        info.flags,
                        info.st_arg.unwrap(),
                        &args,
                        info.offset,
                    );
                }
                Opcode::Istore16 => {
                    pos.func.dfg.replace(inst).istore16_complex(
                        info.flags,
                        info.st_arg.unwrap(),
                        &args,
                        info.offset,
                    );
                }
                Opcode::Istore32 => {
                    pos.func.dfg.replace(inst).istore32_complex(
                        info.flags,
                        info.st_arg.unwrap(),
                        &args,
                        info.offset,
                    );
                }
                _ => panic!("Unsupported load or store opcode"),
            },
            InstructionData::BinaryImm {
                opcode: Opcode::IaddImm,
                arg,
                imm,
            } => match pos.func.dfg[inst] {
                // Operand is an iadd_imm. Fold the immediate into the offset if possible.
                InstructionData::Load {
                    arg: ref mut load_arg,
                    ref mut offset,
                    ..
                } => {
                    if let Some(imm) = offset.try_add_i64(imm.into()) {
                        *load_arg = arg;
                        *offset = imm;
                    } else {
                        // Overflow.
                        return;
                    }
                }
                InstructionData::Store {
                    args: ref mut store_args,
                    ref mut offset,
                    ..
                } => {
                    if let Some(imm) = offset.try_add_i64(imm.into()) {
                        store_args[1] = arg;
                        *offset = imm;
                    } else {
                        // Overflow.
                        return;
                    }
                }
                _ => panic!(),
            },
            _ => {
                // Address value is defined by some other kind of instruction.
                return;
            }
        }
    } else {
        // Address value is not the result of an instruction.
        return;
    }

    let ok = pos.func.update_encoding(inst, isa).is_ok();
    debug_assert!(ok);
}

//----------------------------------------------------------------------
//
// The main post-opt pass.

pub fn do_postopt(func: &mut Function, isa: &dyn TargetIsa) {
    let _tt = timing::postopt();
    let mut pos = EncCursor::new(func, isa);
    while let Some(_block) = pos.next_block() {
        let mut last_flags_clobber = None;
        while let Some(inst) = pos.next_inst() {
            if isa.uses_cpu_flags() {
                // Optimize instructions to make use of flags.
                optimize_cpu_flags(&mut pos, inst, last_flags_clobber, isa);

                // Track the most recent seen instruction that clobbers the flags.
                if let Some(constraints) = isa
                    .encoding_info()
                    .operand_constraints(pos.func.encodings[inst])
                {
                    if constraints.clobbers_flags {
                        last_flags_clobber = Some(inst)
                    }
                }
            }

            if isa.uses_complex_addresses() {
                optimize_complex_addresses(&mut pos, inst, isa);
            }
        }
    }
}
