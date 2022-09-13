//! Lower a single Cranelift instruction into vcode.

use crate::ir::Inst as IRInst;

use crate::ir::Opcode;
use crate::isa::riscv64::settings as aarch64_settings;
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::settings::Flags;
use crate::CodegenResult;

use crate::isa::riscv64::inst::*;
use target_lexicon::Triple;

/// Actually codegen an instruction's results into registers.
pub(crate) fn lower_insn_to_regs(
    ctx: &mut Lower<Inst>,
    insn: IRInst,
    triple: &Triple,
    flags: &Flags,
    isa_flags: &aarch64_settings::Flags,
) -> CodegenResult<()> {
    let op = ctx.data(insn).opcode();
    let outputs = insn_outputs(ctx, insn);

    let ty = if outputs.len() > 0 {
        Some(ctx.output_ty(insn, 0))
    } else {
        None
    };

    if let Ok(()) = super::lower::isle::lower(ctx, flags, triple, isa_flags, &outputs, insn) {
        return Ok(());
    }

    let implemented_in_isle = |ctx: &mut Lower<Inst>| -> ! {
        unreachable!(
            "implemented in ISLE: inst = `{}`, type = `{:?}`",
            ctx.dfg().display_inst(insn),
            ty
        );
    };

    match op {
        Opcode::Iconst | Opcode::Bconst | Opcode::Null => implemented_in_isle(ctx),

        Opcode::F32const | Opcode::F64const => unreachable!(
            "Should never see constant ops at top level lowering entry
            point, as constants are rematerialized at use-sites"
        ),

        Opcode::Iadd => implemented_in_isle(ctx),
        Opcode::Isub => implemented_in_isle(ctx),
        Opcode::UaddSat | Opcode::SaddSat | Opcode::UsubSat | Opcode::SsubSat => {
            implemented_in_isle(ctx)
        }

        Opcode::Ineg => implemented_in_isle(ctx),

        Opcode::Imul => implemented_in_isle(ctx),

        Opcode::Umulhi | Opcode::Smulhi => implemented_in_isle(ctx),

        Opcode::Udiv | Opcode::Sdiv | Opcode::Urem | Opcode::Srem => implemented_in_isle(ctx),

        Opcode::Uextend | Opcode::Sextend => implemented_in_isle(ctx),

        Opcode::Bnot => implemented_in_isle(ctx),

        Opcode::Band
        | Opcode::Bor
        | Opcode::Bxor
        | Opcode::BandNot
        | Opcode::BorNot
        | Opcode::BxorNot => implemented_in_isle(ctx),

        Opcode::Ishl | Opcode::Ushr | Opcode::Sshr => implemented_in_isle(ctx),

        Opcode::Rotr | Opcode::Rotl => implemented_in_isle(ctx),

        Opcode::Bitrev | Opcode::Clz | Opcode::Cls | Opcode::Ctz => implemented_in_isle(ctx),

        Opcode::Popcnt => implemented_in_isle(ctx),

        Opcode::Load
        | Opcode::Uload8
        | Opcode::Sload8
        | Opcode::Uload16
        | Opcode::Sload16
        | Opcode::Uload32
        | Opcode::Sload32
        | Opcode::Sload8x8
        | Opcode::Uload8x8
        | Opcode::Sload16x4
        | Opcode::Uload16x4
        | Opcode::Sload32x2
        | Opcode::Uload32x2 => {
            implemented_in_isle(ctx);
        }

        Opcode::Store | Opcode::Istore8 | Opcode::Istore16 | Opcode::Istore32 => {
            implemented_in_isle(ctx);
        }

        Opcode::StackAddr => {
            implemented_in_isle(ctx);
        }

        Opcode::AtomicRmw => {
            implemented_in_isle(ctx);
        }

        Opcode::AtomicCas => {
            implemented_in_isle(ctx);
        }

        Opcode::AtomicLoad => {
            implemented_in_isle(ctx);
        }

        Opcode::AtomicStore => {
            implemented_in_isle(ctx);
        }

        Opcode::Fence => {
            implemented_in_isle(ctx);
        }

        Opcode::StackLoad
        | Opcode::StackStore
        | Opcode::DynamicStackLoad
        | Opcode::DynamicStackStore
        | Opcode::DynamicStackAddr => {
            panic!("Direct stack memory access not supported; should not be used by Wasm");
        }

        Opcode::HeapAddr => {
            panic!("heap_addr should have been removed by legalization!");
        }

        Opcode::TableAddr => {
            panic!("table_addr should have been removed by legalization!");
        }

        Opcode::Nop => {
            // Nothing.
        }

        Opcode::Select => {
            implemented_in_isle(ctx);
        }

        Opcode::Selectif | Opcode::SelectifSpectreGuard => {
            implemented_in_isle(ctx);
        }

        Opcode::Bitselect => {
            implemented_in_isle(ctx);
        }

        Opcode::Vselect => vec_not_implemented(),

        Opcode::Trueif => {
            implemented_in_isle(ctx);
        }

        Opcode::Trueff => {
            implemented_in_isle(ctx);
        }

        Opcode::IsNull | Opcode::IsInvalid => {
            implemented_in_isle(ctx);
        }

        Opcode::Copy => {
            implemented_in_isle(ctx);
        }

        Opcode::Breduce | Opcode::Ireduce => {
            implemented_in_isle(ctx);
        }

        Opcode::Bextend | Opcode::Bmask => {
            implemented_in_isle(ctx);
        }

        Opcode::Bint => {
            implemented_in_isle(ctx);
        }

        Opcode::Bitcast => {
            implemented_in_isle(ctx);
        }

        Opcode::Return => {
            implemented_in_isle(ctx);
        }

        Opcode::Ifcmp | Opcode::Ffcmp => {
            panic!("Should never reach ifcmp as isel root!");
        }

        Opcode::Icmp => {
            implemented_in_isle(ctx);
        }

        Opcode::Fcmp => {
            implemented_in_isle(ctx);
        }

        Opcode::Debugtrap => {
            implemented_in_isle(ctx);
        }

        Opcode::Trap | Opcode::ResumableTrap => {
            implemented_in_isle(ctx);
        }

        Opcode::Trapif => {
            implemented_in_isle(ctx);
        }

        Opcode::Trapff => {
            implemented_in_isle(ctx);
        }

        Opcode::Trapz | Opcode::Trapnz | Opcode::ResumableTrapnz => {
            panic!("trapz / trapnz / resumable_trapnz should have been removed by legalization!");
        }

        Opcode::FuncAddr => {
            implemented_in_isle(ctx);
        }

        Opcode::GlobalValue => {
            panic!("global_value should have been removed by legalization!");
        }

        Opcode::SymbolValue => {
            implemented_in_isle(ctx);
        }

        Opcode::Call | Opcode::CallIndirect => {
            implemented_in_isle(ctx);
        }

        Opcode::GetPinnedReg => pinned_register_not_used(),

        Opcode::SetPinnedReg => pinned_register_not_used(),

        Opcode::Jump
        | Opcode::Brz
        | Opcode::Brnz
        | Opcode::BrIcmp
        | Opcode::Brif
        | Opcode::Brff
        | Opcode::BrTable => {
            panic!("Branch opcode reached non-branch lowering logic!");
        }

        Opcode::Vconst => vec_not_implemented(),

        Opcode::RawBitcast => {
            implemented_in_isle(ctx);
        }

        Opcode::Extractlane => vec_not_implemented(),

        Opcode::Insertlane => vec_not_implemented(),

        Opcode::Splat => vec_not_implemented(),

        Opcode::ScalarToVector => vec_not_implemented(),

        Opcode::VanyTrue | Opcode::VallTrue => vec_not_implemented(),

        Opcode::VhighBits => vec_not_implemented(),

        Opcode::Shuffle => vec_not_implemented(),

        Opcode::Swizzle => vec_not_implemented(),

        Opcode::Isplit => {
            implemented_in_isle(ctx);
        }

        Opcode::Iconcat => {
            implemented_in_isle(ctx);
        }

        Opcode::Imax | Opcode::Umax | Opcode::Umin | Opcode::Imin => {
            implemented_in_isle(ctx);
        }

        Opcode::IaddPairwise => vec_not_implemented(),

        Opcode::WideningPairwiseDotProductS => vec_not_implemented(),

        Opcode::Fadd | Opcode::Fsub | Opcode::Fmul | Opcode::Fdiv | Opcode::Fmin | Opcode::Fmax => {
            implemented_in_isle(ctx);
        }

        Opcode::FminPseudo | Opcode::FmaxPseudo => {
            implemented_in_isle(ctx);
        }

        Opcode::Sqrt | Opcode::Fneg | Opcode::Fabs => {
            implemented_in_isle(ctx);
        }
        Opcode::Fpromote | Opcode::Fdemote => {
            implemented_in_isle(ctx);
        }

        Opcode::Ceil | Opcode::Floor | Opcode::Trunc | Opcode::Nearest => {
            implemented_in_isle(ctx);
        }

        Opcode::Fma => {
            implemented_in_isle(ctx);
        }
        Opcode::Fcopysign => {
            implemented_in_isle(ctx);
        }

        Opcode::FcvtToUint | Opcode::FcvtToSint => {
            implemented_in_isle(ctx);
        }
        Opcode::FcvtFromUint | Opcode::FcvtFromSint => {
            implemented_in_isle(ctx);
        }

        Opcode::FcvtToUintSat | Opcode::FcvtToSintSat => {
            implemented_in_isle(ctx);
        }

        Opcode::IaddIfcout => {
            implemented_in_isle(ctx);
        }

        Opcode::IaddImm
        | Opcode::ImulImm
        | Opcode::UdivImm
        | Opcode::SdivImm
        | Opcode::UremImm
        | Opcode::SremImm
        | Opcode::IrsubImm
        | Opcode::IaddCin
        | Opcode::IaddIfcin
        | Opcode::IaddCout
        | Opcode::IaddCarry
        | Opcode::IaddIfcarry
        | Opcode::IsubBin
        | Opcode::IsubIfbin
        | Opcode::IsubBout
        | Opcode::IsubIfbout
        | Opcode::IsubBorrow
        | Opcode::IsubIfborrow
        | Opcode::BandImm
        | Opcode::BorImm
        | Opcode::BxorImm
        | Opcode::RotlImm
        | Opcode::RotrImm
        | Opcode::IshlImm
        | Opcode::UshrImm
        | Opcode::SshrImm
        | Opcode::IcmpImm
        | Opcode::IfcmpImm => {
            panic!(
                "op:{:?} ALU+imm and ALU+carry ops should not appear here!",
                op
            );
        }

        Opcode::Iabs => {
            implemented_in_isle(ctx);
        }
        Opcode::AvgRound => vec_not_implemented(),

        Opcode::Snarrow | Opcode::Unarrow | Opcode::Uunarrow => vec_not_implemented(),

        Opcode::SwidenLow | Opcode::SwidenHigh | Opcode::UwidenLow | Opcode::UwidenHigh => {
            vec_not_implemented()
        }

        Opcode::TlsValue => {}

        Opcode::SqmulRoundSat => vec_not_implemented(),

        Opcode::FcvtLowFromSint => vec_not_implemented(),

        Opcode::FvpromoteLow => vec_not_implemented(),

        Opcode::Fvdemote => vec_not_implemented(),

        Opcode::Vconcat | Opcode::Vsplit => vec_not_implemented(),
        Opcode::ExtractVector => vec_not_implemented(),

        Opcode::GetFramePointer | Opcode::GetStackPointer | Opcode::GetReturnAddress => {
            implemented_in_isle(ctx)
        }
    }
    Ok(())
}

fn vec_not_implemented() -> ! {
    unreachable!()
}

fn pinned_register_not_used() -> ! {
    unreachable!()
}
