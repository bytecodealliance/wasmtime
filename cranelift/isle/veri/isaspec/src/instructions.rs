use crate::{
    aarch64,
    bits::{Bits, Segment},
    builder::{
        Arm, Case, Cases, InstConfig, Mapping, MappingBuilder, Mappings, Match, Opcodes, SpecConfig,
    },
    configure::{flags_mappings, reg_target, spec_fp_reg, verify_opcode_template},
    constraints::Target,
    memory::{ReadEffect, SetEffect},
    spec::{
        spec_as_bit_vector_width, spec_binary, spec_const_bit_vector, spec_const_int,
        spec_discriminator, spec_eq, spec_eq_bool, spec_extract, spec_false, spec_field, spec_true,
        spec_var,
    },
    spec_config,
};
use anyhow::Result;
use cranelift_codegen::{
    Reg, Writable,
    ir::{MemFlagsData, types::I8},
    isa::aarch64::inst::{
        ALUOp, ALUOp3, AMode, ASIMDFPModImm, ASIMDMovModImm, BitOp, Cond, ExtendOp,
        FPULeftShiftImm, FPUOp1, FPUOp2, FPUOpRI, FPUOpRIMod, FPURightShiftImm, FpuRoundMode,
        FpuToIntOp, Imm12, Inst, IntToFpuOp, MoveWideConst, MoveWideOp, NZCV, OperandSize, SImm9,
        ScalarSize, ShiftOp, ShiftOpAndAmt, ShiftOpShiftImm, UImm5, UImm12Scaled, VecALUOp,
        VecLanesOp, VecMisc2, VectorSize, vreg, writable_vreg, writable_xreg, xreg,
    },
};
use cranelift_isle::ast::{SpecExpr, SpecOp};
use itertools::Itertools;
use std::collections::HashMap;
use std::path::PathBuf;
use std::vec;

/// Configuration for an ISLE specification file to generate.
pub struct FileConfig {
    pub name: PathBuf,
    pub specs: Vec<SpecConfig>,
}

/// Define specifications to generate.
pub fn define() -> Result<Vec<FileConfig>> {
    Ok(vec![
        FileConfig {
            name: "alu_rrr.isle".into(),
            specs: vec![define_alu_rrr()],
        },
        FileConfig {
            name: "alu_rrrr.isle".into(),
            specs: vec![define_alu_rrrr()],
        },
        FileConfig {
            name: "alu_rr_imm12.isle".into(),
            specs: vec![define_alu_rr_imm12()?],
        },
        FileConfig {
            name: "alu_rrr_shift.isle".into(),
            specs: vec![define_alu_rrr_shift()?],
        },
        FileConfig {
            name: "alu_rrr_extend.isle".into(),
            specs: vec![define_alu_rrr_extend()],
        },
        FileConfig {
            name: "bit_rr.isle".into(),
            specs: vec![define_bit_rr()],
        },
        FileConfig {
            name: "loads.isle".into(),
            specs: define_loads()?,
        },
        FileConfig {
            name: "stores.isle".into(),
            specs: define_stores()?,
        },
        FileConfig {
            name: "mov_wide.isle".into(),
            specs: vec![define_mov_wide()?],
        },
        FileConfig {
            name: "extend.isle".into(),
            specs: vec![define_extend()],
        },
        FileConfig {
            name: "conds.isle".into(),
            specs: define_conds()?,
        },
        FileConfig {
            name: "fpu_move_imm.isle".into(),
            specs: vec![define_fpu_move_imm()],
        },
        FileConfig {
            name: "fpu_cmp.isle".into(),
            specs: vec![define_fpu_cmp()],
        },
        FileConfig {
            name: "fpu_rr.isle".into(),
            specs: vec![define_fpu_rr()],
        },
        FileConfig {
            name: "fpu_round.isle".into(),
            specs: vec![define_fpu_round()],
        },
        FileConfig {
            name: "fpu_rri.isle".into(),
            specs: vec![define_fpu_rri()],
        },
        FileConfig {
            name: "fpu_rrimod.isle".into(),
            specs: vec![define_fpu_rrimod()],
        },
        FileConfig {
            name: "fpu_rrr.isle".into(),
            specs: vec![define_fpu_rrr()],
        },
        FileConfig {
            name: "mov_to_fpu.isle".into(),
            specs: vec![define_mov_to_fpu()],
        },
        FileConfig {
            name: "int_to_fpu.isle".into(),
            specs: vec![define_int_to_fpu()],
        },
        FileConfig {
            name: "fpu_to_int.isle".into(),
            specs: vec![define_fpu_to_int()],
        },
        FileConfig {
            name: "mov_from_vec.isle".into(),
            specs: vec![define_mov_from_vec()],
        },
        FileConfig {
            name: "vec_dup_imm.isle".into(),
            specs: vec![define_vec_dup_imm()?],
        },
        FileConfig {
            name: "vec_rrr.isle".into(),
            specs: vec![define_vec_rrr()],
        },
        FileConfig {
            name: "vec_misc.isle".into(),
            specs: vec![define_vec_misc()],
        },
        FileConfig {
            name: "vec_lanes.isle".into(),
            specs: vec![define_vec_lanes()],
        },
    ])
}

// MInst.AluRRR specification configuration.
fn define_alu_rrr() -> SpecConfig {
    let sizes = [OperandSize::Size64, OperandSize::Size32];
    let alu_ops = [
        ALUOp::Add,
        ALUOp::Sub,
        ALUOp::Orr,
        ALUOp::OrrNot,
        ALUOp::And,
        ALUOp::AndNot,
        ALUOp::Eor,
        ALUOp::EorNot,
        ALUOp::AddS,
        ALUOp::SubS,
        ALUOp::SMulH,
        ALUOp::UMulH,
        ALUOp::SDiv,
        ALUOp::UDiv,
        ALUOp::Extr,
        ALUOp::Lsr,
        ALUOp::Asr,
        ALUOp::Lsl,
        ALUOp::Adc,
        // Flag ops not required yet:
        // ALUOp::Sbc,
        // ALUOp::AdcS,
        // ALUOp::SbcS,
    ];

    spec_config! {
        (AluRRR alu_op size rd rn rm)
        {
            enumerate   (size, sizes);
            enumerate   (alu_op, alu_ops);
            register    (rd, write, gp, 4);
            register    (rn, read,  gp, 5);
            register    (rm, read,  gp, 6);
            filter      (is_alu_op_size_supported(alu_op, size));
            flags       ();
            instruction ();
        }
    }
}

fn is_alu_op_size_supported(alu_op: ALUOp, size: OperandSize) -> bool {
    match alu_op {
        ALUOp::SMulH | ALUOp::UMulH => size == OperandSize::Size64,
        _ => true,
    }
}

// MInst.AluRRRR specification configuration.
fn define_alu_rrrr() -> SpecConfig {
    let sizes = [OperandSize::Size64, OperandSize::Size32];
    let alu3_ops = [ALUOp3::MAdd, ALUOp3::MSub, ALUOp3::UMAddL, ALUOp3::SMAddL];
    spec_config! {
        (AluRRRR alu_op size rd rn rm ra)
        {
            enumerate   (size, sizes);
            enumerate   (alu_op, alu3_ops);
            register    (rd, write, gp, 4);
            register    (rn, read,  gp, 5);
            register    (rm, read,  gp, 6);
            register    (ra, read,  gp, 7);
            filter      (is_alu3_op_size_supported(alu_op, size));
            instruction ();
        }
    }
}

fn is_alu3_op_size_supported(alu3_op: ALUOp3, size: OperandSize) -> bool {
    match alu3_op {
        ALUOp3::UMAddL | ALUOp3::SMAddL => size == OperandSize::Size32,
        _ => true,
    }
}

// MInst.AluRRImm12 specification configuration.
fn define_alu_rr_imm12() -> Result<SpecConfig> {
    // ALUOps supported by AluRRImm12.
    let alu_ops = [ALUOp::Add, ALUOp::Sub, ALUOp::AddS, ALUOp::SubS];

    // OperandSize
    let sizes = [OperandSize::Size32, OperandSize::Size64];

    // Imm12.shift12
    let shift12s = [false, true];

    // Execution scope: define opcode template fields.
    let mut scope = aarch64::state();
    let imm12_bits = Target::Var("bits".to_string());
    scope.global(imm12_bits.clone());

    // Mappings
    let mut mappings = flags_mappings();
    mappings.writes.insert(
        aarch64::gpreg(4),
        Mapping::require(spec_var("rd".to_string())),
    );
    mappings.reads.insert(
        aarch64::gpreg(5),
        Mapping::require(spec_var("rn".to_string())),
    );
    mappings.reads.insert(
        imm12_bits.clone(),
        MappingBuilder::var("imm12").field("bits").build(),
    );

    Ok(SpecConfig {
        term: "MInst.AluRRImm12".to_string(),
        args: ["alu_op", "size", "rd", "rn", "imm12"]
            .map(String::from)
            .to_vec(),
        cases: Cases::Match(Match {
            on: spec_var("size".to_string()),
            arms: sizes
                .iter()
                .rev()
                .map(|size| {
                    Ok(Arm {
                        variant: format!("{size:?}"),
                        args: Vec::new(),
                        body: Cases::Match(Match {
                            on: spec_var("alu_op".to_string()),
                            arms: alu_ops
                                .iter()
                                .map(|alu_op| {
                                    Ok(Arm {
                                        variant: format!("{alu_op:?}"),
                                        args: Vec::new(),
                                        body: Cases::Cases(
                                            shift12s
                                                .iter()
                                                .map(|shift12| {
                                                    let template = alu_rr_imm12_template(
                                                        *alu_op,
                                                        *size,
                                                        writable_xreg(4),
                                                        xreg(5),
                                                        *shift12,
                                                    )?;
                                                    Ok(Case {
                                                        conds: vec![spec_eq_bool(
                                                            spec_field(
                                                                "shift12".to_string(),
                                                                spec_var("imm12".to_string()),
                                                            ),
                                                            *shift12,
                                                        )],
                                                        cases: Cases::Instruction(InstConfig {
                                                            opcodes: Opcodes::Template(template),
                                                            scope: scope.clone(),
                                                            mappings: mappings.clone(),
                                                        }),
                                                    })
                                                })
                                                .collect::<Result<_>>()?,
                                        ),
                                    })
                                })
                                .collect::<Result<_>>()?,
                        }),
                    })
                })
                .collect::<Result<_>>()?,
        }),
    })
}

