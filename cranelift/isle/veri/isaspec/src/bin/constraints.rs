use std::vec;

use anyhow::Result;
use clap::Parser as ClapParser;
use cranelift_codegen::ir::types::I64;
use cranelift_codegen::isa::aarch64::inst::{
    ALUOp, ALUOp3, BitOp, Cond, FPUOp1, FPUOp2, Imm12, ImmLogic, ImmShift, Inst, MoveWideConst,
    MoveWideOp, NZCV, OperandSize, ScalarSize, ShiftOp, ShiftOpAndAmt, ShiftOpShiftImm, VecALUOp,
    VecMisc2, VectorSize, vreg, writable_vreg, writable_xreg, xreg,
};
use cranelift_isle::printer;
use cranelift_isle_veri_aslp::ast::Block;
use cranelift_isle_veri_aslp::client::Client;
use tracing::debug;

use cranelift_isle_veri_isaspec::aarch64;
use cranelift_isle_veri_isaspec::constraints::Translator;
use cranelift_isle_veri_isaspec::semantics::inst_semantics;

#[derive(ClapParser)]
#[command(version, about)]
struct Args {
    /// Server URL
    #[arg(long = "server", required = true)]
    server: String,

    /// Print debugging output (repeat for more detail)
    #[arg(short = 'd', long = "debug", action = clap::ArgAction::Count)]
    debug_level: u8,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Setup tracing output.
    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .with_level(true)
        .with_target(false)
        .with_max_level(match args.debug_level {
            0 => tracing::Level::WARN,
            1 => tracing::Level::INFO,
            2 => tracing::Level::DEBUG,
            _ => tracing::Level::TRACE,
        })
        .init();

    // ASLp client.
    let client = Client::new(args.server)?;

    // Conversion.
    let insts = define_insts();
    for inst in &insts {
        println!("-------------------------------------");
        let opcode = aarch64::opcode(inst);
        let asm = aarch64::assembly(inst);
        println!("inst = {inst:#?}");
        println!("opcode = {opcode:08x}");
        println!("asm = {asm}");
        println!("----");
        let block = inst_semantics(inst, &client)?;
        convert_block(&block)?;
        println!("-------------------------------------");
    }

    Ok(())
}

