use crate::isa::unwind::input::UnwindInfo;
use crate::isa::x64::inst::{
    args::{AluRmiROpcode, Amode, RegMemImm, SyntheticAmode},
    regs, Inst,
};
use crate::machinst::{UnwindInfoContext, UnwindInfoGenerator};
use crate::result::CodegenResult;
use alloc::vec::Vec;
use regalloc::Reg;

#[cfg(feature = "unwind")]
pub(crate) mod systemv;

pub struct X64UnwindInfo;

impl UnwindInfoGenerator<Inst> for X64UnwindInfo {
    fn create_unwind_info(
        context: UnwindInfoContext<Inst>,
    ) -> CodegenResult<Option<UnwindInfo<Reg>>> {
        use crate::isa::unwind::input::{self, UnwindCode};
        let mut codes = Vec::new();
        const WORD_SIZE: u8 = 8;

        for i in context.prologue.clone() {
            let i = i as usize;
            let inst = &context.insts[i];
            let offset = context.insts_layout[i];

            match inst {
                Inst::Push64 {
                    src: RegMemImm::Reg { reg },
                } => {
                    codes.push((
                        offset,
                        UnwindCode::StackAlloc {
                            size: WORD_SIZE.into(),
                        },
                    ));
                    codes.push((
                        offset,
                        UnwindCode::SaveRegister {
                            reg: *reg,
                            stack_offset: 0,
                        },
                    ));
                }
                Inst::MovRR { src, dst, .. } => {
                    if *src == regs::rsp() {
                        codes.push((offset, UnwindCode::SetFramePointer { reg: dst.to_reg() }));
                    }
                }
                Inst::AluRmiR {
                    is_64: true,
                    op: AluRmiROpcode::Sub,
                    src: RegMemImm::Imm { simm32 },
                    dst,
                    ..
                } if dst.to_reg() == regs::rsp() => {
                    let imm = *simm32;
                    codes.push((offset, UnwindCode::StackAlloc { size: imm }));
                }
                Inst::MovRM {
                    src,
                    dst: SyntheticAmode::Real(Amode::ImmReg { simm32, base, .. }),
                    ..
                } if *base == regs::rsp() => {
                    // `mov reg, imm(rsp)`
                    let imm = *simm32;
                    codes.push((
                        offset,
                        UnwindCode::SaveRegister {
                            reg: *src,
                            stack_offset: imm,
                        },
                    ));
                }
                Inst::AluRmiR {
                    is_64: true,
                    op: AluRmiROpcode::Add,
                    src: RegMemImm::Imm { simm32 },
                    dst,
                    ..
                } if dst.to_reg() == regs::rsp() => {
                    let imm = *simm32;
                    codes.push((offset, UnwindCode::StackDealloc { size: imm }));
                }
                _ => {}
            }
        }

        let last_epilogue_end = context.len;
        let epilogues_unwind_codes = context
            .epilogues
            .iter()
            .map(|epilogue| {
                // TODO add logic to process epilogue instruction instead of
                // returning empty array.
                let end = epilogue.end as usize - 1;
                let end_offset = context.insts_layout[end];
                if end_offset == last_epilogue_end {
                    // Do not remember/restore for very last epilogue.
                    return vec![];
                }

                let start = epilogue.start as usize;
                let offset = context.insts_layout[start];
                vec![
                    (offset, UnwindCode::RememberState),
                    // TODO epilogue instructions
                    (end_offset, UnwindCode::RestoreState),
                ]
            })
            .collect();

        let prologue_size = context.insts_layout[context.prologue.end as usize];
        Ok(Some(input::UnwindInfo {
            prologue_size,
            prologue_unwind_codes: codes,
            epilogues_unwind_codes,
            function_size: context.len,
            word_size: WORD_SIZE,
            initial_sp_offset: WORD_SIZE,
        }))
    }
}