fn alu_rr_imm12_template(
    alu_op: ALUOp,
    size: OperandSize,
    rd: Writable<Reg>,
    rn: Reg,
    shift12: bool,
) -> Result<Bits> {
    // Assemble a base instruction with a placeholder for the imm12 field.
    let placeholder = Imm12 { bits: 0, shift12 };
    let base = Inst::AluRRImm12 {
        alu_op,
        size,
        rd,
        rn,
        imm12: placeholder,
    };
    let opcode = aarch64::opcode(&base);
    let bits = Bits::from_u32(opcode);

    // Splice in symbolic immediate fields.
    let imm = Bits {
        segments: vec![Segment::Symbolic("bits".to_string(), 12)],
    };
    let template = Bits::splice(&bits, &imm, 10)?;

    // Verify template against the assembler.
    verify_opcode_template(&template, |assignment: &HashMap<String, u32>| {
        let bits = assignment.get("bits").unwrap();
        let imm12 = Imm12 {
            bits: (*bits).try_into().unwrap(),
            shift12,
        };
        Ok(Inst::AluRRImm12 {
            alu_op,
            size,
            rd,
            rn,
            imm12,
        })
    })?;

    Ok(template)
}

// MInst.AluRRRShift specification configuration.
fn define_alu_rrr_shift() -> Result<SpecConfig> {
    // ALUOps supported by AluRRImm12.
    let alu_ops = [
        ALUOp::Add,
        ALUOp::Sub,
        ALUOp::Orr,
        ALUOp::And,
        ALUOp::Eor,
        ALUOp::OrrNot,
        ALUOp::EorNot,
        ALUOp::AndNot,
        ALUOp::Extr,
        // Flags:
        // ALUOp::AddS,
        // ALUOp::SubS,
        // ALUOp::AndS,
    ];

    // OperandSize
    let sizes = [OperandSize::Size32, OperandSize::Size64];

    // ShiftOp
    //
    // For the genuinely shifted-register ops, LSL/LSR/ASR are all valid for
    // both operand sizes. ShiftOp::ROR is a defined variant but not a valid
    // opcode here, so it is omitted.
    let default_shiftops = [ShiftOp::LSL, ShiftOp::LSR, ShiftOp::ASR];

    // `Extr` reuses the `AluRRRShift` shape but is not actually a
    // shifted-register instruction: the low bit of the shift-op field encodes
    // the architectural `N` bit, which AArch64 requires to equal `sf` (the
    // operand-size bit), otherwise the encoding is UNDEFINED. So `Extr` only
    // assembles to a real instruction for the shift-ops whose encoding makes
    // `N == sf`: LSL (0b00, N=0) for the 32-bit size and LSR (0b01, N=1) for
    // the 64-bit size. This matches `a64_extr_imm`, the only producer of
    // `AluRRRShift { Extr, .. }` in lowering. See `alu_rrr_shift_sizes` for the
    // size restriction that pairs each of these shift-ops with its one valid
    // operand size.
    let extr_shiftops = [ShiftOp::LSL, ShiftOp::LSR];

    Ok(SpecConfig {
        term: "MInst.AluRRRShift".to_string(),
        args: ["alu_op", "size", "rd", "rn", "rm", "shiftop"]
            .map(String::from)
            .to_vec(),
        cases: Cases::Match(Match {
            on: spec_var("alu_op".to_string()),
            arms: alu_ops
                .iter()
                .map(|alu_op| {
                    let shiftops: &[ShiftOp] = if matches!(*alu_op, ALUOp::Extr) {
                        &extr_shiftops
                    } else {
                        &default_shiftops
                    };
                    Ok(Arm {
                        variant: format!("{alu_op:?}"),
                        args: Vec::new(),

                        // Shift operation cases.
                        //
                        // Note that the `ShiftOpAndAmt` model actually uses
                        // `ALUOp` to represent the shift operation.  ISLE does
                        // not actually materialize the ShiftOp type itself, and
                        // without support for ghost types, we cannot represent
                        // it.  Therefore, co-opting ALUOp for the purpose is
                        // the best we can do.
                        body: Cases::Match(Match {
                            on: spec_field("op".to_string(), spec_var("shiftop".to_string())),
                            arms: shiftops
                                .iter()
                                .map(|shiftop| {
                                    // Map shift operation to the correspondong ALUOp.
                                    let alu_shift_op = alu_op_from_shiftop(*shiftop);
                                    Ok(Arm {
                                        variant: format!("{alu_shift_op:?}"),
                                        args: Vec::new(),
                                        body: Cases::Cases(
                                            alu_rrr_shift_sizes(*alu_op, *shiftop, &sizes)
                                                .into_iter()
                                                .map(|size| {
                                                    alu_rrr_shift_size_case(*alu_op, size, *shiftop)
                                                })
                                                .collect::<Result<_>>()?,
                                        ),
                                    })
                                })
                                .collect::<Result<_>>()?,
                        }),
                    })
                })
                .collect::<Result<_>>()?,
        }),
    })
}

fn alu_rrr_shift_size_case(alu_op: ALUOp, size: OperandSize, op: ShiftOp) -> Result<Case> {
    // Shift amount field depends on operand size.
    let amt_width = match size {
        OperandSize::Size32 => 5,
        OperandSize::Size64 => 6,
    };
    let amt_var = format!("amt{}", amt_width);

    // Setup scope with shift amount variable.
    let amt_target = Target::Var(amt_var.clone());
    let mut scope = aarch64::state();
    scope.global(amt_target.clone());

    // Expressions for the shift amount.
    //
    // The model of the shift amount is an 8 bit value, but the instruction
    // representations only allow 5 or 6 bits (depending on operand size).  We
    // extract the shift bits from the operand, and require that the higher bits
    // are zero.
    static FULL_AMT_WIDTH: usize = 8;
    let full_amt_expr = spec_field("amt".to_string(), spec_var("shiftop".to_string()));
    let amt_expr = spec_extract(amt_width - 1, 0, full_amt_expr.clone());
    let amt_overflow_expr = spec_extract(FULL_AMT_WIDTH - 1, amt_width, full_amt_expr.clone());
    let amt_overflow_width = FULL_AMT_WIDTH - amt_width;
    let no_amt_overflow = spec_eq(
        amt_overflow_expr,
        spec_const_bit_vector(0, amt_overflow_width),
    );

    // Mappings
    let mut mappings = flags_mappings();
    mappings.writes.insert(
        aarch64::gpreg(4),
        Mapping::require(spec_var("rd".to_string())),
    );
    mappings.reads.insert(
        aarch64::gpreg(5),
        Mapping::require(spec_var("rn".to_string())),
    );
    mappings.reads.insert(
        aarch64::gpreg(6),
        Mapping::require(spec_var("rm".to_string())),
    );
    mappings
        .reads
        .insert(amt_target.clone(), Mapping::require(amt_expr.clone()));

    // Opcode template
    //
    // Assemble a base instruction with a placeholder for the shift amount.
    let placeholder = ShiftOpShiftImm::maybe_from_shift(0).unwrap();
    let rd = writable_xreg(4);
    let rn = xreg(5);
    let rm = xreg(6);
    let shiftop = ShiftOpAndAmt::new(op, placeholder);
    let base = Inst::AluRRRShift {
        alu_op,
        size,
        rd,
        rn,
        rm,
        shiftop,
    };
    let opcode = aarch64::opcode(&base);
    let bits = Bits::from_u32(opcode);

    // Splice in symbolic shift amount.
    //
    // The shift amount is 6 bits in the 64-bit case, and 5 bits in the 32-bit
    // case.  Note that in the 32-bit case, the instruction is explicitly
    // undefined when bit 5 is 1. Therefore, we must ensure that the symbolic
    // field variable is only 5 bits.
    let amt = Bits {
        segments: vec![Segment::Symbolic(amt_var.to_string(), amt_width)],
    };
    let template = Bits::splice(&bits, &amt, 10)?;

    // Verify template against the assembler.
    verify_opcode_template(&template, |assignment: &HashMap<String, u32>| {
        let amt = assignment.get(&amt_var).unwrap();
        let shift = ShiftOpShiftImm::maybe_from_shift((*amt).into()).unwrap();
        let shiftop = ShiftOpAndAmt::new(op, shift);
        Ok(Inst::AluRRRShift {
            alu_op,
            size,
            rd,
            rn,
            rm,
            shiftop,
        })
    })?;

    Ok(Case {
        conds: vec![
            spec_discriminator(format!("{size:?}"), spec_var("size".to_string())),
            no_amt_overflow,
        ],
        cases: Cases::Instruction(InstConfig {
            opcodes: Opcodes::Template(template),
            scope: scope.clone(),
            mappings: mappings.clone(),
        }),
    })
}