// Define instructions to test.
fn define_insts() -> Vec<Inst> {
    let mut insts = Vec::new();

    // OperandSize
    let sizes = [OperandSize::Size32, OperandSize::Size64];

    // AluRRR
    let alu_ops = vec![
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
        ALUOp::Adc,
        ALUOp::Sbc,
        ALUOp::AdcS,
        ALUOp::SbcS,
        ALUOp::Lsr,
        ALUOp::Asr,
        ALUOp::Lsl,
        ALUOp::Extr,
        ALUOp::SDiv,
        ALUOp::UDiv,
    ];
    for alu_op in alu_ops {
        insts.push(Inst::AluRRR {
            alu_op,
            size: OperandSize::Size64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        });
    }

    // AluRRImm12
    let alu_ops_imm12 = [ALUOp::Add, ALUOp::Sub, ALUOp::AddS, ALUOp::SubS];
    let imm12_vals = [0x000123u64, 0x123000u64];
    for alu_op in alu_ops_imm12 {
        for imm12_val in imm12_vals {
            let imm12 = Imm12::maybe_from_u64(imm12_val).unwrap();
            insts.push(Inst::AluRRImm12 {
                alu_op,
                size: OperandSize::Size64,
                rd: writable_xreg(4),
                rn: xreg(5),
                imm12,
            });
        }
    }

    // AluRRImmLogic
    let alu_ops_imml = [ALUOp::And, ALUOp::EorNot];
    let imml_vals = [0xf003fffff003ffffu64, 0xffffffffff000000u64];
    for alu_op in alu_ops_imml {
        for imml_val in imml_vals {
            let imml = ImmLogic::maybe_from_u64(imml_val, I64).unwrap();
            insts.push(Inst::AluRRImmLogic {
                alu_op,
                size: OperandSize::Size64,
                rd: writable_xreg(4),
                rn: xreg(5),
                imml,
            });
        }
    }

    // AluRRImmShift
    let alu_ops_immshift = [ALUOp::Lsr, ALUOp::Lsl];
    let immshift_vals = [13u64, 62];
    for alu_op in alu_ops_immshift {
        for immshift_val in immshift_vals {
            let immshift = ImmShift::maybe_from_u64(immshift_val).unwrap();
            insts.push(Inst::AluRRImmShift {
                alu_op,
                size: OperandSize::Size64,
                rd: writable_xreg(4),
                rn: xreg(5),
                immshift,
            });
        }
    }

    // AluRRRShift
    let alu_ops_rrr_shift = [ALUOp::Add, ALUOp::And];
    let shiftops = [ShiftOp::LSL, ShiftOp::ASR];
    let amts = [13u64, 63];
    for alu_op in alu_ops_rrr_shift {
        for shiftop in shiftops {
            for amt in amts {
                let shiftop =
                    ShiftOpAndAmt::new(shiftop, ShiftOpShiftImm::maybe_from_shift(amt).unwrap());
                insts.push(Inst::AluRRRShift {
                    alu_op,
                    size: OperandSize::Size64,
                    rd: writable_xreg(4),
                    rn: xreg(5),
                    rm: xreg(6),
                    shiftop,
                });
            }
        }
    }

    // AluRRRR
    let alu_ops = vec![ALUOp3::MAdd, ALUOp3::MSub, ALUOp3::UMAddL, ALUOp3::SMAddL];
    for alu_op in alu_ops {
        insts.push(Inst::AluRRRR {
            alu_op,
            size: OperandSize::Size32,
            rd: writable_xreg(4),
            rn: xreg(1),
            rm: xreg(2),
            ra: xreg(3),
        });
    }

    // BitRR
    let ops = vec![
        BitOp::RBit,
        BitOp::Clz,
        BitOp::Cls,
        BitOp::Rev16,
        BitOp::Rev32,
        BitOp::Rev64,
    ];
    for op in ops {
        insts.push(Inst::BitRR {
            op,
            size: OperandSize::Size64,
            rd: writable_xreg(2),
            rn: xreg(1),
        });
    }

    // MovWide
    let mov_wide_ops = [MoveWideOp::MovN, MoveWideOp::MovZ];
    let values = [0x00001234u64, 0x12340000u64];
    for mov_wide_op in mov_wide_ops {
        for size in sizes {
            for value in values {
                insts.push(Inst::MovWide {
                    op: mov_wide_op,
                    rd: writable_xreg(4),
                    imm: MoveWideConst::maybe_from_u64(value).unwrap(),
                    size,
                });
            }
        }
    }

    // CSel
    let conds = vec![
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
        Cond::Al,
        Cond::Nv,
    ];
    for cond in conds.clone() {
        insts.push(Inst::CSel {
            rd: writable_xreg(3),
            cond,
            rn: xreg(1),
            rm: xreg(2),
        });
    }

    // CSNeg
    for cond in conds.clone() {
        insts.push(Inst::CSNeg {
            rd: writable_xreg(3),
            cond,
            rn: xreg(1),
            rm: xreg(2),
        });
    }

    // CCmp
    for cond in conds.clone() {
        insts.push(Inst::CCmp {
            size: OperandSize::Size64,
            rn: xreg(1),
            rm: xreg(2),
            nzcv: NZCV::new(true, false, true, false),
            cond,
        });
    }

    // FpuCmp
    insts.push(Inst::FpuCmp {
        size: ScalarSize::Size64,
        rn: vreg(1),
        rm: vreg(2),
    });

    // FpuRR
    let fpu_op1s = [FPUOp1::Neg];
    for fpu_op1 in fpu_op1s {
        insts.push(Inst::FpuRR {
            fpu_op: fpu_op1,
            size: ScalarSize::Size64,
            rd: writable_vreg(1),
            rn: vreg(2),
        });
    }

    // FpuRRR
    let fpu_op2s = [FPUOp2::Add, FPUOp2::Sub];
    for fpu_op2 in fpu_op2s {
        insts.push(Inst::FpuRRR {
            fpu_op: fpu_op2,
            size: ScalarSize::Size64,
            rd: writable_vreg(1),
            rn: vreg(2),
            rm: vreg(3),
        });
    }

    // VecRRR
    let alu_ops = vec![
        VecALUOp::Cmeq,
        VecALUOp::Cmge,
        VecALUOp::Cmgt,
        VecALUOp::Cmhs,
        VecALUOp::Cmhi,
        VecALUOp::And,
        VecALUOp::Bic,
        VecALUOp::Orr,
        VecALUOp::Umaxp,
        VecALUOp::Add,
        VecALUOp::Sub,
        VecALUOp::Mul,
        VecALUOp::Sshl,
        VecALUOp::Ushl,
        VecALUOp::Umin,
        VecALUOp::Smin,
        VecALUOp::Umax,
        VecALUOp::Smax,
        VecALUOp::Urhadd,
        VecALUOp::Addp,
        VecALUOp::Zip1,
        VecALUOp::Zip2,
        VecALUOp::Uzp1,
        VecALUOp::Uzp2,
        VecALUOp::Trn1,
        VecALUOp::Trn2,
        // TODO: 128-bit bitvector literal
        // VecALUOp::Eor,
        // TODO: boolean literals
        // VecALUOp::Sqadd,
        // VecALUOp::Uqadd,
        // VecALUOp::Sqsub,
        // VecALUOp::Uqsub,
        // VecALUOp::Sqrdmulh,
        // TODO: floating point.
        // VecALUOp::Fcmeq,
        // VecALUOp::Fcmgt,
        // VecALUOp::Fcmge,
        // VecALUOp::Fadd,
        // VecALUOp::Fsub,
        // VecALUOp::Fdiv,
        // VecALUOp::Fmax,
        // VecALUOp::Fmin,
        // VecALUOp::Fmul,
    ];
    for alu_op in alu_ops {
        insts.push(Inst::VecRRR {
            alu_op,
            rd: writable_vreg(3),
            rn: vreg(1),
            rm: vreg(2),
            size: VectorSize::Size32x4,
        });
    }

    // VecMisc
    let vec_misc2s = vec![VecMisc2::Cnt];
    for vec_misc2 in vec_misc2s {
        insts.push(Inst::VecMisc {
            op: vec_misc2,
            rd: writable_vreg(3),
            rn: vreg(1),
            size: VectorSize::Size8x8,
        });
    }

    insts
}

// Convert a semantics block and print the result.
fn convert_block(block: &Block) -> Result<()> {
    // Translation.
    let mut translator = Translator::new(aarch64::state(), "v".to_string());
    translator.translate(block)?;

    // Report.
    let global = translator.global();
    debug!("scope: {global:#?}");

    let init = global.init();
    let bindings = global.bindings();

    for r in global.reads() {
        println!("read:\t{r}\t{}", init[r]);
    }

    for w in global.writes() {
        println!(
            "write:\t{w}\t{}",
            bindings[w].as_var().expect("binding should be variable")
        );
    }

    println!();

    for constraint in global.constraints() {
        printer::dump(constraint).unwrap();
        println!();
    }

    Ok(())
}