/// Operand sizes for which the `(alu_op, op)` pairing assembles to a real
/// instruction, in the order they should appear in the generated spec.
fn alu_rrr_shift_sizes(alu_op: ALUOp, op: ShiftOp, sizes: &[OperandSize]) -> Vec<OperandSize> {
    match (alu_op, op) {
        // `Extr` requires the architectural `N` bit (the low bit of the
        // shift-op encoding) to equal `sf`, so each shift-op is valid for
        // exactly one operand size: LSL (N=0) -> 32-bit, LSR (N=1) -> 64-bit.
        // Any other pairing emits an UNDEFINED encoding (`N != sf`).
        (ALUOp::Extr, ShiftOp::LSL) => vec![OperandSize::Size32],
        (ALUOp::Extr, ShiftOp::LSR) => vec![OperandSize::Size64],
        _ => sizes.iter().rev().copied().collect(),
    }
}

fn alu_op_from_shiftop(op: ShiftOp) -> ALUOp {
    match op {
        ShiftOp::LSL => ALUOp::Lsl,
        ShiftOp::LSR => ALUOp::Lsr,
        ShiftOp::ASR => ALUOp::Asr,
        ShiftOp::ROR => ALUOp::Extr,
    }
}

// MInst.AluRRRExtend specification configuration.
fn define_alu_rrr_extend() -> SpecConfig {
    // ALUOps supported by AluRRRExtend.
    let alu_ops = [ALUOp::Add, ALUOp::Sub, ALUOp::AddS, ALUOp::SubS];

    // OperandSize
    let sizes = [OperandSize::Size64, OperandSize::Size32];

    // ExtendOp
    let extendops = [
        ExtendOp::UXTB,
        ExtendOp::UXTH,
        ExtendOp::UXTW,
        ExtendOp::UXTX,
        ExtendOp::SXTB,
        ExtendOp::SXTH,
        ExtendOp::SXTW,
        ExtendOp::SXTX,
    ];

    spec_config! {
        (AluRRRExtend alu_op size rd rn rm extendop)
        {
            enumerate   (size, sizes);
            enumerate   (alu_op, alu_ops);
            register    (rd, write, gp, 4);
            register    (rn, read,  gp, 5);
            register    (rm, read,  gp, 6);
            enumerate   (extendop, extendops);
            flags       ();
            instruction ();
        }
    }
}

// MInst.BitRR specification configuration.
fn define_bit_rr() -> SpecConfig {
    // BitRR
    let bit_ops = [
        BitOp::Cls,
        BitOp::RBit,
        BitOp::Clz,
        // --------------
        // BitOp::Rev16,
        // BitOp::Rev32,
        // BitOp::Rev64,
    ];

    // OperandSize
    let sizes = [OperandSize::Size64, OperandSize::Size32];

    spec_config! {
        (BitRR op size rd rn)
        {
            enumerate   (size, sizes);
            enumerate   (op, bit_ops);
            register    (rd, write, gp, 4);
            register    (rn, read,  gp64, 5);
            instruction ();
        }
    }
}

// MInst.MovWide specification configuration.
fn define_mov_wide() -> Result<SpecConfig> {
    // MovWideOps
    let mov_wide_ops = [MoveWideOp::MovZ, MoveWideOp::MovN];

    // OperandSize
    let sizes = [OperandSize::Size32, OperandSize::Size64];

    // Execution scope: define opcode template fields.
    let mut scope = aarch64::state();
    let mov_wide_const_bits = Target::Var("bits".to_string());
    scope.global(mov_wide_const_bits.clone());

    // Mappings
    let mut mappings = Mappings::default();
    mappings.writes.insert(
        aarch64::gpreg(4),
        Mapping::require(spec_var("rd".to_string())),
    );
    mappings.reads.insert(
        mov_wide_const_bits.clone(),
        MappingBuilder::var("imm").field("bits").build(),
    );

    Ok(SpecConfig {
        term: "MInst.MovWide".to_string(),
        args: ["op", "rd", "imm", "size"].map(String::from).to_vec(),
        cases: Cases::Match(Match {
            on: spec_var("size".to_string()),
            arms: sizes
                .iter()
                .rev()
                .map(|size| {
                    let max_shift = size.bits() / 16;
                    Ok(Arm {
                        variant: format!("{size:?}"),
                        args: Vec::new(),
                        body: Cases::Match(Match {
                            on: spec_var("op".to_string()),
                            arms: mov_wide_ops
                                .iter()
                                .map(|mov_wide_op| {
                                    Ok(Arm {
                                        variant: format!("{mov_wide_op:?}"),
                                        args: Vec::new(),
                                        body: Cases::Cases(
                                            (0..max_shift)
                                                .map(|shift| {
                                                    let template = mov_wide_template(
                                                        *mov_wide_op,
                                                        writable_xreg(4),
                                                        shift,
                                                        *size,
                                                    )?;
                                                    Ok(Case {
                                                        conds: vec![spec_eq(
                                                            spec_field(
                                                                "shift".to_string(),
                                                                spec_var("imm".to_string()),
                                                            ),
                                                            spec_const_bit_vector(shift.into(), 2),
                                                        )],
                                                        cases: Cases::Instruction(InstConfig {
                                                            opcodes: Opcodes::Template(template),
                                                            scope: scope.clone(),
                                                            mappings: mappings.clone(),
                                                        }),
                                                    })
                                                })
                                                .collect::<Result<_>>()?,
                                        ),
                                    })
                                })
                                .collect::<Result<_>>()?,
                        }),
                    })
                })
                .collect::<Result<_>>()?,
        }),
    })
}

fn mov_wide_template(
    op: MoveWideOp,
    rd: Writable<Reg>,
    shift: u8,
    size: OperandSize,
) -> Result<Bits> {
    // Assemble a base instruction with a placeholder for the immediate bits field.
    let placeholder = MoveWideConst { bits: 0, shift };
    let base = Inst::MovWide {
        op,
        rd,
        imm: placeholder,
        size,
    };
    let opcode = aarch64::opcode(&base);
    let bits = Bits::from_u32(opcode);

    // Splice in symbolic immediate fields.
    let imm = Bits {
        segments: vec![Segment::Symbolic("bits".to_string(), 16)],
    };
    let template = Bits::splice(&bits, &imm, 5)?;

    // Verify template against the assembler.
    verify_opcode_template(&template, |assignment: &HashMap<String, u32>| {
        let bits = assignment.get("bits").unwrap();
        let imm = MoveWideConst {
            bits: (*bits).try_into().unwrap(),
            shift,
        };
        Ok(Inst::MovWide { op, rd, imm, size })
    })?;

    Ok(template)
}

fn define_extend() -> SpecConfig {
    // Extend
    let signed = [false, true];
    let bits = [8u8, 16u8, 32u8, 64u8];

    let mut mappings = Mappings::default();
    mappings.writes.insert(
        aarch64::gpreg(4),
        Mapping::require(spec_var("rd".to_string())),
    );
    mappings.reads.insert(
        aarch64::gpreg(5),
        Mapping::require(spec_as_bit_vector_width(spec_var("rn".to_string()), 64)),
    );

    SpecConfig {
        // Spec signature.
        term: "MInst.Extend".to_string(),
        args: ["rd", "rn", "signed", "from_bits", "to_bits"]
            .map(String::from)
            .to_vec(),
        cases: Cases::Cases(
            bits.iter()
                .cartesian_product(&bits)
                .filter(|(from_bits, to_bits)| from_bits <= to_bits && **from_bits < 64)
                .cartesian_product(&signed)
                .map(|((from_bits, to_bits), signed)| Case {
                    conds: vec![
                        spec_eq_bool(spec_var("signed".to_string()), *signed),
                        spec_eq(
                            spec_var("from_bits".to_string()),
                            spec_const_bit_vector((*from_bits).into(), 8),
                        ),
                        spec_eq(
                            spec_var("to_bits".to_string()),
                            spec_const_bit_vector((*to_bits).into(), 8),
                        ),
                    ],
                    cases: Cases::Instruction(InstConfig {
                        // Instruction to generate specification from.
                        opcodes: Opcodes::Instruction(Inst::Extend {
                            rd: writable_xreg(4),
                            rn: xreg(5),
                            signed: *signed,
                            from_bits: *from_bits,
                            to_bits: *to_bits,
                        }),
                        scope: aarch64::state(),
                        mappings: mappings.clone(),
                    }),
                })
                .collect(),
        ),
    }
}

fn define_loads() -> Result<Vec<SpecConfig>> {
    // Destination register for general-purpose loads.
    let dst = writable_xreg(4);
    let rd = spec_var("rd".to_string());

    // ULoad8
    let uload8 = define_load("MInst.ULoad8", 8, dst, &rd, |rd, mem, flags| Inst::ULoad8 {
        rd,
        mem,
        flags,
    })?;

    // SLoad8
    let sload8 = define_load("MInst.SLoad8", 8, dst, &rd, |rd, mem, flags| Inst::SLoad8 {
        rd,
        mem,
        flags,
    })?;

    // ULoad16
    let uload16 = define_load("MInst.ULoad16", 16, dst, &rd, |rd, mem, flags| {
        Inst::ULoad16 { rd, mem, flags }
    })?;

    // SLoad16
    let sload16 = define_load("MInst.SLoad16", 16, dst, &rd, |rd, mem, flags| {
        Inst::SLoad16 { rd, mem, flags }
    })?;

    // ULoad32
    let uload32 = define_load("MInst.ULoad32", 32, dst, &rd, |rd, mem, flags| {
        Inst::ULoad32 { rd, mem, flags }
    })?;

    // SLoad32
    let sload32 = define_load("MInst.SLoad32", 32, dst, &rd, |rd, mem, flags| {
        Inst::SLoad32 { rd, mem, flags }
    })?;

    // ULoad64
    let uload64 = define_load("MInst.ULoad64", 64, dst, &rd, |rd, mem, flags| {
        Inst::ULoad64 { rd, mem, flags }
    })?;

    // Destination register for floating-point loads.
    let dst = writable_vreg(4);
    let rd = spec_fp_reg("rd");

    // FpuLoad32
    let fpu_load32 = define_load("MInst.FpuLoad32", 32, dst, &rd, |rd, mem, flags| {
        Inst::FpuLoad32 { rd, mem, flags }
    })?;

    // FpuLoad64
    let fpu_load64 = define_load("MInst.FpuLoad64", 64, dst, &rd, |rd, mem, flags| {
        Inst::FpuLoad64 { rd, mem, flags }
    })?;

    Ok(vec![
        uload8, sload8, uload16, sload16, uload32, sload32, uload64, fpu_load32, fpu_load64,
    ])
}

fn define_load<F>(
    term: &str,
    size_bits: usize,
    dst: Writable<Reg>,
    rd: &SpecExpr,
    inst: F,
) -> Result<SpecConfig>
where
    F: Fn(Writable<Reg>, AMode, MemFlagsData) -> Inst,
{
    // Mappings.
    let mut mappings = Mappings::default();

    // Destination register.
    mappings
        .writes
        .insert(reg_target(dst.to_reg())?, Mapping::require(rd.clone()));

    // ISA load state mapped to read effect.
    let read_effect = ReadEffect::new();
    static ISA_LOAD: &str = "isa_load";
    static LOADED_VALUE: &str = "loaded_value";
    mappings.writes.insert(
        read_effect.active,
        MappingBuilder::state(ISA_LOAD).field("active").build(),
    );
    mappings.writes.insert(
        read_effect.addr,
        MappingBuilder::state(ISA_LOAD).field("addr").build(),
    );
    mappings.writes.insert(
        read_effect.size_bits,
        MappingBuilder::state(ISA_LOAD).field("size_bits").build(),
    );
    mappings.reads.insert(
        read_effect.value,
        Mapping::require(spec_binary(
            SpecOp::ConvTo,
            spec_const_int(size_bits.try_into().unwrap()),
            spec_var(LOADED_VALUE.to_string()),
        )),
    );

    // Enumerate AModes.
    let arms = amode_cases(&mappings, |mem, flags| inst(dst, mem, flags))?;

    Ok(SpecConfig {
        term: term.to_string(),
        args: ["rd", "mem", "flags"].map(String::from).to_vec(),
        cases: Cases::Match(Match {
            on: spec_var("mem".to_string()),
            arms,
        }),
    })
}

fn define_stores() -> Result<Vec<SpecConfig>> {
    // Source register for general-purpose loads.
    let src = xreg(4);
    let rd = spec_as_bit_vector_width(spec_var("rd".to_string()), 64);

    // Store8
    let store8 = define_store("MInst.Store8", 8, src, &rd, |rd, mem, flags| Inst::Store8 {
        rd,
        mem,
        flags,
    })?;

    // Store16
    let store16 = define_store("MInst.Store16", 16, src, &rd, |rd, mem, flags| {
        Inst::Store16 { rd, mem, flags }
    })?;

    // Store32
    let store32 = define_store("MInst.Store32", 32, src, &rd, |rd, mem, flags| {
        Inst::Store32 { rd, mem, flags }
    })?;

    // Store64
    let store64 = define_store("MInst.Store64", 64, src, &rd, |rd, mem, flags| {
        Inst::Store64 { rd, mem, flags }
    })?;

    // Source register for floating-point stores.
    let src = vreg(4);
    let rd = spec_fp_reg("rd");

    // FpuStore32
    let fpu_store32 = define_store("MInst.FpuStore32", 32, src, &rd, |rd, mem, flags| {
        Inst::FpuStore32 { rd, mem, flags }
    })?;

    // FpuStore64
    let fpu_store64 = define_store("MInst.FpuStore64", 64, src, &rd, |rd, mem, flags| {
        Inst::FpuStore64 { rd, mem, flags }
    })?;

    Ok(vec![
        store8,
        store16,
        store32,
        store64,
        fpu_store32,
        fpu_store64,
    ])
}

fn define_store<F>(
    term: &str,
    size_bits: usize,
    src: Reg,
    rd: &SpecExpr,
    inst: F,
) -> Result<SpecConfig>
where
    F: Fn(Reg, AMode, MemFlagsData) -> Inst,
{
    // Mappings.
    let mut mappings = Mappings::default();

    // Source register.
    mappings
        .reads
        .insert(reg_target(src)?, Mapping::require(rd.clone()));

    // ISA store state mapped to memory set effect.
    let set_effect = SetEffect::new();
    static ISA_STORE: &str = "isa_store";
    mappings.writes.insert(
        set_effect.active,
        MappingBuilder::state(ISA_STORE).field("active").build(),
    );
    mappings.writes.insert(
        set_effect.addr,
        MappingBuilder::state(ISA_STORE).field("addr").build(),
    );
    mappings.writes.insert(
        set_effect.size_bits,
        MappingBuilder::state(ISA_STORE).field("size_bits").build(),
    );
    mappings.writes.insert(
        set_effect.value,
        MappingBuilder::new(spec_binary(
            SpecOp::ConvTo,
            spec_const_int(size_bits.try_into().unwrap()),
            spec_field("value".to_string(), spec_var(ISA_STORE.to_string())),
        ))
        .modifies(ISA_STORE)
        .build(),
    );

    // Enumerate AModes.
    let arms = amode_cases(&mappings, |mem, flags| inst(src, mem, flags))?;

    Ok(SpecConfig {
        term: term.to_string(),
        args: ["rd", "mem", "flags"].map(String::from).to_vec(),
        cases: Cases::Match(Match {
            on: spec_var("mem".to_string()),
            arms,
        }),
    })
}

fn amode_cases<F>(mappings: &Mappings, inst: F) -> Result<Vec<Arm>>
where
    F: Fn(AMode, MemFlagsData) -> Inst,
{
    // RegReg
    let mut reg_reg_mappings = mappings.clone();
    reg_reg_mappings.reads.insert(
        aarch64::gpreg(5),
        Mapping::require(spec_var("rn".to_string())),
    );
    reg_reg_mappings.reads.insert(
        aarch64::gpreg(6),
        Mapping::require(spec_var("rm".to_string())),
    );

    let reg_reg = Arm {
        variant: "RegReg".to_string(),
        args: ["rn", "rm"].map(String::from).to_vec(),
        body: Cases::Instruction(InstConfig {
            opcodes: Opcodes::Instruction(inst(
                AMode::RegReg {
                    rn: xreg(5),
                    rm: xreg(6),
                },
                MemFlagsData::new(),
            )),
            scope: aarch64::state(),
            mappings: reg_reg_mappings,
        }),
    };

    // RegScaled
    let mut reg_scaled_mappings = mappings.clone();
    reg_scaled_mappings.reads.insert(
        aarch64::gpreg(5),
        Mapping::require(spec_var("rn".to_string())),
    );
    reg_scaled_mappings.reads.insert(
        aarch64::gpreg(6),
        Mapping::require(spec_var("rm".to_string())),
    );

    let reg_scaled = Arm {
        variant: "RegScaled".to_string(),
        args: ["rn", "rm"].map(String::from).to_vec(),
        body: Cases::Instruction(InstConfig {
            opcodes: Opcodes::Instruction(inst(
                AMode::RegScaled {
                    rn: xreg(5),
                    rm: xreg(6),
                },
                MemFlagsData::new(),
            )),
            scope: aarch64::state(),
            mappings: reg_scaled_mappings,
        }),
    };

    // RegScaledExtended
    let extendops = [
        // Not supported by assembler: UXTB, UXTH, UXTX, SXTB, SXTH
        ExtendOp::UXTW,
        ExtendOp::SXTW,
        ExtendOp::SXTX,
    ];
    let mut reg_scaled_extended_mappings = mappings.clone();
    reg_scaled_extended_mappings.reads.insert(
        aarch64::gpreg(5),
        Mapping::require(spec_var("rn".to_string())),
    );
    reg_scaled_extended_mappings.reads.insert(
        aarch64::gpreg(6),
        Mapping::require(spec_var("rm".to_string())),
    );

    let reg_scaled_extended = Arm {
        variant: "RegScaledExtended".to_string(),
        args: ["rn", "rm", "extendop"].map(String::from).to_vec(),
        body: Cases::Match(Match {
            on: spec_var("extendop".to_string()),
            arms: extendops
                .into_iter()
                .map(|extendop| Arm {
                    variant: format!("{extendop:?}"),
                    args: Vec::new(),
                    body: Cases::Instruction(InstConfig {
                        opcodes: Opcodes::Instruction(inst(
                            AMode::RegScaledExtended {
                                rn: xreg(5),
                                rm: xreg(6),
                                extendop,
                            },
                            MemFlagsData::new(),
                        )),
                        scope: aarch64::state(),
                        mappings: reg_scaled_extended_mappings.clone(),
                    }),
                })
                .collect(),
        }),
    };

    // RegExtended
    let mut reg_extended_mappings = mappings.clone();
    reg_extended_mappings.reads.insert(
        aarch64::gpreg(5),
        Mapping::require(spec_var("rn".to_string())),
    );
    reg_extended_mappings.reads.insert(
        aarch64::gpreg(6),
        Mapping::require(spec_var("rm".to_string())),
    );

    let reg_extended = Arm {
        variant: "RegExtended".to_string(),
        args: ["rn", "rm", "extendop"].map(String::from).to_vec(),
        body: Cases::Match(Match {
            on: spec_var("extendop".to_string()),
            arms: extendops
                .into_iter()
                .map(|extendop| Arm {
                    variant: format!("{extendop:?}"),
                    args: Vec::new(),
                    body: Cases::Instruction(InstConfig {
                        opcodes: Opcodes::Instruction(inst(
                            AMode::RegExtended {
                                rn: xreg(5),
                                rm: xreg(6),
                                extendop,
                            },
                            MemFlagsData::new(),
                        )),
                        scope: aarch64::state(),
                        mappings: reg_extended_mappings.clone(),
                    }),
                })
                .collect(),
        }),
    };

    // Unscaled
    let mut unscaled_scope = aarch64::state();
    let simm9 = Target::Var("simm9".to_string());
    unscaled_scope.global(simm9.clone());

    let mut unscaled_mappings = mappings.clone();
    unscaled_mappings.reads.insert(
        aarch64::gpreg(5),
        Mapping::require(spec_var("rn".to_string())),
    );
    unscaled_mappings
        .reads
        .insert(simm9, Mapping::require(spec_var("simm9".to_string())));

    let unscaled_template =
        amode_unscaled_template(xreg(5), |amode| inst(amode, MemFlagsData::new()))?;

    let unscaled = Arm {
        variant: "Unscaled".to_string(),
        args: ["rn", "simm9"].map(String::from).to_vec(),
        body: Cases::Instruction(InstConfig {
            opcodes: Opcodes::Template(unscaled_template),
            scope: unscaled_scope,
            mappings: unscaled_mappings.clone(),
        }),
    };

    // UnsignedOffset
    let mut unsigned_offset_scope = aarch64::state();
    let uimm12 = Target::Var("uimm12".to_string());
    unsigned_offset_scope.global(uimm12.clone());

    let mut unsigned_offset_mappings = mappings.clone();
    unsigned_offset_mappings.reads.insert(
        aarch64::gpreg(5),
        Mapping::require(spec_var("rn".to_string())),
    );
    unsigned_offset_mappings
        .reads
        .insert(uimm12, Mapping::require(spec_var("uimm12".to_string())));

    let unsigned_offset_template =
        amode_unsigned_offset_template(xreg(5), |amode| inst(amode, MemFlagsData::new()))?;

    let unsigned_offset = Arm {
        variant: "UnsignedOffset".to_string(),
        args: ["rn", "uimm12"].map(String::from).to_vec(),
        body: Cases::Instruction(InstConfig {
            opcodes: Opcodes::Template(unsigned_offset_template),
            scope: unsigned_offset_scope,
            mappings: unsigned_offset_mappings,
        }),
    };

    Ok(vec![
        reg_reg,
        reg_scaled,
        reg_scaled_extended,
        reg_extended,
        unscaled,
        unsigned_offset,
    ])
}

fn amode_unscaled_template<F>(rn: Reg, inst: F) -> Result<Bits>
where
    F: Fn(AMode) -> Inst,
{
    // Assemble a base instruction with a placeholder for the immediate bits field.
    let placeholder = SImm9::maybe_from_i64(0).unwrap();
    let base = inst(AMode::Unscaled {
        rn,
        simm9: placeholder,
    });
    let opcode = aarch64::opcode(&base);
    let bits = Bits::from_u32(opcode);

    // Splice in symbolic immediate fields.
    let imm = Bits {
        segments: vec![Segment::Symbolic("simm9".to_string(), 9)],
    };
    let template = Bits::splice(&bits, &imm, 12)?;

    // Verify template against the assembler.
    verify_opcode_template(&template, |assignment: &HashMap<String, u32>| {
        let bits = assignment.get("simm9").unwrap();
        let imm = SImm9 {
            value: (*bits).try_into().unwrap(),
        };
        Ok(inst(AMode::Unscaled { rn, simm9: imm }))
    })?;

    Ok(template)
}

fn amode_unsigned_offset_template<F>(rn: Reg, inst: F) -> Result<Bits>
where
    F: Fn(AMode) -> Inst,
{
    // Assemble a base instruction with a placeholder for the immediate bits field.
    let placeholder = UImm12Scaled::zero(I8);
    let base = inst(AMode::UnsignedOffset {
        rn,
        uimm12: placeholder,
    });
    let opcode = aarch64::opcode(&base);
    let bits = Bits::from_u32(opcode);

    // Splice in symbolic immediate fields.
    let imm = Bits {
        segments: vec![Segment::Symbolic("uimm12".to_string(), 12)],
    };
    let template = Bits::splice(&bits, &imm, 10)?;

    // Verify template against the assembler.
    verify_opcode_template(&template, |assignment: &HashMap<String, u32>| {
        let bits = assignment.get("uimm12").unwrap();
        let uimm12 = UImm12Scaled::maybe_from_i64((*bits).into(), I8).unwrap();
        Ok(inst(AMode::UnsignedOffset { rn, uimm12 }))
    })?;

    Ok(template)
}

fn define_conds() -> Result<Vec<SpecConfig>> {
    // CSel
    let csel = define_csel("MInst.CSel", |rd, cond, rn, rm| Inst::CSel {
        rd,
        cond,
        rn,
        rm,
    });

    // CSNeg
    let csneg = define_csel("MInst.CSNeg", |rd, cond, rn, rm| Inst::CSNeg {
        rd,
        cond,
        rn,
        rm,
    });

    // CSet
    let cset = define_cset("MInst.CSet", |rd, cond| Inst::CSet { rd, cond });

    // CSetm
    let csetm = define_cset("MInst.CSetm", |rd, cond| Inst::CSetm { rd, cond });

    // CCmp
    let ccmp = define_ccmp()?;

    // CCmpImm
    let ccmp_imm = define_ccmp_imm()?;

    Ok(vec![csel, csneg, cset, csetm, ccmp, ccmp_imm])
}

fn define_csel<F>(term: &str, inst: F) -> SpecConfig
where
    F: Fn(Writable<Reg>, Cond, Reg, Reg) -> Inst,
{
    // Flags and register mappings.
    let mut mappings = flags_mappings();
    mappings.writes.insert(
        aarch64::gpreg(4),
        Mapping::require(spec_var("rd".to_string())),
    );
    mappings.reads.insert(
        aarch64::gpreg(5),
        Mapping::require(spec_var("rn".to_string())),
    );
    mappings.reads.insert(
        aarch64::gpreg(6),
        Mapping::require(spec_var("rm".to_string())),
    );

    SpecConfig {
        term: term.to_string(),
        args: ["rd", "cond", "rn", "rm"].map(String::from).to_vec(),

        cases: Cases::Match(Match {
            on: spec_var("cond".to_string()),
            arms: conds()
                .iter()
                .rev()
                .map(|cond| Arm {
                    variant: format!("{cond:?}"),
                    args: Vec::new(),
                    body: Cases::Instruction(InstConfig {
                        opcodes: Opcodes::Instruction(inst(
                            writable_xreg(4),
                            *cond,
                            xreg(5),
                            xreg(6),
                        )),
                        scope: aarch64::state(),
                        mappings: mappings.clone(),
                    }),
                })
                .collect(),
        }),
    }
}

fn define_cset<F>(term: &str, inst: F) -> SpecConfig
where
    F: Fn(Writable<Reg>, Cond) -> Inst,
{
    // Flags and register mappings.
    let mut mappings = flags_mappings();
    mappings.writes.insert(
        aarch64::gpreg(4),
        Mapping::require(spec_var("rd".to_string())),
    );

    SpecConfig {
        term: term.to_string(),
        args: ["rd", "cond"].map(String::from).to_vec(),

        cases: Cases::Match(Match {
            on: spec_var("cond".to_string()),
            arms: conds()
                .iter()
                .rev()
                .map(|cond| Arm {
                    variant: format!("{cond:?}"),
                    args: Vec::new(),
                    body: Cases::Instruction(InstConfig {
                        opcodes: Opcodes::Instruction(inst(writable_xreg(4), *cond)),
                        scope: aarch64::state(),
                        mappings: mappings.clone(),
                    }),
                })
                .collect(),
        }),
    }
}

fn define_ccmp() -> Result<SpecConfig> {
    // OperandSize
    let sizes = [OperandSize::Size32, OperandSize::Size64];

    // Execution scope: define opcode template fields.
    let mut scope = aarch64::state();
    for flag in &["n", "z", "c", "v"] {
        scope.global(Target::Var(flag.to_string()));
    }

    // Flags and register mappings.
    let mut mappings = flags_mappings();
    mappings.reads.insert(
        aarch64::gpreg(5),
        Mapping::require(spec_var("rn".to_string())),
    );
    mappings.reads.insert(
        aarch64::gpreg(6),
        Mapping::require(spec_var("rm".to_string())),
    );
    for flag in &["n", "z", "c", "v"] {
        mappings.reads.insert(
            Target::Var(flag.to_string()),
            MappingBuilder::var("nzcv")
                .field(&flag.to_uppercase())
                .build(),
        );
    }

    Ok(SpecConfig {
        term: "MInst.CCmp".to_string(),
        args: ["size", "rn", "rm", "nzcv", "cond"]
            .map(String::from)
            .to_vec(),

        cases: Cases::Match(Match {
            on: spec_var("size".to_string()),
            arms: sizes
                .iter()
                .rev()
                .map(|size| {
                    Ok(Arm {
                        variant: format!("{size:?}"),
                        args: Vec::new(),
                        body: Cases::Match(Match {
                            on: spec_var("cond".to_string()),
                            arms: conds()
                                .iter()
                                .rev()
                                .map(|cond| {
                                    let template = ccmp_template(*size, xreg(5), xreg(6), *cond)?;
                                    Ok(Arm {
                                        variant: format!("{cond:?}"),
                                        args: Vec::new(),
                                        body: Cases::Instruction(InstConfig {
                                            opcodes: Opcodes::Template(template),
                                            scope: scope.clone(),
                                            mappings: mappings.clone(),
                                        }),
                                    })
                                })
                                .collect::<Result<_>>()?,
                        }),
                    })
                })
                .collect::<Result<_>>()?,
        }),
    })
}

fn ccmp_template(size: OperandSize, rn: Reg, rm: Reg, cond: Cond) -> Result<Bits> {
    // Assemble a base instruction with a placeholder for the NZCV field.
    let placeholder = NZCV::new(false, false, false, false);
    let base = Inst::CCmp {
        size,
        rn,
        rm,
        nzcv: placeholder,
        cond,
    };
    let opcode = aarch64::opcode(&base);
    let bits = Bits::from_u32(opcode);

    // Splice in symbolic immediate fields.
    let nzcv = Bits {
        segments: vec![
            Segment::Symbolic("v".to_string(), 1),
            Segment::Symbolic("c".to_string(), 1),
            Segment::Symbolic("z".to_string(), 1),
            Segment::Symbolic("n".to_string(), 1),
        ],
    };
    let template = Bits::splice(&bits, &nzcv, 0)?;

    // Verify template against the assembler.
    verify_opcode_template(&template, |assignment: &HashMap<String, u32>| {
        let nzcv = NZCV::new(
            assignment["n"] != 0,
            assignment["z"] != 0,
            assignment["c"] != 0,
            assignment["v"] != 0,
        );
        Ok(Inst::CCmp {
            size,
            rn,
            rm,
            nzcv,
            cond,
        })
    })?;

    Ok(template)
}

fn define_ccmp_imm() -> Result<SpecConfig> {
    // OperandSize
    let sizes = [OperandSize::Size32, OperandSize::Size64];

    // Execution scope: define opcode template fields.
    let mut scope = aarch64::state();
    let imm_var = Target::Var("imm".to_string());
    scope.global(imm_var.clone());
    for flag in &["n", "z", "c", "v"] {
        scope.global(Target::Var(flag.to_string()));
    }

    // Flags and register mappings.
    let mut mappings = flags_mappings();
    mappings.reads.insert(
        aarch64::gpreg(5),
        Mapping::require(spec_var("rn".to_string())),
    );
    mappings
        .reads
        .insert(imm_var, Mapping::require(spec_var("imm".to_string())));
    for flag in &["n", "z", "c", "v"] {
        mappings.reads.insert(
            Target::Var(flag.to_string()),
            MappingBuilder::var("nzcv")
                .field(&flag.to_uppercase())
                .build(),
        );
    }

    Ok(SpecConfig {
        term: "MInst.CCmpImm".to_string(),
        args: ["size", "rn", "imm", "nzcv", "cond"]
            .map(String::from)
            .to_vec(),

        cases: Cases::Match(Match {
            on: spec_var("size".to_string()),
            arms: sizes
                .iter()
                .rev()
                .map(|size| {
                    Ok(Arm {
                        variant: format!("{size:?}"),
                        args: Vec::new(),
                        body: Cases::Match(Match {
                            on: spec_var("cond".to_string()),
                            arms: conds()
                                .iter()
                                .rev()
                                .map(|cond| {
                                    let template = ccmp_imm_template(*size, xreg(5), *cond)?;
                                    Ok(Arm {
                                        variant: format!("{cond:?}"),
                                        args: Vec::new(),
                                        body: Cases::Instruction(InstConfig {
                                            opcodes: Opcodes::Template(template),
                                            scope: scope.clone(),
                                            mappings: mappings.clone(),
                                        }),
                                    })
                                })
                                .collect::<Result<_>>()?,
                        }),
                    })
                })
                .collect::<Result<_>>()?,
        }),
    })
}

fn ccmp_imm_template(size: OperandSize, rn: Reg, cond: Cond) -> Result<Bits> {
    // Assemble a base instruction with a placeholder for the immediate and NZCV fields.
    let imm_placeholder = UImm5::maybe_from_u8(0).unwrap();
    let nzcv_placeholder = NZCV::new(false, false, false, false);
    let base = Inst::CCmpImm {
        size,
        rn,
        imm: imm_placeholder,
        nzcv: nzcv_placeholder,
        cond,
    };
    let opcode = aarch64::opcode(&base);
    let bits = Bits::from_u32(opcode);

    // Splice in symbolic immediate fields.
    let nzcv = Bits {
        segments: vec![
            Segment::Symbolic("v".to_string(), 1),
            Segment::Symbolic("c".to_string(), 1),
            Segment::Symbolic("z".to_string(), 1),
            Segment::Symbolic("n".to_string(), 1),
        ],
    };
    let template = Bits::splice(&bits, &nzcv, 0)?;
    let imm = Bits {
        segments: vec![Segment::Symbolic("imm".to_string(), 5)],
    };
    let template = Bits::splice(&template, &imm, 16)?;

    // Verify template against the assembler.
    verify_opcode_template(&template, |assignment: &HashMap<String, u32>| {
        let imm = UImm5::maybe_from_u8(assignment["imm"].try_into()?).unwrap();
        let nzcv = NZCV::new(
            assignment["n"] != 0,
            assignment["z"] != 0,
            assignment["c"] != 0,
            assignment["v"] != 0,
        );
        Ok(Inst::CCmpImm {
            size,
            rn,
            imm,
            nzcv,
            cond,
        })
    })?;

    Ok(template)
}

/// All condition codes.
fn conds() -> Vec<Cond> {
    vec![
        Cond::Eq,
        Cond::Ne,
        Cond::Hs,
        Cond::Lo,
        Cond::Mi,
        Cond::Pl,
        Cond::Vs,
        Cond::Vc,
        Cond::Hi,
        Cond::Ls,
        Cond::Ge,
        Cond::Lt,
        Cond::Gt,
        Cond::Le,
    ]
}

// MInst.FpuRRR specification configuration.
fn define_fpu_rrr() -> SpecConfig {
    // FPUOp2
    let fpu_op2s = [
        FPUOp2::Add,
        FPUOp2::Sub,
        FPUOp2::Mul,
        FPUOp2::Div,
        FPUOp2::Min,
        FPUOp2::Max,
    ];

    // ScalarSize
    let sizes = [ScalarSize::Size64, ScalarSize::Size32];

    spec_config! {
        (FpuRRR fpu_op size rd rn rm)
        {
            enumerate   (size, sizes);
            enumerate   (fpu_op, fpu_op2s);
            register    (rd, write, fp, 4);
            register    (rn, read,  fp, 5);
            register    (rm, read,  fp, 6);
            fpcr        ();
            instruction ();
        }
    }
}

// MInst.FpuRR specification configuration.
fn define_fpu_rr() -> SpecConfig {
    // FPUOp1
    let fpu_op1s = [
        FPUOp1::Neg,
        FPUOp1::Abs,
        FPUOp1::Sqrt,
        FPUOp1::Cvt64To32,
        FPUOp1::Cvt32To64,
    ];

    // ScalarSize
    let sizes = [ScalarSize::Size64, ScalarSize::Size32];

    spec_config! {
        (FpuRR fpu_op size rd rn)
        {
            enumerate   (size, sizes);
            enumerate   (fpu_op, fpu_op1s);
            register    (rd, write, fp, 4);
            register    (rn, read,  fp, 5);
            filter      (is_fpu_op1_size_supported(fpu_op, size));
            fpcr        ();
            instruction ();
        }
    }
}

fn is_fpu_op1_size_supported(fpu_op1: FPUOp1, size: ScalarSize) -> bool {
    match fpu_op1 {
        FPUOp1::Cvt64To32 => size == ScalarSize::Size64,
        FPUOp1::Cvt32To64 => size == ScalarSize::Size32,
        _ => true,
    }
}

fn define_fpu_move_imm() -> SpecConfig {
    // ScalarSize
    let sizes = [ScalarSize::Size32, ScalarSize::Size64];

    // Execution scope: define opcode template fields.
    let mut scope = aarch64::state();
    let imm_bits = Target::Var("bits".to_string());
    scope.global(imm_bits.clone());

    // Mappings
    let mut mappings = flags_mappings();
    mappings.writes.insert(
        aarch64::vreg(4),
        Mapping::require(spec_var("rd".to_string())),
    );
    mappings.reads.insert(
        imm_bits.clone(),
        MappingBuilder::var("imm").field("imm").build(),
    );

    SpecConfig {
        term: "MInst.FpuMoveFPImm".to_string(),
        args: ["rd", "imm", "size"].map(String::from).to_vec(),
        cases: Cases::Match(Match {
            on: spec_var("size".to_string()),
            arms: sizes
                .iter()
                .rev()
                .map(|size| Arm {
                    variant: format!("{size:?}"),
                    args: Vec::new(),
                    body: Cases::Instruction({
                        InstConfig {
                            opcodes: Opcodes::Template(
                                fpu_move_imm_template(*size, writable_vreg(4)).unwrap(),
                            ),
                            scope: scope.clone(),
                            mappings: mappings.clone(),
                        }
                    }),
                })
                .collect(),
        }),
    }
}

fn fpu_move_imm_template(size: ScalarSize, rd: Writable<Reg>) -> Result<Bits> {
    // Assemble a base instruction with a placeholder for the imm12 field.
    let placeholder = ASIMDFPModImm { imm: 0, size };
    let base = Inst::FpuMoveFPImm {
        rd,
        imm: placeholder,
        size,
    };
    let opcode = aarch64::opcode(&base);
    let bits = Bits::from_u32(opcode);
    // Splice in symbolic immediate fields.
    let imm = Bits {
        segments: vec![Segment::Symbolic("bits".to_string(), 8)],
    };
    let template = Bits::splice(&bits, &imm, 13)?;

    // Verify template against the assembler.
    verify_opcode_template(&template, |assignment: &HashMap<String, u32>| {
        let bits = assignment.get("bits").unwrap();
        let imm = ASIMDFPModImm {
            imm: (*bits).try_into().unwrap(),
            size,
        };
        Ok(Inst::FpuMoveFPImm { rd, imm, size })
    })?;

    Ok(template)
}

fn define_fpu_cmp() -> SpecConfig {
    // ScalarSize
    let sizes = [ScalarSize::Size64, ScalarSize::Size32];

    spec_config! {
        (FpuCmp size rn rm)
        {
            enumerate   (size, sizes);
            register    (rn, read, fp, 4);
            register    (rm, read,  fp, 5);
            fpcr        ();
            flags       ();
            instruction ();
        }
    }
}

// MInst.MovToFpu specification configuration.
fn define_mov_to_fpu() -> SpecConfig {
    // ScalarSize
    let sizes = [ScalarSize::Size64, ScalarSize::Size32, ScalarSize::Size16];

    spec_config! {
        (MovToFpu rd rn size)
        {
            enumerate   (size, sizes);
            register    (rd, write, vec, 4);
            register    (rn, read,  gp64, 5);
            instruction ();
        }
    }
}

// ;; Conversion: integer -> FP.
// MInst.IntToFpu specification configuration.
fn define_int_to_fpu() -> SpecConfig {
    let ops: [IntToFpuOp; 8] = [
        IntToFpuOp::I64ToF64,
        IntToFpuOp::U64ToF64,
        IntToFpuOp::I64ToF32,
        IntToFpuOp::U64ToF32,
        IntToFpuOp::I32ToF64,
        IntToFpuOp::U32ToF64,
        IntToFpuOp::I32ToF32,
        IntToFpuOp::U32ToF32,
    ];

    spec_config! {
        (IntToFpu op rd rn)
        {
            enumerate   (op, ops);
            register    (rd, write, vec, 4);
            register    (rn, read,  gp, 5);
            fpcr        ();
            flags       ();
            instruction ();
        }
    }
}

// ;; Conversion: integer -> FP.
// MInst.FpuToInt specification configuration.
fn define_fpu_to_int() -> SpecConfig {
    let ops: [FpuToIntOp; 8] = [
        FpuToIntOp::F64ToI64,
        FpuToIntOp::F64ToU64,
        FpuToIntOp::F64ToI32,
        FpuToIntOp::F64ToU32,
        FpuToIntOp::F32ToI64,
        FpuToIntOp::F32ToU64,
        FpuToIntOp::F32ToI32,
        FpuToIntOp::F32ToU32,
    ];

    spec_config! {
        (FpuToInt op rd rn)
        {
            enumerate   (op, ops);
            register    (rd, write, gp, 4);
            register    (rn, read,  vec, 5);
            fpcr        ();
            flags       ();
            instruction ();
        }
    }
}

fn define_fpu_round() -> SpecConfig {
    // FpuRoundMode
    let modes = [
        FpuRoundMode::Nearest64,
        FpuRoundMode::Nearest32,
        FpuRoundMode::Zero64,
        FpuRoundMode::Zero32,
        FpuRoundMode::Plus64,
        FpuRoundMode::Plus32,
        FpuRoundMode::Minus64,
        FpuRoundMode::Minus32,
    ];

    spec_config! {
        (FpuRound op rd rn)
        {
            enumerate   (op, modes);
            register    (rd, write, fp, 4);
            register    (rn, read,  fp, 5);
            fpcr        ();
            instruction ();
        }
    }
}

fn define_fpu_rri() -> SpecConfig {
    let ops = [
        (
            32,
            FPUOpRI::UShr32(FPURightShiftImm {
                amount: 31,
                lane_size_in_bits: 32,
            }),
        ),
        (
            64,
            FPUOpRI::UShr64(FPURightShiftImm {
                amount: 63,
                lane_size_in_bits: 64,
            }),
        ),
    ];
    // FpuRRI
    let mut mappings = Mappings::default();
    mappings
        .writes
        .insert(aarch64::vreg(4), Mapping::require(spec_fp_reg("rd")));
    mappings
        .reads
        .insert(aarch64::vreg(5), Mapping::require(spec_fp_reg("rn")));

    SpecConfig {
        term: "MInst.FpuRRI".to_string(),
        args: ["fpu_op", "rd", "rn"].map(String::from).to_vec(),

        cases: Cases::Cases(
            ops.iter()
                .rev()
                .map(|(size, fpu_op)| Case {
                    conds: vec![spec_eq(
                        spec_field(
                            "lane_size_in_bits".to_string(),
                            spec_var("fpu_op".to_string()),
                        ),
                        spec_const_bit_vector(*size, 8),
                    )],
                    cases: Cases::Instruction(InstConfig {
                        opcodes: Opcodes::Instruction(Inst::FpuRRI {
                            fpu_op: *fpu_op,
                            rd: writable_vreg(4),
                            rn: vreg(5),
                        }),
                        scope: aarch64::state(),
                        mappings: mappings.clone(),
                    }),
                })
                .collect(),
        ),
    }
}

// ;; Variant of FpuRRI that modifies its `rd`
fn define_fpu_rrimod() -> SpecConfig {
    let ops = [
        (
            32,
            FPUOpRIMod::Sli32(FPULeftShiftImm {
                amount: 31,
                lane_size_in_bits: 32,
            }),
        ),
        (
            64,
            FPUOpRIMod::Sli64(FPULeftShiftImm {
                amount: 63,
                lane_size_in_bits: 64,
            }),
        ),
    ];
    // FpuRRIMod
    let mut mappings = Mappings::default();
    mappings
        .writes
        .insert(aarch64::vreg(4), Mapping::require(spec_fp_reg("rd")));
    mappings
        .reads
        .insert(aarch64::vreg(4), Mapping::require(spec_fp_reg("ri")));
    mappings
        .reads
        .insert(aarch64::vreg(5), Mapping::require(spec_fp_reg("rn")));

    SpecConfig {
        term: "MInst.FpuRRIMod".to_string(),
        args: ["fpu_op", "rd", "ri", "rn"].map(String::from).to_vec(),

        cases: Cases::Cases(
            ops.iter()
                .rev()
                .map(|(size, fpu_op)| Case {
                    conds: vec![spec_eq(
                        spec_field(
                            "lane_size_in_bits".to_string(),
                            spec_var("fpu_op".to_string()),
                        ),
                        spec_const_bit_vector(*size, 8),
                    )],
                    cases: Cases::Instruction(InstConfig {
                        opcodes: Opcodes::Instruction(Inst::FpuRRIMod {
                            fpu_op: *fpu_op,
                            rd: writable_vreg(4),
                            ri: vreg(4),
                            rn: vreg(5),
                        }),
                        scope: aarch64::state(),
                        mappings: mappings.clone(),
                    }),
                })
                .collect(),
        ),
    }
}

// MInst.MovFromVec specification configuration.
fn define_mov_from_vec() -> SpecConfig {
    // ScalarSize
    let sizes = [
        ScalarSize::Size8,
        ScalarSize::Size16,
        ScalarSize::Size32,
        ScalarSize::Size64,
    ];

    // MovFromVec
    let mut mappings = Mappings::default();
    mappings.writes.insert(
        aarch64::gpreg(4),
        Mapping::require(spec_var("rd".to_string())),
    );
    mappings.reads.insert(
        aarch64::vreg(5),
        Mapping::require(spec_as_bit_vector_width(spec_var("rn".to_string()), 128)),
    );

    SpecConfig {
        term: "MInst.MovFromVec".to_string(),
        args: ["rd", "rn", "idx", "size"].map(String::from).to_vec(),

        cases: Cases::Match(Match {
            on: spec_var("size".to_string()),
            arms: sizes
                .iter()
                .rev()
                .map(|size| {
                    let lanes = 128 / size.ty().bits();
                    Arm {
                        variant: format!("{size:?}"),
                        args: Vec::new(),
                        body: Cases::Cases(
                            (0..lanes)
                                .map(|idx| Case {
                                    conds: vec![spec_eq(
                                        spec_var("idx".to_string()),
                                        spec_const_bit_vector(idx.into(), 8),
                                    )],
                                    cases: Cases::Instruction(InstConfig {
                                        opcodes: Opcodes::Instruction(Inst::MovFromVec {
                                            rd: writable_xreg(4),
                                            rn: vreg(5),
                                            idx: idx.try_into().unwrap(),
                                            size: *size,
                                        }),
                                        scope: aarch64::state(),
                                        mappings: mappings.clone(),
                                    }),
                                })
                                .collect(),
                        ),
                    }
                })
                .collect(),
        }),
    }
}

// MInst.VecDupImm specification configuration.
//
// Note this specification only handles the 8-bit immediate field of ASIMDMovModImm.
// This is sufficient to handle the limited uses of VecDupImm right now.
//
// TODO: handle all ASIMDMovModImm parameters
fn define_vec_dup_imm() -> Result<SpecConfig> {
    // VectorSize
    let sizes = [VectorSize::Size32x2];

    // Invert
    let inverts = [false, true];

    // Execution scope: define opcode template fields.
    let mut scope = aarch64::state();
    let abc_bits = Target::Var("abc".to_string());
    let defgh_bits = Target::Var("defgh".to_string());
    scope.global(abc_bits.clone());
    scope.global(defgh_bits.clone());

    // Mappings
    let mut mappings = flags_mappings();
    mappings.writes.insert(
        aarch64::vreg(4),
        Mapping::require(spec_var("rd".to_string())),
    );
    let imm = spec_field("imm".to_string(), spec_var("imm".to_string()));
    mappings.reads.insert(
        abc_bits.clone(),
        Mapping::require(spec_extract(7, 5, imm.clone())),
    );
    mappings.reads.insert(
        defgh_bits.clone(),
        Mapping::require(spec_extract(4, 0, imm.clone())),
    );

    Ok(SpecConfig {
        term: "MInst.VecDupImm".to_string(),
        args: ["rd", "imm", "invert", "size"].map(String::from).to_vec(),
        cases: Cases::Match(Match {
            on: spec_var("size".to_string()),
            arms: sizes
                .iter()
                .rev()
                .map(|size| {
                    Ok(Arm {
                        variant: format!("{size:?}"),
                        args: Vec::new(),
                        body: Cases::Cases(
                            inverts
                                .iter()
                                .map(|invert| {
                                    Ok(Case {
                                        conds: vec![spec_eq_bool(
                                            spec_var("invert".to_string()),
                                            *invert,
                                        )],
                                        cases: Cases::Instruction(InstConfig {
                                            opcodes: Opcodes::Template(vec_dup_imm_template(
                                                writable_vreg(4),
                                                *invert,
                                                *size,
                                            )?),
                                            scope: scope.clone(),
                                            mappings: mappings.clone(),
                                        }),
                                    })
                                })
                                .collect::<Result<_>>()?,
                        ),
                    })
                })
                .collect::<Result<_>>()?,
        }),
    })
}

fn vec_dup_imm_template(rd: Writable<Reg>, invert: bool, size: VectorSize) -> Result<Bits> {
    // Assemble a base instruction with a placeholder for the immediate field.
    // TODO: handle all ASIMDMovModImm parameters shift, is_64bit, and shift_ones.
    let placeholder = ASIMDMovModImm {
        imm: 0,
        shift: 0,
        is_64bit: false,
        shift_ones: false,
    };
    let base = Inst::VecDupImm {
        rd,
        imm: placeholder,
        invert,
        size,
    };
    let opcode = aarch64::opcode(&base);
    let bits = Bits::from_u32(opcode);

    // Splice in symbolic immediate fields.
    let abc = Bits {
        segments: vec![Segment::Symbolic("abc".to_string(), 3)],
    };
    let template = Bits::splice(&bits, &abc, 16)?;

    let defgh = Bits {
        segments: vec![Segment::Symbolic("defgh".to_string(), 5)],
    };
    let template = Bits::splice(&template, &defgh, 5)?;

    // Verify template against the assembler.
    verify_opcode_template(&template, |assignment: &HashMap<String, u32>| {
        let abc = assignment.get("abc").unwrap();
        let defgh = assignment.get("defgh").unwrap();
        let bits = (abc << 5) | defgh;

        let imm = ASIMDMovModImm {
            imm: bits.try_into().unwrap(),
            shift: 0,
            is_64bit: false,
            shift_ones: false,
        };
        Ok(Inst::VecDupImm {
            rd,
            imm,
            invert,
            size,
        })
    })?;

    Ok(template)
}

// MInst.VecRRR specification configuration.
fn define_vec_rrr() -> SpecConfig {
    // VecALUOp
    let vec_alu_ops = [VecALUOp::Addp];

    // VectorSize
    let sizes = [VectorSize::Size8x16, VectorSize::Size8x8];

    spec_config! {
        (VecRRR alu_op rd rn rm size)
        {
            enumerate   (size, sizes);
            enumerate   (alu_op, vec_alu_ops);
            register    (rd, write, vec, 4);
            register    (rn, read,  vec, 5);
            register    (rm, read,  vec, 6);
            instruction ();
        }
    }
}

// MInst.VecMisc specification configuration.
fn define_vec_misc() -> SpecConfig {
    // VecMisc2
    let ops = [VecMisc2::Cnt];

    // VectorSize
    let sizes = [VectorSize::Size8x16, VectorSize::Size8x8];

    spec_config! {
        (VecMisc op rd rn size)
        {
            enumerate   (size, sizes);
            enumerate   (op, ops);
            register    (rd, write, vec, 4);
            register    (rn, read,  vec, 5);
            instruction ();
        }
    }
}

// MInst.VecLanes specification configuration.
fn define_vec_lanes() -> SpecConfig {
    // VecLanesOp
    let vec_lanes_ops = [VecLanesOp::Uminv, VecLanesOp::Addv];

    // VectorSize
    let sizes = [
        VectorSize::Size8x8,
        VectorSize::Size8x16,
        VectorSize::Size16x4,
        VectorSize::Size16x8,
        VectorSize::Size32x4,
    ];

    // VecLanes
    let mut mappings = Mappings::default();
    mappings.writes.insert(
        aarch64::vreg(4),
        Mapping::require(spec_var("rd".to_string())),
    );
    mappings.reads.insert(
        aarch64::vreg(5),
        Mapping::require(spec_var("rn".to_string())),
    );

    SpecConfig {
        term: "MInst.VecLanes".to_string(),
        args: ["op", "rd", "rn", "size"].map(String::from).to_vec(),

        cases: Cases::Match(Match {
            on: spec_var("size".to_string()),
            arms: sizes
                .iter()
                .rev()
                .map(|size| Arm {
                    variant: format!("{size:?}"),
                    args: Vec::new(),
                    body: Cases::Match(Match {
                        on: spec_var("op".to_string()),
                        arms: vec_lanes_ops
                            .iter()
                            .map(|op| Arm {
                                variant: format!("{op:?}"),
                                args: Vec::new(),
                                body: Cases::Instruction(InstConfig {
                                    opcodes: Opcodes::Instruction(Inst::VecLanes {
                                        op: *op,
                                        rd: writable_vreg(4),
                                        rn: vreg(5),
                                        size: *size,
                                    }),
                                    scope: aarch64::state(),
                                    mappings: mappings.clone(),
                                }),
                            })
                            .collect(),
                    }),
                })
                .collect(),
        }),
    }
}
