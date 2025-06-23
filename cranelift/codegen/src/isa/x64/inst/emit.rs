use crate::ir::KnownSymbol;
use crate::ir::immediates::{Ieee32, Ieee64};
use crate::isa::x64::encoding::evex::{EvexInstruction, EvexVectorLength, RegisterOrAmode};
use crate::isa::x64::encoding::rex::{
    LegacyPrefixes, OpcodeMap, RexFlags, emit_std_enc_enc, emit_std_enc_mem, emit_std_reg_mem,
    emit_std_reg_reg, int_reg_enc, reg_enc,
};
use crate::isa::x64::encoding::vex::{VexInstruction, VexVectorLength};
use crate::isa::x64::external::{AsmInst, CraneliftRegisters, PairedGpr};
use crate::isa::x64::inst::args::*;
use crate::isa::x64::inst::*;
use crate::isa::x64::lower::isle::generated_code::{Atomic128RmwSeqOp, AtomicRmwSeqOp};
use cranelift_assembler_x64 as asm;

/// A small helper to generate a signed conversion instruction.
fn emit_signed_cvt(
    sink: &mut MachBuffer<Inst>,
    info: &EmitInfo,
    state: &mut EmitState,
    src: Reg,
    dst: Writable<Reg>,
    to_f64: bool,
) {
    assert!(src.is_real());
    assert!(dst.to_reg().is_real());

    // Handle an unsigned int, which is the "easy" case: a signed conversion
    // will do the right thing.
    let dst = WritableXmm::from_writable_reg(dst).unwrap();
    if to_f64 {
        asm::inst::cvtsi2sdq_a::new(dst, src).emit(sink, info, state);
    } else {
        asm::inst::cvtsi2ssq_a::new(dst, src).emit(sink, info, state);
    }
}

/// Emits a one way conditional jump if CC is set (true).
fn one_way_jmp(sink: &mut MachBuffer<Inst>, cc: CC, label: MachLabel) {
    let cond_start = sink.cur_offset();
    let cond_disp_off = cond_start + 2;
    sink.use_label_at_offset(cond_disp_off, label, LabelUse::JmpRel32);
    sink.put1(0x0F);
    sink.put1(0x80 + cc.get_enc());
    sink.put4(0x0);
}

/// Emits a relocation, attaching the current source location as well.
fn emit_reloc(sink: &mut MachBuffer<Inst>, kind: Reloc, name: &ExternalName, addend: Addend) {
    sink.add_reloc(kind, name, addend);
}

/// The top-level emit function.
///
/// Important!  Do not add improved (shortened) encoding cases to existing
/// instructions without also adding tests for those improved encodings.  That
/// is a dangerous game that leads to hard-to-track-down errors in the emitted
/// code.
///
/// For all instructions, make sure to have test coverage for all of the
/// following situations.  Do this by creating the cross product resulting from
/// applying the following rules to each operand:
///
/// (1) for any insn that mentions a register: one test using a register from
///     the group [rax, rcx, rdx, rbx, rsp, rbp, rsi, rdi] and a second one
///     using a register from the group [r8, r9, r10, r11, r12, r13, r14, r15].
///     This helps detect incorrect REX prefix construction.
///
/// (2) for any insn that mentions a byte register: one test for each of the
///     four encoding groups [al, cl, dl, bl], [spl, bpl, sil, dil],
///     [r8b .. r11b] and [r12b .. r15b].  This checks that
///     apparently-redundant REX prefixes are retained when required.
///
/// (3) for any insn that contains an immediate field, check the following
///     cases: field is zero, field is in simm8 range (-128 .. 127), field is
///     in simm32 range (-0x8000_0000 .. 0x7FFF_FFFF).  This is because some
///     instructions that require a 32-bit immediate have a short-form encoding
///     when the imm is in simm8 range.
///
/// Rules (1), (2) and (3) don't apply for registers within address expressions
/// (`Addr`s).  Those are already pretty well tested, and the registers in them
/// don't have any effect on the containing instruction (apart from possibly
/// require REX prefix bits).
///
/// When choosing registers for a test, avoid using registers with the same
/// offset within a given group.  For example, don't use rax and r8, since they
/// both have the lowest 3 bits as 000, and so the test won't detect errors
/// where those 3-bit register sub-fields are confused by the emitter.  Instead
/// use (eg) rax (lo3 = 000) and r9 (lo3 = 001).  Similarly, don't use (eg) cl
/// and bpl since they have the same offset in their group; use instead (eg) cl
/// and sil.
///
/// For all instructions, also add a test that uses only low-half registers
/// (rax .. rdi, xmm0 .. xmm7) etc, so as to check that any redundant REX
/// prefixes are correctly omitted.  This low-half restriction must apply to
/// _all_ registers in the insn, even those in address expressions.
///
/// Following these rules creates large numbers of test cases, but it's the
/// only way to make the emitter reliable.
///
/// Known possible improvements:
///
/// * there's a shorter encoding for shl/shr/sar by a 1-bit immediate.  (Do we
///   care?)
pub(crate) fn emit(
    inst: &Inst,
    sink: &mut MachBuffer<Inst>,
    info: &EmitInfo,
    state: &mut EmitState,
) {
    let matches_isa_flags = |iset_requirement: &InstructionSet| -> bool {
        match iset_requirement {
            // Cranelift assumes SSE2 at least.
            InstructionSet::SSE | InstructionSet::SSE2 => true,
            InstructionSet::CMPXCHG16b => info.isa_flags.use_cmpxchg16b(),
            InstructionSet::SSE3 => info.isa_flags.use_sse3(),
            InstructionSet::SSSE3 => info.isa_flags.use_ssse3(),
            InstructionSet::SSE41 => info.isa_flags.use_sse41(),
            InstructionSet::SSE42 => info.isa_flags.use_sse42(),
            InstructionSet::Popcnt => info.isa_flags.use_popcnt(),
            InstructionSet::Lzcnt => info.isa_flags.use_lzcnt(),
            InstructionSet::BMI1 => info.isa_flags.use_bmi1(),
            InstructionSet::BMI2 => info.isa_flags.has_bmi2(),
            InstructionSet::FMA => info.isa_flags.has_fma(),
            InstructionSet::AVX => info.isa_flags.has_avx(),
            InstructionSet::AVX2 => info.isa_flags.has_avx2(),
            InstructionSet::AVX512BITALG => info.isa_flags.has_avx512bitalg(),
            InstructionSet::AVX512DQ => info.isa_flags.has_avx512dq(),
            InstructionSet::AVX512F => info.isa_flags.has_avx512f(),
            InstructionSet::AVX512VBMI => info.isa_flags.has_avx512vbmi(),
            InstructionSet::AVX512VL => info.isa_flags.has_avx512vl(),
        }
    };

    // Certain instructions may be present in more than one ISA feature set; we must at least match
    // one of them in the target CPU.
    let isa_requirements = inst.available_in_any_isa();
    if !isa_requirements.is_empty() && !isa_requirements.iter().all(matches_isa_flags) {
        panic!(
            "Cannot emit inst '{inst:?}' for target; failed to match ISA requirements: {isa_requirements:?}"
        )
    }
    match inst {
        Inst::CheckedSRemSeq { divisor, .. } | Inst::CheckedSRemSeq8 { divisor, .. } => {
            // Validate that the register constraints of the dividend and the
            // destination are all as expected.
            let (dst, size) = match inst {
                Inst::CheckedSRemSeq {
                    dividend_lo,
                    dividend_hi,
                    dst_quotient,
                    dst_remainder,
                    size,
                    ..
                } => {
                    let dividend_lo = dividend_lo.to_reg();
                    let dividend_hi = dividend_hi.to_reg();
                    let dst_quotient = dst_quotient.to_reg().to_reg();
                    let dst_remainder = dst_remainder.to_reg().to_reg();
                    debug_assert_eq!(dividend_lo, regs::rax());
                    debug_assert_eq!(dividend_hi, regs::rdx());
                    debug_assert_eq!(dst_quotient, regs::rax());
                    debug_assert_eq!(dst_remainder, regs::rdx());
                    (regs::rdx(), *size)
                }
                Inst::CheckedSRemSeq8 { dividend, dst, .. } => {
                    let dividend = dividend.to_reg();
                    let dst = dst.to_reg().to_reg();
                    debug_assert_eq!(dividend, regs::rax());
                    debug_assert_eq!(dst, regs::rax());
                    (regs::rax(), OperandSize::Size8)
                }
                _ => unreachable!(),
            };

            // Generates the following code sequence:
            //
            // cmp -1 %divisor
            // jnz $do_op
            //
            // ;; for srem, result is 0
            // mov #0, %dst
            // j $done
            //
            // $do_op:
            // idiv %divisor
            //
            // $done:

            let do_op = sink.get_label();
            let done_label = sink.get_label();

            // Check if the divisor is -1, and if it isn't then immediately
            // go to the `idiv`.
            let inst = Inst::cmp_mi_sxb(size, *divisor, -1);
            inst.emit(sink, info, state);
            one_way_jmp(sink, CC::NZ, do_op);

            // ... otherwise the divisor is -1 and the result is always 0. This
            // is written to the destination register which will be %rax for
            // 8-bit srem and %rdx otherwise.
            //
            // Note that for 16-to-64-bit srem operations this leaves the
            // second destination, %rax, unchanged. This isn't semantically
            // correct if a lowering actually tries to use the `dst_quotient`
            // output but for srem only the `dst_remainder` output is used for
            // now.
            let inst = Inst::imm(OperandSize::Size64, 0, Writable::from_reg(dst));
            inst.emit(sink, info, state);
            let inst = Inst::jmp_known(done_label);
            inst.emit(sink, info, state);

            // Here the `idiv` is executed, which is different depending on the
            // size
            sink.bind_label(do_op, state.ctrl_plane_mut());
            let rax = Gpr::RAX;
            let rdx = Gpr::RDX;
            let writable_rax = Writable::from_reg(rax);
            let writable_rdx = Writable::from_reg(rdx);
            let inst: AsmInst = match size {
                OperandSize::Size8 => asm::inst::idivb_m::new(
                    PairedGpr::from(writable_rax),
                    *divisor,
                    TrapCode::INTEGER_DIVISION_BY_ZERO,
                )
                .into(),

                OperandSize::Size16 => asm::inst::idivw_m::new(
                    PairedGpr::from(writable_rax),
                    PairedGpr::from(writable_rdx),
                    *divisor,
                    TrapCode::INTEGER_DIVISION_BY_ZERO,
                )
                .into(),

                OperandSize::Size32 => asm::inst::idivl_m::new(
                    PairedGpr::from(writable_rax),
                    PairedGpr::from(writable_rdx),
                    *divisor,
                    TrapCode::INTEGER_DIVISION_BY_ZERO,
                )
                .into(),

                OperandSize::Size64 => asm::inst::idivq_m::new(
                    PairedGpr::from(writable_rax),
                    PairedGpr::from(writable_rdx),
                    *divisor,
                    TrapCode::INTEGER_DIVISION_BY_ZERO,
                )
                .into(),
            };
            inst.emit(sink, info, state);

            sink.bind_label(done_label, state.ctrl_plane_mut());
        }

        Inst::MovFromPReg { src, dst } => {
            let src: Reg = (*src).into();
            debug_assert!([regs::rsp(), regs::rbp(), regs::pinned_reg()].contains(&src));
            asm::inst::movq_mr::new(*dst, Gpr::unwrap_new(src)).emit(sink, info, state);
        }

        Inst::MovToPReg { src, dst } => {
            let dst: Reg = (*dst).into();
            debug_assert!([regs::rsp(), regs::rbp(), regs::pinned_reg()].contains(&dst));
            let dst = WritableGpr::from_writable_reg(Writable::from_reg(dst)).unwrap();
            asm::inst::movq_mr::new(dst, *src).emit(sink, info, state);
        }

        Inst::Setcc { cc, dst } => {
            let dst = dst.to_reg().to_reg();
            let opcode = 0x0f90 + cc.get_enc() as u32;
            let mut rex_flags = RexFlags::clear_w();
            rex_flags.always_emit();
            emit_std_enc_enc(
                sink,
                LegacyPrefixes::None,
                opcode,
                2,
                0,
                reg_enc(dst),
                rex_flags,
            );
        }

        Inst::XmmCmove {
            ty,
            cc,
            consequent,
            alternative,
            dst,
        } => {
            let alternative = *alternative;
            let dst = *dst;
            debug_assert_eq!(alternative, dst.to_reg());
            let consequent = *consequent;

            // Lowering of the Select IR opcode when the input is an fcmp relies on the fact that
            // this doesn't clobber flags. Make sure to not do so here.
            let next = sink.get_label();

            // Jump if cc is *not* set.
            one_way_jmp(sink, cc.invert(), next);
            Inst::gen_move(dst.map(|r| r.to_reg()), consequent.to_reg(), *ty)
                .emit(sink, info, state);

            sink.bind_label(next, state.ctrl_plane_mut());
        }

        Inst::StackProbeLoop {
            tmp,
            frame_size,
            guard_size,
        } => {
            assert!(info.flags.enable_probestack());
            assert!(guard_size.is_power_of_two());

            let tmp = *tmp;

            // Number of probes that we need to perform
            let probe_count = align_to(*frame_size, *guard_size) / guard_size;

            // The inline stack probe loop has 3 phases:
            //
            // We generate the "guard area" register which is essentially the frame_size aligned to
            // guard_size. We copy the stack pointer and subtract the guard area from it. This
            // gets us a register that we can use to compare when looping.
            //
            // After that we emit the loop. Essentially we just adjust the stack pointer one guard_size'd
            // distance at a time and then touch the stack by writing anything to it. We use the previously
            // created "guard area" register to know when to stop looping.
            //
            // When we have touched all the pages that we need, we have to restore the stack pointer
            // to where it was before.
            //
            // Generate the following code:
            //         mov  tmp_reg, rsp
            //         sub  tmp_reg, guard_size * probe_count
            // .loop_start:
            //         sub  rsp, guard_size
            //         mov  [rsp], rsp
            //         cmp  rsp, tmp_reg
            //         jne  .loop_start
            //         add  rsp, guard_size * probe_count

            // Create the guard bound register
            // mov  tmp_reg, rsp
            let inst = Inst::gen_move(tmp, regs::rsp(), types::I64);
            inst.emit(sink, info, state);

            // sub  tmp_reg, GUARD_SIZE * probe_count
            let guard_plus_count = i32::try_from(guard_size * probe_count)
                .expect("`guard_size * probe_count` is too large to fit in a 32-bit immediate");
            Inst::subq_mi(tmp, guard_plus_count).emit(sink, info, state);

            // Emit the main loop!
            let loop_start = sink.get_label();
            sink.bind_label(loop_start, state.ctrl_plane_mut());

            // sub  rsp, GUARD_SIZE
            let rsp = Writable::from_reg(regs::rsp());
            let guard_size_ = i32::try_from(*guard_size)
                .expect("`guard_size` is too large to fit in a 32-bit immediate");
            Inst::subq_mi(rsp, guard_size_).emit(sink, info, state);

            // TODO: `mov [rsp], 0` would be better, but we don't have that instruction
            // Probe the stack! We don't use Inst::gen_store_stack here because we need a predictable
            // instruction size.
            // mov  [rsp], rsp
            asm::inst::movl_mr::new(Amode::imm_reg(0, regs::rsp()), Gpr::RSP)
                .emit(sink, info, state);

            // Compare and jump if we are not done yet
            // cmp  rsp, tmp_reg
            let tmp = Gpr::unwrap_new(tmp.to_reg());
            asm::inst::cmpq_rm::new(tmp, Gpr::RSP).emit(sink, info, state);

            // jne  .loop_start
            // TODO: Encoding the conditional jump as a short jump
            // could save us us 4 bytes here.
            one_way_jmp(sink, CC::NZ, loop_start);

            // The regular prologue code is going to emit a `sub` after this, so we need to
            // reset the stack pointer
            //
            // TODO: It would be better if we could avoid the `add` + `sub` that is generated here
            // and in the stack adj portion of the prologue
            //
            // add rsp, GUARD_SIZE * probe_count
            Inst::addq_mi(rsp, guard_plus_count).emit(sink, info, state);
        }

        Inst::CallKnown { info: call_info } => {
            if let Some(s) = state.take_stack_map() {
                let offset = sink.cur_offset() + 5;
                sink.push_user_stack_map(state, offset, s);
            }

            sink.put1(0xE8);
            // The addend adjusts for the difference between the end of the
            // instruction and the beginning of the immediate field.
            emit_reloc(sink, Reloc::X86CallPCRel4, &call_info.dest, -4);
            sink.put4(0);

            if let Some(try_call) = call_info.try_call_info.as_ref() {
                sink.add_call_site(&try_call.exception_dests);
            } else {
                sink.add_call_site(&[]);
            }

            // Reclaim the outgoing argument area that was released by the
            // callee, to ensure that StackAMode values are always computed from
            // a consistent SP.
            if call_info.callee_pop_size > 0 {
                let rsp = Writable::from_reg(regs::rsp());
                let callee_pop_size = i32::try_from(call_info.callee_pop_size)
                    .expect("`callee_pop_size` is too large to fit in a 32-bit immediate");
                Inst::subq_mi(rsp, callee_pop_size).emit(sink, info, state);
            }

            // Load any stack-carried return values.
            call_info.emit_retval_loads::<X64ABIMachineSpec, _, _>(
                state.frame_layout().stackslots_size,
                |inst| inst.emit(sink, info, state),
                |_space_needed| None,
            );

            // If this is a try-call, jump to the continuation
            // (normal-return) block.
            if let Some(try_call) = call_info.try_call_info.as_ref() {
                let jmp = Inst::JmpKnown {
                    dst: try_call.continuation,
                };
                jmp.emit(sink, info, state);
            }
        }

        Inst::ReturnCallKnown { info: call_info } => {
            emit_return_call_common_sequence(sink, info, state, &call_info);

            // Finally, jump to the callee!
            //
            // Note: this is not `Inst::Jmp { .. }.emit(..)` because we have
            // different metadata in this case: we don't have a label for the
            // target, but rather a function relocation.
            sink.put1(0xE9);
            // The addend adjusts for the difference between the end of the instruction and the
            // beginning of the immediate field.
            emit_reloc(sink, Reloc::X86CallPCRel4, &call_info.dest, -4);
            sink.put4(0);
            sink.add_call_site(&[]);
        }

        Inst::ReturnCallUnknown { info: call_info } => {
            let callee = call_info.dest;

            emit_return_call_common_sequence(sink, info, state, &call_info);

            Inst::JmpUnknown {
                target: RegMem::reg(callee),
            }
            .emit(sink, info, state);
            sink.add_call_site(&[]);
        }

        Inst::CallUnknown {
            info: call_info, ..
        } => {
            let dest = call_info.dest.clone();

            match dest {
                RegMem::Reg { reg } => {
                    let reg_enc = int_reg_enc(reg);
                    emit_std_enc_enc(
                        sink,
                        LegacyPrefixes::None,
                        0xFF,
                        1,
                        2, /*subopcode*/
                        reg_enc,
                        RexFlags::clear_w(),
                    );
                }

                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state.frame_layout(), sink);
                    emit_std_enc_mem(
                        sink,
                        LegacyPrefixes::None,
                        0xFF,
                        1,
                        2, /*subopcode*/
                        addr,
                        RexFlags::clear_w(),
                        0,
                    );
                }
            }

            if let Some(s) = state.take_stack_map() {
                let offset = sink.cur_offset();
                sink.push_user_stack_map(state, offset, s);
            }

            if let Some(try_call) = call_info.try_call_info.as_ref() {
                sink.add_call_site(&try_call.exception_dests);
            } else {
                sink.add_call_site(&[]);
            }

            // Reclaim the outgoing argument area that was released by the callee, to ensure that
            // StackAMode values are always computed from a consistent SP.
            if call_info.callee_pop_size > 0 {
                let rsp = Writable::from_reg(regs::rsp());
                let callee_pop_size = i32::try_from(call_info.callee_pop_size)
                    .expect("`callee_pop_size` is too large to fit in a 32-bit immediate");
                Inst::subq_mi(rsp, callee_pop_size).emit(sink, info, state);
            }

            // Load any stack-carried return values.
            call_info.emit_retval_loads::<X64ABIMachineSpec, _, _>(
                state.frame_layout().stackslots_size,
                |inst| inst.emit(sink, info, state),
                |_space_needed| None,
            );

            if let Some(try_call) = call_info.try_call_info.as_ref() {
                let jmp = Inst::JmpKnown {
                    dst: try_call.continuation,
                };
                jmp.emit(sink, info, state);
            }
        }

        Inst::Args { .. } => {}
        Inst::Rets { .. } => {}

        Inst::StackSwitchBasic {
            store_context_ptr,
            load_context_ptr,
            in_payload0,
            out_payload0,
        } => {
            // Note that we do not emit anything for preserving and restoring
            // ordinary registers here: That's taken care of by regalloc for us,
            // since we marked this instruction as clobbering all registers.
            //
            // Also note that we do nothing about passing the single payload
            // value: We've informed regalloc that it is sent and received via
            // the fixed register given by [stack_switch::payload_register]

            let (tmp1, tmp2) = {
                // Ideally we would just ask regalloc for two temporary registers.
                // However, adding any early defs to the constraints on StackSwitch
                // causes TooManyLiveRegs. Fortunately, we can manually find tmp
                // registers without regalloc: Since our instruction clobbers all
                // registers, we can simply pick any register that is not assigned
                // to the operands.

                let all = crate::isa::x64::abi::ALL_CLOBBERS;

                let used_regs = [
                    **load_context_ptr,
                    **store_context_ptr,
                    **in_payload0,
                    *out_payload0.to_reg(),
                ];

                let mut tmps = all.into_iter().filter_map(|preg| {
                    let reg: Reg = preg.into();
                    if !used_regs.contains(&reg) {
                        WritableGpr::from_writable_reg(isle::WritableReg::from_reg(reg))
                    } else {
                        None
                    }
                });
                (tmps.next().unwrap(), tmps.next().unwrap())
            };

            let layout = stack_switch::control_context_layout();
            let rsp_offset = layout.stack_pointer_offset as i32;
            let pc_offset = layout.ip_offset as i32;
            let rbp_offset = layout.frame_pointer_offset as i32;

            // Location to which someone switch-ing back to this stack will jump
            // to: Right behind the `StackSwitch` instruction
            let resume = sink.get_label();

            //
            // For RBP and RSP we do the following:
            // - Load new value for register from `load_context_ptr` +
            // corresponding offset.
            // - Store previous (!) value of register at `store_context_ptr` +
            // corresponding offset.
            //
            // Since `load_context_ptr` and `store_context_ptr` are allowed to be
            // equal, we need to use a temporary register here.
            //

            let mut exchange = |offset, reg| {
                let addr = SyntheticAmode::real(Amode::imm_reg(offset, **load_context_ptr));
                asm::inst::movq_rm::new(tmp1, addr).emit(sink, info, state);

                asm::inst::movq_mr::new(
                    Amode::imm_reg(offset, **store_context_ptr),
                    Gpr::new(reg).unwrap(),
                )
                .emit(sink, info, state);

                let dst = Writable::from_reg(reg);
                asm::inst::movq_mr::new(dst.map(Gpr::unwrap_new), tmp1.to_reg())
                    .emit(sink, info, state);
            };

            exchange(rsp_offset, regs::rsp());
            exchange(rbp_offset, regs::rbp());

            //
            // Load target PC, store resume PC, jump to target PC
            //

            let addr = SyntheticAmode::real(Amode::imm_reg(pc_offset, **load_context_ptr));
            asm::inst::movq_rm::new(tmp1, addr).emit(sink, info, state);

            let amode = Amode::RipRelative { target: resume };
            asm::inst::leaq_rm::new(tmp2, amode).emit(sink, info, state);

            asm::inst::movq_mr::new(
                Amode::imm_reg(pc_offset, **store_context_ptr),
                tmp2.to_reg(),
            )
            .emit(sink, info, state);

            let inst = Inst::JmpUnknown {
                target: RegMem::reg(tmp1.to_reg().into()),
            };
            emit(&inst, sink, info, state);

            sink.bind_label(resume, state.ctrl_plane_mut());
        }

        Inst::JmpKnown { dst } => {
            let br_start = sink.cur_offset();
            let br_disp_off = br_start + 1;
            let br_end = br_start + 5;

            sink.use_label_at_offset(br_disp_off, *dst, LabelUse::JmpRel32);
            sink.add_uncond_branch(br_start, br_end, *dst);

            sink.put1(0xE9);
            // Placeholder for the label value.
            sink.put4(0x0);
        }

        Inst::WinchJmpIf { cc, taken } => {
            let cond_start = sink.cur_offset();
            let cond_disp_off = cond_start + 2;

            sink.use_label_at_offset(cond_disp_off, *taken, LabelUse::JmpRel32);
            // Since this is not a terminator, don't enroll in the branch inversion mechanism.

            sink.put1(0x0F);
            sink.put1(0x80 + cc.get_enc());
            // Placeholder for the label value.
            sink.put4(0x0);
        }

        Inst::JmpCond {
            cc,
            taken,
            not_taken,
        } => {
            // If taken.
            let cond_start = sink.cur_offset();
            let cond_disp_off = cond_start + 2;
            let cond_end = cond_start + 6;

            sink.use_label_at_offset(cond_disp_off, *taken, LabelUse::JmpRel32);
            let inverted: [u8; 6] = [0x0F, 0x80 + (cc.invert().get_enc()), 0x00, 0x00, 0x00, 0x00];
            sink.add_cond_branch(cond_start, cond_end, *taken, &inverted[..]);

            sink.put1(0x0F);
            sink.put1(0x80 + cc.get_enc());
            // Placeholder for the label value.
            sink.put4(0x0);

            // If not taken.
            let uncond_start = sink.cur_offset();
            let uncond_disp_off = uncond_start + 1;
            let uncond_end = uncond_start + 5;

            sink.use_label_at_offset(uncond_disp_off, *not_taken, LabelUse::JmpRel32);
            sink.add_uncond_branch(uncond_start, uncond_end, *not_taken);

            sink.put1(0xE9);
            // Placeholder for the label value.
            sink.put4(0x0);
        }

        Inst::JmpCondOr {
            cc1,
            cc2,
            taken,
            not_taken,
        } => {
            // Emit:
            //   jcc1 taken
            //   jcc2 taken
            //   jmp not_taken
            //
            // Note that we enroll both conditionals in the
            // branch-chomping mechanism because MachBuffer
            // simplification can continue upward as long as it keeps
            // chomping branches. In the best case, if taken ==
            // not_taken and that one block is the fallthrough block,
            // all three branches can disappear.

            // jcc1 taken
            let cond_1_start = sink.cur_offset();
            let cond_1_disp_off = cond_1_start + 2;
            let cond_1_end = cond_1_start + 6;

            sink.use_label_at_offset(cond_1_disp_off, *taken, LabelUse::JmpRel32);
            let inverted: [u8; 6] = [
                0x0F,
                0x80 + (cc1.invert().get_enc()),
                0x00,
                0x00,
                0x00,
                0x00,
            ];
            sink.add_cond_branch(cond_1_start, cond_1_end, *taken, &inverted[..]);

            sink.put1(0x0F);
            sink.put1(0x80 + cc1.get_enc());
            sink.put4(0x0);

            // jcc2 taken
            let cond_2_start = sink.cur_offset();
            let cond_2_disp_off = cond_2_start + 2;
            let cond_2_end = cond_2_start + 6;

            sink.use_label_at_offset(cond_2_disp_off, *taken, LabelUse::JmpRel32);
            let inverted: [u8; 6] = [
                0x0F,
                0x80 + (cc2.invert().get_enc()),
                0x00,
                0x00,
                0x00,
                0x00,
            ];
            sink.add_cond_branch(cond_2_start, cond_2_end, *taken, &inverted[..]);

            sink.put1(0x0F);
            sink.put1(0x80 + cc2.get_enc());
            sink.put4(0x0);

            // jmp not_taken
            let uncond_start = sink.cur_offset();
            let uncond_disp_off = uncond_start + 1;
            let uncond_end = uncond_start + 5;

            sink.use_label_at_offset(uncond_disp_off, *not_taken, LabelUse::JmpRel32);
            sink.add_uncond_branch(uncond_start, uncond_end, *not_taken);

            sink.put1(0xE9);
            sink.put4(0x0);
        }

        Inst::JmpUnknown { target } => {
            let target = target.clone();

            match target {
                RegMem::Reg { reg } => {
                    let reg_enc = int_reg_enc(reg);
                    emit_std_enc_enc(
                        sink,
                        LegacyPrefixes::None,
                        0xFF,
                        1,
                        4, /*subopcode*/
                        reg_enc,
                        RexFlags::clear_w(),
                    );
                }

                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state.frame_layout(), sink);
                    emit_std_enc_mem(
                        sink,
                        LegacyPrefixes::None,
                        0xFF,
                        1,
                        4, /*subopcode*/
                        addr,
                        RexFlags::clear_w(),
                        0,
                    );
                }
            }
        }

        &Inst::JmpTableSeq {
            idx,
            tmp1,
            tmp2,
            ref targets,
            ref default_target,
            ..
        } => {
            // This sequence is *one* instruction in the vcode, and is expanded only here at
            // emission time, because we cannot allow the regalloc to insert spills/reloads in
            // the middle; we depend on hardcoded PC-rel addressing below.
            //
            // We don't have to worry about emitting islands, because the only label-use type has a
            // maximum range of 2 GB. If we later consider using shorter-range label references,
            // this will need to be revisited.

            // We generate the following sequence. Note that the only read of %idx is before the
            // write to %tmp2, so regalloc may use the same register for both; fix x64/inst/mod.rs
            // if you change this.
            // lea start_of_jump_table_offset(%rip), %tmp1
            // movslq [%tmp1, %idx, 4], %tmp2 ;; shift of 2, viz. multiply index by 4
            // addq %tmp2, %tmp1
            // j *%tmp1
            // $start_of_jump_table:
            // -- jump table entries

            // Load base address of jump table.
            let start_of_jumptable = sink.get_label();
            asm::inst::leaq_rm::new(tmp1, Amode::rip_relative(start_of_jumptable))
                .emit(sink, info, state);

            // Load value out of the jump table. It's a relative offset to the target block, so it
            // might be negative; use a sign-extension.
            let inst = Inst::movsx_rm_r(
                ExtMode::LQ,
                RegMem::mem(Amode::imm_reg_reg_shift(
                    0,
                    Gpr::unwrap_new(tmp1.to_reg()),
                    Gpr::unwrap_new(idx),
                    2,
                )),
                tmp2,
            );
            inst.emit(sink, info, state);

            // Add base of jump table to jump-table-sourced block offset.
            asm::inst::addq_rm::new(tmp1, tmp2).emit(sink, info, state);

            // Branch to computed address.
            let inst = Inst::jmp_unknown(RegMem::reg(tmp1.to_reg()));
            inst.emit(sink, info, state);

            // Emit jump table (table of 32-bit offsets).
            sink.bind_label(start_of_jumptable, state.ctrl_plane_mut());
            let jt_off = sink.cur_offset();
            for &target in targets.iter().chain(std::iter::once(default_target)) {
                let word_off = sink.cur_offset();
                // off_into_table is an addend here embedded in the label to be later patched at
                // the end of codegen. The offset is initially relative to this jump table entry;
                // with the extra addend, it'll be relative to the jump table's start, after
                // patching.
                let off_into_table = word_off - jt_off;
                sink.use_label_at_offset(word_off, target, LabelUse::PCRel32);
                sink.put4(off_into_table);
            }
        }

        Inst::TrapIf { cc, trap_code } => {
            let trap_label = sink.defer_trap(*trap_code);
            one_way_jmp(sink, *cc, trap_label);
        }

        Inst::TrapIfAnd {
            cc1,
            cc2,
            trap_code,
        } => {
            let trap_label = sink.defer_trap(*trap_code);
            let else_label = sink.get_label();

            // Jump to the end if the first condition isn't true, and then if
            // the second condition is true go to the trap.
            one_way_jmp(sink, cc1.invert(), else_label);
            one_way_jmp(sink, *cc2, trap_label);

            sink.bind_label(else_label, state.ctrl_plane_mut());
        }

        Inst::TrapIfOr {
            cc1,
            cc2,
            trap_code,
        } => {
            let trap_label = sink.defer_trap(*trap_code);

            // Emit two jumps to the same trap if either condition code is true.
            one_way_jmp(sink, *cc1, trap_label);
            one_way_jmp(sink, *cc2, trap_label);
        }

        Inst::XmmUnaryRmR {
            op,
            src: src_e,
            dst: reg_g,
        } => {
            let reg_g = reg_g.to_reg().to_reg();
            let src_e = src_e.clone().to_reg_mem().clone();

            let rex = RexFlags::clear_w();

            let (prefix, opcode, num_opcodes) = match op {
                SseOpcode::Pabsb => (LegacyPrefixes::_66, 0x0F381C, 3),
                SseOpcode::Pabsw => (LegacyPrefixes::_66, 0x0F381D, 3),
                SseOpcode::Pabsd => (LegacyPrefixes::_66, 0x0F381E, 3),
                _ => unimplemented!("Opcode {:?} not implemented", op),
            };

            match src_e {
                RegMem::Reg { reg: reg_e } => {
                    emit_std_reg_reg(sink, prefix, opcode, num_opcodes, reg_g, reg_e, rex);
                }
                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state.frame_layout(), sink);
                    emit_std_reg_mem(sink, prefix, opcode, num_opcodes, reg_g, addr, rex, 0);
                }
            };
        }

        Inst::XmmUnaryRmREvex { op, src, dst } => {
            let dst = dst.to_reg().to_reg();
            let src = match src.clone().to_reg_mem().clone() {
                RegMem::Reg { reg } => {
                    RegisterOrAmode::Register(reg.to_real_reg().unwrap().hw_enc().into())
                }
                RegMem::Mem { addr } => {
                    RegisterOrAmode::Amode(addr.finalize(state.frame_layout(), sink))
                }
            };

            let (prefix, map, w, opcode) = match op {
                Avx512Opcode::Vcvtudq2ps => (LegacyPrefixes::_F2, OpcodeMap::_0F, false, 0x7a),
                Avx512Opcode::Vpabsq => (LegacyPrefixes::_66, OpcodeMap::_0F38, true, 0x1f),
                Avx512Opcode::Vpopcntb => (LegacyPrefixes::_66, OpcodeMap::_0F38, false, 0x54),
                _ => unimplemented!("Opcode {:?} not implemented", op),
            };
            EvexInstruction::new()
                .length(EvexVectorLength::V128)
                .prefix(prefix)
                .map(map)
                .w(w)
                .opcode(opcode)
                .tuple_type(op.tuple_type())
                .reg(dst.to_real_reg().unwrap().hw_enc())
                .rm(src)
                .encode(sink);
        }

        Inst::XmmUnaryRmRImmEvex { op, src, dst, imm } => {
            let dst = dst.to_reg().to_reg();
            let src = match src.clone().to_reg_mem().clone() {
                RegMem::Reg { reg } => {
                    RegisterOrAmode::Register(reg.to_real_reg().unwrap().hw_enc().into())
                }
                RegMem::Mem { addr } => {
                    RegisterOrAmode::Amode(addr.finalize(state.frame_layout(), sink))
                }
            };

            let (opcode, opcode_ext, w) = match op {
                Avx512Opcode::VpsraqImm => (0x72, 4, true),
                _ => unimplemented!("Opcode {:?} not implemented", op),
            };
            EvexInstruction::new()
                .length(EvexVectorLength::V128)
                .prefix(LegacyPrefixes::_66)
                .map(OpcodeMap::_0F)
                .w(w)
                .opcode(opcode)
                .reg(opcode_ext)
                .vvvvv(dst.to_real_reg().unwrap().hw_enc())
                .tuple_type(op.tuple_type())
                .rm(src)
                .imm(*imm)
                .encode(sink);
        }

        Inst::XmmRmR {
            op,
            src1,
            src2,
            dst,
        } => emit(
            &Inst::XmmRmRUnaligned {
                op: *op,
                dst: *dst,
                src1: *src1,
                src2: XmmMem::unwrap_new(src2.clone().to_reg_mem()),
            },
            sink,
            info,
            state,
        ),

        Inst::XmmRmRUnaligned {
            op,
            src1,
            src2: src_e,
            dst: reg_g,
        } => {
            let src1 = src1.to_reg();
            let reg_g = reg_g.to_reg().to_reg();
            let src_e = src_e.clone().to_reg_mem().clone();
            debug_assert_eq!(src1, reg_g);

            let rex = RexFlags::clear_w();
            let (prefix, opcode, length) = match op {
                SseOpcode::Packssdw => (LegacyPrefixes::_66, 0x0F6B, 2),
                SseOpcode::Packsswb => (LegacyPrefixes::_66, 0x0F63, 2),
                SseOpcode::Packusdw => (LegacyPrefixes::_66, 0x0F382B, 3),
                SseOpcode::Packuswb => (LegacyPrefixes::_66, 0x0F67, 2),
                SseOpcode::Pmaddubsw => (LegacyPrefixes::_66, 0x0F3804, 3),
                SseOpcode::Pavgb => (LegacyPrefixes::_66, 0x0FE0, 2),
                SseOpcode::Pavgw => (LegacyPrefixes::_66, 0x0FE3, 2),
                SseOpcode::Pcmpeqb => (LegacyPrefixes::_66, 0x0F74, 2),
                SseOpcode::Pcmpeqw => (LegacyPrefixes::_66, 0x0F75, 2),
                SseOpcode::Pcmpeqd => (LegacyPrefixes::_66, 0x0F76, 2),
                SseOpcode::Pcmpeqq => (LegacyPrefixes::_66, 0x0F3829, 3),
                SseOpcode::Pcmpgtb => (LegacyPrefixes::_66, 0x0F64, 2),
                SseOpcode::Pcmpgtw => (LegacyPrefixes::_66, 0x0F65, 2),
                SseOpcode::Pcmpgtd => (LegacyPrefixes::_66, 0x0F66, 2),
                SseOpcode::Pcmpgtq => (LegacyPrefixes::_66, 0x0F3837, 3),
                SseOpcode::Pmaddwd => (LegacyPrefixes::_66, 0x0FF5, 2),
                SseOpcode::Pshufb => (LegacyPrefixes::_66, 0x0F3800, 3),
                _ => unimplemented!("Opcode {:?} not implemented", op),
            };

            match src_e {
                RegMem::Reg { reg: reg_e } => {
                    emit_std_reg_reg(sink, prefix, opcode, length, reg_g, reg_e, rex);
                }
                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state.frame_layout(), sink);
                    emit_std_reg_mem(sink, prefix, opcode, length, reg_g, addr, rex, 0);
                }
            }
        }

        Inst::XmmRmiRVex {
            op,
            src1,
            src2,
            dst,
        } => {
            use LegacyPrefixes as LP;
            use OpcodeMap as OM;

            let dst = dst.to_reg().to_reg();
            let src1 = src1.to_reg();
            let src2 = src2.clone().to_reg_mem_imm().clone();

            // When the opcode is commutative, src1 is xmm{0..7}, and src2 is
            // xmm{8..15}, then we can swap the operands to save one byte on the
            // instruction's encoding.
            let (src1, src2) = match (src1, src2) {
                (src1, RegMemImm::Reg { reg: src2 })
                    if op.is_commutative()
                        && src1.to_real_reg().unwrap().hw_enc() < 8
                        && src2.to_real_reg().unwrap().hw_enc() >= 8 =>
                {
                    (src2, RegMemImm::Reg { reg: src1 })
                }
                (src1, src2) => (src1, src2),
            };

            let src2 = match src2 {
                // For opcodes where one of the operands is an immediate the
                // encoding is a bit different, notably the usage of
                // `opcode_ext`, so handle that specially here.
                RegMemImm::Imm { simm32 } => {
                    let (opcode, opcode_ext, prefix) = match op {
                        AvxOpcode::Vpsrlw => (0x71, 2, LegacyPrefixes::_66),
                        AvxOpcode::Vpsrld => (0x72, 2, LegacyPrefixes::_66),
                        AvxOpcode::Vpsrlq => (0x73, 2, LegacyPrefixes::_66),
                        AvxOpcode::Vpsllw => (0x71, 6, LegacyPrefixes::_66),
                        AvxOpcode::Vpslld => (0x72, 6, LegacyPrefixes::_66),
                        AvxOpcode::Vpsllq => (0x73, 6, LegacyPrefixes::_66),
                        AvxOpcode::Vpsraw => (0x71, 4, LegacyPrefixes::_66),
                        AvxOpcode::Vpsrad => (0x72, 4, LegacyPrefixes::_66),
                        _ => panic!("unexpected rmi_r_vex opcode with immediate {op:?}"),
                    };
                    VexInstruction::new()
                        .length(VexVectorLength::V128)
                        .prefix(prefix)
                        .map(OpcodeMap::_0F)
                        .opcode(opcode)
                        .opcode_ext(opcode_ext)
                        .vvvv(dst.to_real_reg().unwrap().hw_enc())
                        .prefix(LegacyPrefixes::_66)
                        .rm(src1.to_real_reg().unwrap().hw_enc())
                        .imm(simm32.try_into().unwrap())
                        .encode(sink);
                    return;
                }
                RegMemImm::Reg { reg } => {
                    RegisterOrAmode::Register(reg.to_real_reg().unwrap().hw_enc().into())
                }
                RegMemImm::Mem { addr } => {
                    RegisterOrAmode::Amode(addr.finalize(state.frame_layout(), sink))
                }
            };

            let (prefix, map, opcode) = match op {
                AvxOpcode::Vminps => (LP::None, OM::_0F, 0x5D),
                AvxOpcode::Vminpd => (LP::_66, OM::_0F, 0x5D),
                AvxOpcode::Vmaxps => (LP::None, OM::_0F, 0x5F),
                AvxOpcode::Vmaxpd => (LP::_66, OM::_0F, 0x5F),
                AvxOpcode::Vandnps => (LP::None, OM::_0F, 0x55),
                AvxOpcode::Vandnpd => (LP::_66, OM::_0F, 0x55),
                AvxOpcode::Vpandn => (LP::_66, OM::_0F, 0xDF),
                AvxOpcode::Vpsrlw => (LP::_66, OM::_0F, 0xD1),
                AvxOpcode::Vpsrld => (LP::_66, OM::_0F, 0xD2),
                AvxOpcode::Vpsrlq => (LP::_66, OM::_0F, 0xD3),
                AvxOpcode::Vpaddb => (LP::_66, OM::_0F, 0xFC),
                AvxOpcode::Vpaddw => (LP::_66, OM::_0F, 0xFD),
                AvxOpcode::Vpaddd => (LP::_66, OM::_0F, 0xFE),
                AvxOpcode::Vpaddq => (LP::_66, OM::_0F, 0xD4),
                AvxOpcode::Vpaddsb => (LP::_66, OM::_0F, 0xEC),
                AvxOpcode::Vpaddsw => (LP::_66, OM::_0F, 0xED),
                AvxOpcode::Vpaddusb => (LP::_66, OM::_0F, 0xDC),
                AvxOpcode::Vpaddusw => (LP::_66, OM::_0F, 0xDD),
                AvxOpcode::Vpsubb => (LP::_66, OM::_0F, 0xF8),
                AvxOpcode::Vpsubw => (LP::_66, OM::_0F, 0xF9),
                AvxOpcode::Vpsubd => (LP::_66, OM::_0F, 0xFA),
                AvxOpcode::Vpsubq => (LP::_66, OM::_0F, 0xFB),
                AvxOpcode::Vpsubsb => (LP::_66, OM::_0F, 0xE8),
                AvxOpcode::Vpsubsw => (LP::_66, OM::_0F, 0xE9),
                AvxOpcode::Vpsubusb => (LP::_66, OM::_0F, 0xD8),
                AvxOpcode::Vpsubusw => (LP::_66, OM::_0F, 0xD9),
                AvxOpcode::Vpavgb => (LP::_66, OM::_0F, 0xE0),
                AvxOpcode::Vpavgw => (LP::_66, OM::_0F, 0xE3),
                AvxOpcode::Vpand => (LP::_66, OM::_0F, 0xDB),
                AvxOpcode::Vandps => (LP::None, OM::_0F, 0x54),
                AvxOpcode::Vandpd => (LP::_66, OM::_0F, 0x54),
                AvxOpcode::Vpor => (LP::_66, OM::_0F, 0xEB),
                AvxOpcode::Vorps => (LP::None, OM::_0F, 0x56),
                AvxOpcode::Vorpd => (LP::_66, OM::_0F, 0x56),
                AvxOpcode::Vpxor => (LP::_66, OM::_0F, 0xEF),
                AvxOpcode::Vxorps => (LP::None, OM::_0F, 0x57),
                AvxOpcode::Vxorpd => (LP::_66, OM::_0F, 0x57),
                AvxOpcode::Vpmullw => (LP::_66, OM::_0F, 0xD5),
                AvxOpcode::Vpmulld => (LP::_66, OM::_0F38, 0x40),
                AvxOpcode::Vpmulhw => (LP::_66, OM::_0F, 0xE5),
                AvxOpcode::Vpmulhrsw => (LP::_66, OM::_0F38, 0x0B),
                AvxOpcode::Vpmulhuw => (LP::_66, OM::_0F, 0xE4),
                AvxOpcode::Vpmuldq => (LP::_66, OM::_0F38, 0x28),
                AvxOpcode::Vpmuludq => (LP::_66, OM::_0F, 0xF4),
                AvxOpcode::Vpunpckhwd => (LP::_66, OM::_0F, 0x69),
                AvxOpcode::Vpunpcklwd => (LP::_66, OM::_0F, 0x61),
                AvxOpcode::Vunpcklps => (LP::None, OM::_0F, 0x14),
                AvxOpcode::Vunpckhps => (LP::None, OM::_0F, 0x15),
                AvxOpcode::Vaddps => (LP::None, OM::_0F, 0x58),
                AvxOpcode::Vaddpd => (LP::_66, OM::_0F, 0x58),
                AvxOpcode::Vsubps => (LP::None, OM::_0F, 0x5C),
                AvxOpcode::Vsubpd => (LP::_66, OM::_0F, 0x5C),
                AvxOpcode::Vmulps => (LP::None, OM::_0F, 0x59),
                AvxOpcode::Vmulpd => (LP::_66, OM::_0F, 0x59),
                AvxOpcode::Vdivps => (LP::None, OM::_0F, 0x5E),
                AvxOpcode::Vdivpd => (LP::_66, OM::_0F, 0x5E),
                AvxOpcode::Vpcmpeqb => (LP::_66, OM::_0F, 0x74),
                AvxOpcode::Vpcmpeqw => (LP::_66, OM::_0F, 0x75),
                AvxOpcode::Vpcmpeqd => (LP::_66, OM::_0F, 0x76),
                AvxOpcode::Vpcmpeqq => (LP::_66, OM::_0F38, 0x29),
                AvxOpcode::Vpcmpgtb => (LP::_66, OM::_0F, 0x64),
                AvxOpcode::Vpcmpgtw => (LP::_66, OM::_0F, 0x65),
                AvxOpcode::Vpcmpgtd => (LP::_66, OM::_0F, 0x66),
                AvxOpcode::Vpcmpgtq => (LP::_66, OM::_0F38, 0x37),
                AvxOpcode::Vpminsb => (LP::_66, OM::_0F38, 0x38),
                AvxOpcode::Vpminsw => (LP::_66, OM::_0F, 0xEA),
                AvxOpcode::Vpminsd => (LP::_66, OM::_0F38, 0x39),
                AvxOpcode::Vpmaxsb => (LP::_66, OM::_0F38, 0x3C),
                AvxOpcode::Vpmaxsw => (LP::_66, OM::_0F, 0xEE),
                AvxOpcode::Vpmaxsd => (LP::_66, OM::_0F38, 0x3D),
                AvxOpcode::Vpminub => (LP::_66, OM::_0F, 0xDA),
                AvxOpcode::Vpminuw => (LP::_66, OM::_0F38, 0x3A),
                AvxOpcode::Vpminud => (LP::_66, OM::_0F38, 0x3B),
                AvxOpcode::Vpmaxub => (LP::_66, OM::_0F, 0xDE),
                AvxOpcode::Vpmaxuw => (LP::_66, OM::_0F38, 0x3E),
                AvxOpcode::Vpmaxud => (LP::_66, OM::_0F38, 0x3F),
                AvxOpcode::Vpunpcklbw => (LP::_66, OM::_0F, 0x60),
                AvxOpcode::Vpunpckhbw => (LP::_66, OM::_0F, 0x68),
                AvxOpcode::Vpacksswb => (LP::_66, OM::_0F, 0x63),
                AvxOpcode::Vpackssdw => (LP::_66, OM::_0F, 0x6B),
                AvxOpcode::Vpackuswb => (LP::_66, OM::_0F, 0x67),
                AvxOpcode::Vpackusdw => (LP::_66, OM::_0F38, 0x2B),
                AvxOpcode::Vpmaddwd => (LP::_66, OM::_0F, 0xF5),
                AvxOpcode::Vpmaddubsw => (LP::_66, OM::_0F38, 0x04),
                AvxOpcode::Vpshufb => (LP::_66, OM::_0F38, 0x00),
                AvxOpcode::Vpsllw => (LP::_66, OM::_0F, 0xF1),
                AvxOpcode::Vpslld => (LP::_66, OM::_0F, 0xF2),
                AvxOpcode::Vpsllq => (LP::_66, OM::_0F, 0xF3),
                AvxOpcode::Vpsraw => (LP::_66, OM::_0F, 0xE1),
                AvxOpcode::Vpsrad => (LP::_66, OM::_0F, 0xE2),
                AvxOpcode::Vaddss => (LP::_F3, OM::_0F, 0x58),
                AvxOpcode::Vaddsd => (LP::_F2, OM::_0F, 0x58),
                AvxOpcode::Vmulss => (LP::_F3, OM::_0F, 0x59),
                AvxOpcode::Vmulsd => (LP::_F2, OM::_0F, 0x59),
                AvxOpcode::Vsubss => (LP::_F3, OM::_0F, 0x5C),
                AvxOpcode::Vsubsd => (LP::_F2, OM::_0F, 0x5C),
                AvxOpcode::Vdivss => (LP::_F3, OM::_0F, 0x5E),
                AvxOpcode::Vdivsd => (LP::_F2, OM::_0F, 0x5E),
                AvxOpcode::Vminss => (LP::_F3, OM::_0F, 0x5D),
                AvxOpcode::Vminsd => (LP::_F2, OM::_0F, 0x5D),
                AvxOpcode::Vmaxss => (LP::_F3, OM::_0F, 0x5F),
                AvxOpcode::Vmaxsd => (LP::_F2, OM::_0F, 0x5F),
                AvxOpcode::Vphaddw => (LP::_66, OM::_0F38, 0x01),
                AvxOpcode::Vphaddd => (LP::_66, OM::_0F38, 0x02),
                AvxOpcode::Vpunpckldq => (LP::_66, OM::_0F, 0x62),
                AvxOpcode::Vpunpckhdq => (LP::_66, OM::_0F, 0x6A),
                AvxOpcode::Vpunpcklqdq => (LP::_66, OM::_0F, 0x6C),
                AvxOpcode::Vpunpckhqdq => (LP::_66, OM::_0F, 0x6D),
                AvxOpcode::Vmovsd => (LP::_F2, OM::_0F, 0x10),
                AvxOpcode::Vmovss => (LP::_F3, OM::_0F, 0x10),
                AvxOpcode::Vsqrtss => (LP::_F3, OM::_0F, 0x51),
                AvxOpcode::Vsqrtsd => (LP::_F2, OM::_0F, 0x51),
                AvxOpcode::Vunpcklpd => (LP::_66, OM::_0F, 0x14),
                _ => panic!("unexpected rmir vex opcode {op:?}"),
            };
            VexInstruction::new()
                .length(VexVectorLength::V128)
                .prefix(prefix)
                .map(map)
                .opcode(opcode)
                .reg(dst.to_real_reg().unwrap().hw_enc())
                .vvvv(src1.to_real_reg().unwrap().hw_enc())
                .rm(src2)
                .encode(sink);
        }

        Inst::XmmRmRImmVex {
            op,
            src1,
            src2,
            dst,
            imm,
        } => {
            let dst = dst.to_reg().to_reg();
            let src1 = src1.to_reg();
            let src2 = match src2.clone().to_reg_mem().clone() {
                RegMem::Reg { reg } => {
                    RegisterOrAmode::Register(reg.to_real_reg().unwrap().hw_enc().into())
                }
                RegMem::Mem { addr } => {
                    RegisterOrAmode::Amode(addr.finalize(state.frame_layout(), sink))
                }
            };

            let (w, prefix, map, opcode) = match op {
                AvxOpcode::Vcmpps => (false, LegacyPrefixes::None, OpcodeMap::_0F, 0xC2),
                AvxOpcode::Vcmppd => (false, LegacyPrefixes::_66, OpcodeMap::_0F, 0xC2),
                AvxOpcode::Vpalignr => (false, LegacyPrefixes::_66, OpcodeMap::_0F3A, 0x0F),
                AvxOpcode::Vinsertps => (false, LegacyPrefixes::_66, OpcodeMap::_0F3A, 0x21),
                AvxOpcode::Vshufps => (false, LegacyPrefixes::None, OpcodeMap::_0F, 0xC6),
                AvxOpcode::Vpblendw => (false, LegacyPrefixes::_66, OpcodeMap::_0F3A, 0x0E),
                _ => panic!("unexpected rmr_imm_vex opcode {op:?}"),
            };

            VexInstruction::new()
                .length(VexVectorLength::V128)
                .prefix(prefix)
                .map(map)
                .w(w)
                .opcode(opcode)
                .reg(dst.to_real_reg().unwrap().hw_enc())
                .vvvv(src1.to_real_reg().unwrap().hw_enc())
                .rm(src2)
                .imm(*imm)
                .encode(sink);
        }

        Inst::XmmRmRVex3 {
            op,
            src1,
            src2,
            src3,
            dst,
        } => {
            let src1 = src1.to_reg();
            let dst = dst.to_reg().to_reg();
            debug_assert_eq!(src1, dst);
            let src2 = src2.to_reg();
            let src3 = match src3.clone().to_reg_mem().clone() {
                RegMem::Reg { reg } => {
                    RegisterOrAmode::Register(reg.to_real_reg().unwrap().hw_enc().into())
                }
                RegMem::Mem { addr } => {
                    RegisterOrAmode::Amode(addr.finalize(state.frame_layout(), sink))
                }
            };

            let (w, map, opcode) = match op {
                AvxOpcode::Vfmadd132ss => (false, OpcodeMap::_0F38, 0x99),
                AvxOpcode::Vfmadd213ss => (false, OpcodeMap::_0F38, 0xA9),
                AvxOpcode::Vfnmadd132ss => (false, OpcodeMap::_0F38, 0x9D),
                AvxOpcode::Vfnmadd213ss => (false, OpcodeMap::_0F38, 0xAD),
                AvxOpcode::Vfmadd132sd => (true, OpcodeMap::_0F38, 0x99),
                AvxOpcode::Vfmadd213sd => (true, OpcodeMap::_0F38, 0xA9),
                AvxOpcode::Vfnmadd132sd => (true, OpcodeMap::_0F38, 0x9D),
                AvxOpcode::Vfnmadd213sd => (true, OpcodeMap::_0F38, 0xAD),
                AvxOpcode::Vfmadd132ps => (false, OpcodeMap::_0F38, 0x98),
                AvxOpcode::Vfmadd213ps => (false, OpcodeMap::_0F38, 0xA8),
                AvxOpcode::Vfnmadd132ps => (false, OpcodeMap::_0F38, 0x9C),
                AvxOpcode::Vfnmadd213ps => (false, OpcodeMap::_0F38, 0xAC),
                AvxOpcode::Vfmadd132pd => (true, OpcodeMap::_0F38, 0x98),
                AvxOpcode::Vfmadd213pd => (true, OpcodeMap::_0F38, 0xA8),
                AvxOpcode::Vfnmadd132pd => (true, OpcodeMap::_0F38, 0x9C),
                AvxOpcode::Vfnmadd213pd => (true, OpcodeMap::_0F38, 0xAC),
                AvxOpcode::Vfmsub132ss => (false, OpcodeMap::_0F38, 0x9B),
                AvxOpcode::Vfmsub213ss => (false, OpcodeMap::_0F38, 0xAB),
                AvxOpcode::Vfnmsub132ss => (false, OpcodeMap::_0F38, 0x9F),
                AvxOpcode::Vfnmsub213ss => (false, OpcodeMap::_0F38, 0xAF),
                AvxOpcode::Vfmsub132sd => (true, OpcodeMap::_0F38, 0x9B),
                AvxOpcode::Vfmsub213sd => (true, OpcodeMap::_0F38, 0xAB),
                AvxOpcode::Vfnmsub132sd => (true, OpcodeMap::_0F38, 0x9F),
                AvxOpcode::Vfnmsub213sd => (true, OpcodeMap::_0F38, 0xAF),
                AvxOpcode::Vfmsub132ps => (false, OpcodeMap::_0F38, 0x9A),
                AvxOpcode::Vfmsub213ps => (false, OpcodeMap::_0F38, 0xAA),
                AvxOpcode::Vfnmsub132ps => (false, OpcodeMap::_0F38, 0x9E),
                AvxOpcode::Vfnmsub213ps => (false, OpcodeMap::_0F38, 0xAE),
                AvxOpcode::Vfmsub132pd => (true, OpcodeMap::_0F38, 0x9A),
                AvxOpcode::Vfmsub213pd => (true, OpcodeMap::_0F38, 0xAA),
                AvxOpcode::Vfnmsub132pd => (true, OpcodeMap::_0F38, 0x9E),
                AvxOpcode::Vfnmsub213pd => (true, OpcodeMap::_0F38, 0xAE),
                _ => unreachable!(),
            };

            VexInstruction::new()
                .length(VexVectorLength::V128)
                .prefix(LegacyPrefixes::_66)
                .map(map)
                .w(w)
                .opcode(opcode)
                .reg(dst.to_real_reg().unwrap().hw_enc())
                .rm(src3)
                .vvvv(src2.to_real_reg().unwrap().hw_enc())
                .encode(sink);
        }

        Inst::XmmMovRMVex { op, src, dst } => {
            let src = src.to_reg();
            let dst = dst.clone().finalize(state.frame_layout(), sink);

            let (prefix, map, opcode) = match op {
                AvxOpcode::Vmovdqu => (LegacyPrefixes::_F3, OpcodeMap::_0F, 0x7F),
                AvxOpcode::Vmovss => (LegacyPrefixes::_F3, OpcodeMap::_0F, 0x11),
                AvxOpcode::Vmovsd => (LegacyPrefixes::_F2, OpcodeMap::_0F, 0x11),
                AvxOpcode::Vmovups => (LegacyPrefixes::None, OpcodeMap::_0F, 0x11),
                AvxOpcode::Vmovupd => (LegacyPrefixes::_66, OpcodeMap::_0F, 0x11),
                _ => unimplemented!("Opcode {:?} not implemented", op),
            };
            VexInstruction::new()
                .length(VexVectorLength::V128)
                .prefix(prefix)
                .map(map)
                .opcode(opcode)
                .rm(dst)
                .reg(src.to_real_reg().unwrap().hw_enc())
                .encode(sink);
        }

        Inst::XmmCmpRmRVex { op, src1, src2 } => {
            let src1 = src1.to_reg();
            let src2 = match src2.clone().to_reg_mem().clone() {
                RegMem::Reg { reg } => {
                    RegisterOrAmode::Register(reg.to_real_reg().unwrap().hw_enc().into())
                }
                RegMem::Mem { addr } => {
                    RegisterOrAmode::Amode(addr.finalize(state.frame_layout(), sink))
                }
            };

            let (prefix, map, opcode) = match op {
                AvxOpcode::Vucomiss => (LegacyPrefixes::None, OpcodeMap::_0F, 0x2E),
                AvxOpcode::Vucomisd => (LegacyPrefixes::_66, OpcodeMap::_0F, 0x2E),
                AvxOpcode::Vptest => (LegacyPrefixes::_66, OpcodeMap::_0F38, 0x17),
                _ => unimplemented!("Opcode {:?} not implemented", op),
            };

            VexInstruction::new()
                .length(VexVectorLength::V128)
                .prefix(prefix)
                .map(map)
                .opcode(opcode)
                .rm(src2)
                .reg(src1.to_real_reg().unwrap().hw_enc())
                .encode(sink);
        }

        Inst::XmmRmREvex {
            op,
            src1,
            src2,
            dst,
        }
        | Inst::XmmRmREvex3 {
            op,
            src1: _, // `dst` reuses `src1`.
            src2: src1,
            src3: src2,
            dst,
        } => {
            let reused_src = match inst {
                Inst::XmmRmREvex3 { src1, .. } => Some(src1.to_reg()),
                _ => None,
            };
            let src1 = src1.to_reg();
            let src2 = match src2.clone().to_reg_mem().clone() {
                RegMem::Reg { reg } => {
                    RegisterOrAmode::Register(reg.to_real_reg().unwrap().hw_enc().into())
                }
                RegMem::Mem { addr } => {
                    RegisterOrAmode::Amode(addr.finalize(state.frame_layout(), sink))
                }
            };
            let dst = dst.to_reg().to_reg();
            if let Some(src1) = reused_src {
                debug_assert_eq!(src1, dst);
            }

            let (w, opcode, map) = match op {
                Avx512Opcode::Vpermi2b => (false, 0x75, OpcodeMap::_0F38),
                Avx512Opcode::Vpmullq => (true, 0x40, OpcodeMap::_0F38),
                Avx512Opcode::Vpsraq => (true, 0xE2, OpcodeMap::_0F),
                _ => unimplemented!("Opcode {:?} not implemented", op),
            };
            EvexInstruction::new()
                .length(EvexVectorLength::V128)
                .prefix(LegacyPrefixes::_66)
                .map(map)
                .w(w)
                .opcode(opcode)
                .tuple_type(op.tuple_type())
                .reg(dst.to_real_reg().unwrap().hw_enc())
                .vvvvv(src1.to_real_reg().unwrap().hw_enc())
                .rm(src2)
                .encode(sink);
        }

        Inst::XmmMinMaxSeq {
            size,
            is_min,
            lhs,
            rhs,
            dst,
        } => {
            let rhs = rhs.to_reg();
            let lhs = lhs.to_reg();
            let dst = dst.to_writable_reg();
            debug_assert_eq!(rhs, dst.to_reg());

            // Generates the following sequence:
            // cmpss/cmpsd %lhs, %rhs_dst
            // jnz do_min_max
            // jp propagate_nan
            //
            // ;; ordered and equal: propagate the sign bit (for -0 vs 0):
            // {and,or}{ss,sd} %lhs, %rhs_dst
            // j done
            //
            // ;; to get the desired NaN behavior (signalling NaN transformed into a quiet NaN, the
            // ;; NaN value is returned), we add both inputs.
            // propagate_nan:
            // add{ss,sd} %lhs, %rhs_dst
            // j done
            //
            // do_min_max:
            // {min,max}{ss,sd} %lhs, %rhs_dst
            //
            // done:
            let done = sink.get_label();
            let propagate_nan = sink.get_label();
            let do_min_max = sink.get_label();

            let (add_op, cmp_op, and_op, or_op, min_max_op) = match size {
                OperandSize::Size32 => (
                    asm::inst::addss_a::new(dst, lhs).into(),
                    asm::inst::ucomiss_a::new(dst.to_reg(), lhs).into(),
                    asm::inst::andps_a::new(dst, lhs).into(),
                    asm::inst::orps_a::new(dst, lhs).into(),
                    if *is_min {
                        asm::inst::minss_a::new(dst, lhs).into()
                    } else {
                        asm::inst::maxss_a::new(dst, lhs).into()
                    },
                ),
                OperandSize::Size64 => (
                    asm::inst::addsd_a::new(dst, lhs).into(),
                    asm::inst::ucomisd_a::new(dst.to_reg(), lhs).into(),
                    asm::inst::andpd_a::new(dst, lhs).into(),
                    asm::inst::orpd_a::new(dst, lhs).into(),
                    if *is_min {
                        asm::inst::minsd_a::new(dst, lhs).into()
                    } else {
                        asm::inst::maxsd_a::new(dst, lhs).into()
                    },
                ),
                _ => unreachable!(),
            };
            let add_op: AsmInst = add_op;
            let or_op: AsmInst = or_op;
            let min_max_op: AsmInst = min_max_op;
            let cmp_op: AsmInst = cmp_op;

            cmp_op.emit(sink, info, state);

            one_way_jmp(sink, CC::NZ, do_min_max);
            one_way_jmp(sink, CC::P, propagate_nan);

            // Ordered and equal. The operands are bit-identical unless they are zero
            // and negative zero. These instructions merge the sign bits in that
            // case, and are no-ops otherwise.
            let inst: AsmInst = if *is_min { or_op } else { and_op };
            inst.emit(sink, info, state);

            let inst = Inst::jmp_known(done);
            inst.emit(sink, info, state);

            // x86's min/max are not symmetric; if either operand is a NaN, they return the
            // read-only operand: perform an addition between the two operands, which has the
            // desired NaN propagation effects.
            sink.bind_label(propagate_nan, state.ctrl_plane_mut());
            add_op.emit(sink, info, state);

            one_way_jmp(sink, CC::P, done);

            sink.bind_label(do_min_max, state.ctrl_plane_mut());
            min_max_op.emit(sink, info, state);

            sink.bind_label(done, state.ctrl_plane_mut());
        }

        Inst::XmmRmRImm {
            op,
            src1,
            src2,
            dst,
            imm,
            size,
        } => {
            let src1 = *src1;
            let dst = dst.to_reg();
            let src2 = src2.clone();
            debug_assert_eq!(src1, dst);

            let (prefix, opcode, len) = match op {
                SseOpcode::Insertps => (LegacyPrefixes::_66, 0x0F3A21, 3),
                SseOpcode::Palignr => (LegacyPrefixes::_66, 0x0F3A0F, 3),
                SseOpcode::Shufps => (LegacyPrefixes::None, 0x0FC6, 2),
                SseOpcode::Pblendw => (LegacyPrefixes::_66, 0x0F3A0E, 3),
                _ => unimplemented!("Opcode {:?} not implemented", op),
            };
            let rex = RexFlags::from(*size);
            match src2 {
                RegMem::Reg { reg } => {
                    emit_std_reg_reg(sink, prefix, opcode, len, dst, reg, rex);
                }
                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state.frame_layout(), sink);
                    // N.B.: bytes_at_end == 1, because of the `imm` byte below.
                    emit_std_reg_mem(sink, prefix, opcode, len, dst, addr, rex, 1);
                }
            }
            sink.put1(*imm);
        }

        Inst::XmmUninitializedValue { .. } | Inst::GprUninitializedValue { .. } => {
            // These instruction formats only exist to declare a register as a
            // `def`; no code is emitted. This is always immediately followed by
            // an instruction, such as `xor <tmp>, <tmp>`, that semantically
            // reads this undefined value but arithmetically produces the same
            // result regardless of its value.
        }

        Inst::XmmCmpRmR { op, src1, src2 } => {
            let src1 = src1.to_reg();
            let src2 = src2.clone().to_reg_mem().clone();

            let rex = RexFlags::clear_w();
            let (prefix, opcode, len) = match op {
                SseOpcode::Ptest => (LegacyPrefixes::_66, 0x0F3817, 3),
                _ => unimplemented!("Emit xmm cmp rm r"),
            };

            match src2 {
                RegMem::Reg { reg } => {
                    emit_std_reg_reg(sink, prefix, opcode, len, src1, reg, rex);
                }
                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state.frame_layout(), sink);
                    emit_std_reg_mem(sink, prefix, opcode, len, src1, addr, rex, 0);
                }
            }
        }

        Inst::CvtUint64ToFloatSeq {
            dst_size,
            src,
            dst,
            tmp_gpr1,
            tmp_gpr2,
        } => {
            let src = src.to_reg();
            let dst = dst.to_writable_reg();
            let tmp_gpr1 = tmp_gpr1.to_writable_reg();
            let tmp_gpr2 = tmp_gpr2.to_writable_reg();

            // Note: this sequence is specific to 64-bit mode; a 32-bit mode would require a
            // different sequence.
            //
            // Emit the following sequence:
            //
            //  cmp 0, %src
            //  jl handle_negative
            //
            //  ;; handle positive, which can't overflow
            //  cvtsi2sd/cvtsi2ss %src, %dst
            //  j done
            //
            //  ;; handle negative: see below for an explanation of what it's doing.
            //  handle_negative:
            //  mov %src, %tmp_gpr1
            //  shr $1, %tmp_gpr1
            //  mov %src, %tmp_gpr2
            //  and $1, %tmp_gpr2
            //  or %tmp_gpr1, %tmp_gpr2
            //  cvtsi2sd/cvtsi2ss %tmp_gpr2, %dst
            //  addsd/addss %dst, %dst
            //
            //  done:

            assert_ne!(src, tmp_gpr1.to_reg());
            assert_ne!(src, tmp_gpr2.to_reg());

            let handle_negative = sink.get_label();
            let done = sink.get_label();

            // If x seen as a signed int64 is not negative, a signed-conversion will do the right
            // thing.
            // TODO use tst src, src here.
            asm::inst::cmpq_mi_sxb::new(src, 0).emit(sink, info, state);

            one_way_jmp(sink, CC::L, handle_negative);

            // Handle a positive int64, which is the "easy" case: a signed conversion will do the
            // right thing.
            emit_signed_cvt(
                sink,
                info,
                state,
                src,
                dst,
                *dst_size == OperandSize::Size64,
            );

            let inst = Inst::jmp_known(done);
            inst.emit(sink, info, state);

            sink.bind_label(handle_negative, state.ctrl_plane_mut());

            // Divide x by two to get it in range for the signed conversion, keep the LSB, and
            // scale it back up on the FP side.
            let inst = Inst::gen_move(tmp_gpr1, src, types::I64);
            inst.emit(sink, info, state);

            // tmp_gpr1 := src >> 1
            asm::inst::shrq_mi::new(tmp_gpr1, 1).emit(sink, info, state);

            let inst = Inst::gen_move(tmp_gpr2, src, types::I64);
            inst.emit(sink, info, state);

            asm::inst::andq_mi_sxb::new(tmp_gpr2, 1).emit(sink, info, state);

            asm::inst::orq_rm::new(tmp_gpr2, tmp_gpr1).emit(sink, info, state);

            emit_signed_cvt(
                sink,
                info,
                state,
                tmp_gpr2.to_reg(),
                dst,
                *dst_size == OperandSize::Size64,
            );

            let inst: AsmInst = match *dst_size {
                OperandSize::Size64 => asm::inst::addsd_a::new(dst, dst.to_reg()).into(),
                OperandSize::Size32 => asm::inst::addss_a::new(dst, dst.to_reg()).into(),
                _ => unreachable!(),
            };
            inst.emit(sink, info, state);

            sink.bind_label(done, state.ctrl_plane_mut());
        }

        Inst::CvtFloatToSintSeq {
            src_size,
            dst_size,
            is_saturating,
            src,
            dst,
            tmp_gpr,
            tmp_xmm,
        } => {
            use OperandSize::*;

            let src = src.to_reg();
            let dst = dst.to_writable_reg();
            let tmp_gpr = tmp_gpr.to_writable_reg();
            let tmp_xmm = tmp_xmm.to_writable_reg();

            // Emits the following common sequence:
            //
            // cvttss2si/cvttsd2si %src, %dst
            // cmp %dst, 1
            // jno done
            //
            // Then, for saturating conversions:
            //
            // ;; check for NaN
            // cmpss/cmpsd %src, %src
            // jnp not_nan
            // xor %dst, %dst
            //
            // ;; positive inputs get saturated to INT_MAX; negative ones to INT_MIN, which is
            // ;; already in %dst.
            // xorpd %tmp_xmm, %tmp_xmm
            // cmpss/cmpsd %src, %tmp_xmm
            // jnb done
            // mov/movaps $INT_MAX, %dst
            //
            // done:
            //
            // Then, for non-saturating conversions:
            //
            // ;; check for NaN
            // cmpss/cmpsd %src, %src
            // jnp not_nan
            // ud2 trap BadConversionToInteger
            //
            // ;; check if INT_MIN was the correct result, against a magic constant:
            // not_nan:
            // movaps/mov $magic, %tmp_gpr
            // movq/movd %tmp_gpr, %tmp_xmm
            // cmpss/cmpsd %tmp_xmm, %src
            // jnb/jnbe $check_positive
            // ud2 trap IntegerOverflow
            //
            // ;; if positive, it was a real overflow
            // check_positive:
            // xorpd %tmp_xmm, %tmp_xmm
            // cmpss/cmpsd %src, %tmp_xmm
            // jnb done
            // ud2 trap IntegerOverflow
            //
            // done:

            let cmp_op: AsmInst = match src_size {
                Size64 => asm::inst::ucomisd_a::new(src, src).into(),
                Size32 => asm::inst::ucomiss_a::new(src, src).into(),
                _ => unreachable!(),
            };

            let cvtt_op = |dst, src| Inst::External {
                inst: match (*src_size, *dst_size) {
                    (Size32, Size32) => asm::inst::cvttss2si_a::new(dst, src).into(),
                    (Size32, Size64) => asm::inst::cvttss2si_aq::new(dst, src).into(),
                    (Size64, Size32) => asm::inst::cvttsd2si_a::new(dst, src).into(),
                    (Size64, Size64) => asm::inst::cvttsd2si_aq::new(dst, src).into(),
                    _ => unreachable!(),
                },
            };

            let done = sink.get_label();

            // The truncation.
            cvtt_op(dst, src).emit(sink, info, state);

            // Compare against 1, in case of overflow the dst operand was INT_MIN.
            let inst = Inst::cmp_mi_sxb(*dst_size, Gpr::unwrap_new(dst.to_reg()), 1);
            inst.emit(sink, info, state);

            one_way_jmp(sink, CC::NO, done); // no overflow => done

            // Check for NaN.
            cmp_op.emit(sink, info, state);

            if *is_saturating {
                let not_nan = sink.get_label();
                one_way_jmp(sink, CC::NP, not_nan); // go to not_nan if not a NaN

                // For NaN, emit 0.
                let inst: AsmInst = match *dst_size {
                    OperandSize::Size32 => asm::inst::xorl_rm::new(dst, dst).into(),
                    OperandSize::Size64 => asm::inst::xorq_rm::new(dst, dst).into(),
                    _ => unreachable!(),
                };
                inst.emit(sink, info, state);

                let inst = Inst::jmp_known(done);
                inst.emit(sink, info, state);

                sink.bind_label(not_nan, state.ctrl_plane_mut());

                // If the input was positive, saturate to INT_MAX.

                // Zero out tmp_xmm.
                asm::inst::xorpd_a::new(tmp_xmm, tmp_xmm.to_reg()).emit(sink, info, state);

                let inst: AsmInst = match src_size {
                    Size64 => asm::inst::ucomisd_a::new(tmp_xmm.to_reg(), src).into(),
                    Size32 => asm::inst::ucomiss_a::new(tmp_xmm.to_reg(), src).into(),
                    _ => unreachable!(),
                };
                inst.emit(sink, info, state);

                // Jump if >= to done.
                one_way_jmp(sink, CC::NB, done);

                // Otherwise, put INT_MAX.
                if *dst_size == OperandSize::Size64 {
                    let inst = Inst::imm(OperandSize::Size64, 0x7fffffffffffffff, dst);
                    inst.emit(sink, info, state);
                } else {
                    let inst = Inst::imm(OperandSize::Size32, 0x7fffffff, dst);
                    inst.emit(sink, info, state);
                }
            } else {
                let inst = Inst::trap_if(CC::P, TrapCode::BAD_CONVERSION_TO_INTEGER);
                inst.emit(sink, info, state);

                // Check if INT_MIN was the correct result: determine the smallest floating point
                // number that would convert to INT_MIN, put it in a temporary register, and compare
                // against the src register.
                // If the src register is less (or in some cases, less-or-equal) than the threshold,
                // trap!

                let mut no_overflow_cc = CC::NB; // >=
                let output_bits = dst_size.to_bits();
                match *src_size {
                    OperandSize::Size32 => {
                        let cst = (-Ieee32::pow2(output_bits - 1)).bits();
                        let inst = Inst::imm(OperandSize::Size32, cst as u64, tmp_gpr);
                        inst.emit(sink, info, state);
                    }
                    OperandSize::Size64 => {
                        // An f64 can represent `i32::min_value() - 1` exactly with precision to spare,
                        // so there are values less than -2^(N-1) that convert correctly to INT_MIN.
                        let cst = if output_bits < 64 {
                            no_overflow_cc = CC::NBE; // >
                            Ieee64::fcvt_to_sint_negative_overflow(output_bits)
                        } else {
                            -Ieee64::pow2(output_bits - 1)
                        };
                        let inst = Inst::imm(OperandSize::Size64, cst.bits(), tmp_gpr);
                        inst.emit(sink, info, state);
                    }
                    _ => unreachable!(),
                }

                let inst: AsmInst = {
                    let tmp_xmm: WritableXmm = tmp_xmm.map(|r| Xmm::new(r).unwrap());
                    match src_size {
                        Size32 => asm::inst::movd_a::new(tmp_xmm, tmp_gpr).into(),
                        Size64 => asm::inst::movq_a::new(tmp_xmm, tmp_gpr).into(),
                        _ => unreachable!(),
                    }
                };
                inst.emit(sink, info, state);

                let inst: AsmInst = match src_size {
                    Size64 => asm::inst::ucomisd_a::new(src, tmp_xmm.to_reg()).into(),
                    Size32 => asm::inst::ucomiss_a::new(src, tmp_xmm.to_reg()).into(),
                    _ => unreachable!(),
                };
                inst.emit(sink, info, state);

                // no trap if src >= or > threshold
                let inst = Inst::trap_if(no_overflow_cc.invert(), TrapCode::INTEGER_OVERFLOW);
                inst.emit(sink, info, state);

                // If positive, it was a real overflow.

                // Zero out the tmp_xmm register.
                asm::inst::xorpd_a::new(tmp_xmm, tmp_xmm.to_reg()).emit(sink, info, state);

                let inst: AsmInst = match src_size {
                    Size64 => asm::inst::ucomisd_a::new(tmp_xmm.to_reg(), src).into(),
                    Size32 => asm::inst::ucomiss_a::new(tmp_xmm.to_reg(), src).into(),
                    _ => unreachable!(),
                };
                inst.emit(sink, info, state);

                // no trap if 0 >= src
                let inst = Inst::trap_if(CC::B, TrapCode::INTEGER_OVERFLOW);
                inst.emit(sink, info, state);
            }

            sink.bind_label(done, state.ctrl_plane_mut());
        }

        Inst::CvtFloatToUintSeq {
            src_size,
            dst_size,
            is_saturating,
            src,
            dst,
            tmp_gpr,
            tmp_xmm,
            tmp_xmm2,
        } => {
            use OperandSize::*;

            let src = src.to_reg();
            let dst = dst.to_writable_reg();
            let tmp_gpr = tmp_gpr.to_writable_reg();
            let tmp_xmm = tmp_xmm.to_writable_reg();
            let tmp_xmm2 = tmp_xmm2.to_writable_reg();

            // The only difference in behavior between saturating and non-saturating is how we
            // handle errors. Emits the following sequence:
            //
            // movaps/mov 2**(int_width - 1), %tmp_gpr
            // movq/movd %tmp_gpr, %tmp_xmm
            // cmpss/cmpsd %tmp_xmm, %src
            // jnb is_large
            //
            // ;; check for NaN inputs
            // jnp not_nan
            // -- non-saturating: ud2 trap BadConversionToInteger
            // -- saturating: xor %dst, %dst; j done
            //
            // not_nan:
            // cvttss2si/cvttsd2si %src, %dst
            // cmp 0, %dst
            // jnl done
            // -- non-saturating: ud2 trap IntegerOverflow
            // -- saturating: xor %dst, %dst; j done
            //
            // is_large:
            // mov %src, %tmp_xmm2
            // subss/subsd %tmp_xmm, %tmp_xmm2
            // cvttss2si/cvttss2sd %tmp_x, %dst
            // cmp 0, %dst
            // jnl next_is_large
            // -- non-saturating: ud2 trap IntegerOverflow
            // -- saturating: movaps $UINT_MAX, %dst; j done
            //
            // next_is_large:
            // add 2**(int_width -1), %dst ;; 2 instructions for 64-bits integers
            //
            // done:

            assert_ne!(tmp_xmm.to_reg(), src, "tmp_xmm clobbers src!");

            let xor_op = |dst, src| Inst::External {
                inst: match *dst_size {
                    Size32 => asm::inst::xorl_rm::new(dst, src).into(),
                    Size64 => asm::inst::xorq_rm::new(dst, src).into(),
                    _ => unreachable!(),
                },
            };

            let subs_op = |dst, src| Inst::External {
                inst: match *src_size {
                    Size32 => asm::inst::subss_a::new(dst, src).into(),
                    Size64 => asm::inst::subsd_a::new(dst, src).into(),
                    _ => unreachable!(),
                },
            };

            let cvtt_op = |dst, src| Inst::External {
                inst: match (*src_size, *dst_size) {
                    (Size32, Size32) => asm::inst::cvttss2si_a::new(dst, src).into(),
                    (Size32, Size64) => asm::inst::cvttss2si_aq::new(dst, src).into(),
                    (Size64, Size32) => asm::inst::cvttsd2si_a::new(dst, src).into(),
                    (Size64, Size64) => asm::inst::cvttsd2si_aq::new(dst, src).into(),
                    _ => unreachable!(),
                },
            };

            let done = sink.get_label();

            let cst = match src_size {
                OperandSize::Size32 => Ieee32::pow2(dst_size.to_bits() - 1).bits() as u64,
                OperandSize::Size64 => Ieee64::pow2(dst_size.to_bits() - 1).bits(),
                _ => unreachable!(),
            };

            let inst = Inst::imm(*src_size, cst, tmp_gpr);
            inst.emit(sink, info, state);

            let inst: AsmInst = {
                let tmp_xmm: WritableXmm = tmp_xmm.map(|r| Xmm::new(r).unwrap());
                match src_size {
                    Size32 => asm::inst::movd_a::new(tmp_xmm, tmp_gpr).into(),
                    Size64 => asm::inst::movq_a::new(tmp_xmm, tmp_gpr).into(),
                    _ => unreachable!(),
                }
            };
            inst.emit(sink, info, state);

            let inst: AsmInst = match src_size {
                Size64 => asm::inst::ucomisd_a::new(src, tmp_xmm.to_reg()).into(),
                Size32 => asm::inst::ucomiss_a::new(src, tmp_xmm.to_reg()).into(),
                _ => unreachable!(),
            };
            inst.emit(sink, info, state);

            let handle_large = sink.get_label();
            one_way_jmp(sink, CC::NB, handle_large); // jump to handle_large if src >= large_threshold

            if *is_saturating {
                // If not NaN jump over this 0-return, otherwise return 0
                let not_nan = sink.get_label();
                one_way_jmp(sink, CC::NP, not_nan);

                xor_op(dst, dst).emit(sink, info, state);

                let inst = Inst::jmp_known(done);
                inst.emit(sink, info, state);
                sink.bind_label(not_nan, state.ctrl_plane_mut());
            } else {
                // Trap.
                let inst = Inst::trap_if(CC::P, TrapCode::BAD_CONVERSION_TO_INTEGER);
                inst.emit(sink, info, state);
            }

            // Actual truncation for small inputs: if the result is not positive, then we had an
            // overflow.

            cvtt_op(dst, src).emit(sink, info, state);

            let inst = Inst::cmp_mi_sxb(*dst_size, Gpr::unwrap_new(dst.to_reg()), 0);
            inst.emit(sink, info, state);

            one_way_jmp(sink, CC::NL, done); // if dst >= 0, jump to done

            if *is_saturating {
                // The input was "small" (< 2**(width -1)), so the only way to get an integer
                // overflow is because the input was too small: saturate to the min value, i.e. 0.
                let inst: AsmInst = match *dst_size {
                    OperandSize::Size32 => asm::inst::xorl_rm::new(dst, dst).into(),
                    OperandSize::Size64 => asm::inst::xorq_rm::new(dst, dst).into(),
                    _ => unreachable!(),
                };
                inst.emit(sink, info, state);

                let inst = Inst::jmp_known(done);
                inst.emit(sink, info, state);
            } else {
                // Trap.
                asm::inst::ud2_zo::new(TrapCode::INTEGER_OVERFLOW).emit(sink, info, state);
            }

            // Now handle large inputs.

            sink.bind_label(handle_large, state.ctrl_plane_mut());

            let inst = Inst::gen_move(tmp_xmm2, src, types::F64);
            inst.emit(sink, info, state);

            subs_op(tmp_xmm2, tmp_xmm.to_reg()).emit(sink, info, state);

            cvtt_op(dst, tmp_xmm2.to_reg()).emit(sink, info, state);

            let inst = Inst::cmp_mi_sxb(*dst_size, Gpr::unwrap_new(dst.to_reg()), 0);
            inst.emit(sink, info, state);

            if *is_saturating {
                let next_is_large = sink.get_label();
                one_way_jmp(sink, CC::NL, next_is_large); // if dst >= 0, jump to next_is_large

                // The input was "large" (>= 2**(width -1)), so the only way to get an integer
                // overflow is because the input was too large: saturate to the max value.
                let inst = Inst::imm(
                    OperandSize::Size64,
                    if *dst_size == OperandSize::Size64 {
                        u64::max_value()
                    } else {
                        u32::max_value() as u64
                    },
                    dst,
                );
                inst.emit(sink, info, state);

                let inst = Inst::jmp_known(done);
                inst.emit(sink, info, state);
                sink.bind_label(next_is_large, state.ctrl_plane_mut());
            } else {
                let inst = Inst::trap_if(CC::L, TrapCode::INTEGER_OVERFLOW);
                inst.emit(sink, info, state);
            }

            if *dst_size == OperandSize::Size64 {
                let inst = Inst::imm(OperandSize::Size64, 1 << 63, tmp_gpr);
                inst.emit(sink, info, state);

                asm::inst::addq_rm::new(dst, tmp_gpr).emit(sink, info, state);
            } else {
                asm::inst::addl_mi::new(dst, asm::Imm32::new(1 << 31)).emit(sink, info, state);
            }

            sink.bind_label(done, state.ctrl_plane_mut());
        }

        Inst::LoadExtName {
            dst,
            name,
            offset,
            distance,
        } => {
            let dst = dst.to_reg();

            if info.flags.is_pic() {
                // Generates: movq symbol@GOTPCREL(%rip), %dst
                let enc_dst = int_reg_enc(dst);
                sink.put1(0x48 | ((enc_dst >> 3) & 1) << 2);
                sink.put1(0x8B);
                sink.put1(0x05 | ((enc_dst & 7) << 3));
                emit_reloc(sink, Reloc::X86GOTPCRel4, name, -4);
                sink.put4(0);
                // Offset in the relocation above applies to the address of the *GOT entry*, not
                // the loaded address; so we emit a separate add or sub instruction if needed.
                if *offset < 0 {
                    assert!(*offset >= -i32::MAX as i64);
                    sink.put1(0x48 | ((enc_dst >> 3) & 1));
                    sink.put1(0x81);
                    sink.put1(0xe8 | (enc_dst & 7));
                    sink.put4((-*offset) as u32);
                } else if *offset > 0 {
                    assert!(*offset <= i32::MAX as i64);
                    sink.put1(0x48 | ((enc_dst >> 3) & 1));
                    sink.put1(0x81);
                    sink.put1(0xc0 | (enc_dst & 7));
                    sink.put4(*offset as u32);
                }
            } else if distance == &RelocDistance::Near {
                // If we know the distance to the name is within 2GB (e.g., a module-local function),
                // we can generate a RIP-relative address, with a relocation.
                // Generates: lea $name(%rip), $dst
                let enc_dst = int_reg_enc(dst);
                sink.put1(0x48 | ((enc_dst >> 3) & 1) << 2);
                sink.put1(0x8D);
                sink.put1(0x05 | ((enc_dst & 7) << 3));
                emit_reloc(sink, Reloc::X86CallPCRel4, name, -4);
                sink.put4(0);
            } else {
                // The full address can be encoded in the register, with a relocation.
                // Generates: movabsq $name, %dst
                let enc_dst = int_reg_enc(dst);
                sink.put1(0x48 | ((enc_dst >> 3) & 1));
                sink.put1(0xB8 | (enc_dst & 7));
                emit_reloc(sink, Reloc::Abs8, name, *offset);
                sink.put8(0);
            }
        }

        Inst::AtomicRmwSeq {
            ty,
            op,
            mem,
            operand,
            temp,
            dst_old,
        } => {
            let operand = *operand;
            let temp = *temp;
            let temp_r = temp.map(|r| *r);
            let dst_old = *dst_old;
            let dst_old_r = dst_old.map(|r| *r);
            debug_assert_eq!(dst_old.to_reg(), regs::rax());
            let mem = mem.finalize(state.frame_layout(), sink).clone();

            // Emit this:
            //    mov{zbq,zwq,zlq,q}     (%r_address), %rax    // rax = old value
            //  again:
            //    movq                   %rax, %r_temp         // rax = old value, r_temp = old value
            //    `op`q                  %r_operand, %r_temp   // rax = old value, r_temp = new value
            //    lock cmpxchg{b,w,l,q}  %r_temp, (%r_address) // try to store new value
            //    jnz again // If this is taken, rax will have a "revised" old value
            //
            // Operand conventions: IN:  %r_address, %r_operand OUT: %rax (old
            //    value), %r_temp (trashed), %rflags (trashed)
            let again_label = sink.get_label();

            // mov{zbq,zwq,zlq,q} (%r_address), %rax
            // No need to call `add_trap` here, since the `i1` emit will do that.
            let i1 = Inst::load(*ty, mem.clone(), dst_old_r, ExtKind::ZeroExtend);
            i1.emit(sink, info, state);

            // again:
            sink.bind_label(again_label, state.ctrl_plane_mut());

            // movq %rax, %r_temp
            asm::inst::movq_mr::new(temp, dst_old.to_reg()).emit(sink, info, state);

            use AtomicRmwSeqOp as RmwOp;
            match op {
                RmwOp::Nand => {
                    // andq %r_operand, %r_temp
                    asm::inst::andq_rm::new(temp, operand).emit(sink, info, state);

                    // notq %r_temp
                    asm::inst::notq_m::new(PairedGpr::from(temp)).emit(sink, info, state);
                }
                RmwOp::Umin | RmwOp::Umax | RmwOp::Smin | RmwOp::Smax => {
                    // cmp %r_temp, %r_operand
                    let temp = temp.to_reg();
                    match *ty {
                        types::I8 => asm::inst::cmpb_mr::new(operand, temp).emit(sink, info, state),
                        types::I16 => {
                            asm::inst::cmpw_mr::new(operand, temp).emit(sink, info, state)
                        }
                        types::I32 => {
                            asm::inst::cmpl_mr::new(operand, temp).emit(sink, info, state)
                        }
                        types::I64 => {
                            asm::inst::cmpq_mr::new(operand, temp).emit(sink, info, state)
                        }
                        _ => unreachable!(),
                    }

                    // cmovcc %r_operand, %r_temp
                    match op {
                        RmwOp::Umin => {
                            asm::inst::cmovbeq_rm::new(temp_r, *operand).emit(sink, info, state)
                        }
                        RmwOp::Umax => {
                            asm::inst::cmovaeq_rm::new(temp_r, *operand).emit(sink, info, state)
                        }
                        RmwOp::Smin => {
                            asm::inst::cmovleq_rm::new(temp_r, *operand).emit(sink, info, state)
                        }
                        RmwOp::Smax => {
                            asm::inst::cmovgeq_rm::new(temp_r, *operand).emit(sink, info, state)
                        }
                        _ => unreachable!(),
                    }
                }
                RmwOp::And => {
                    // andq %r_operand, %r_temp
                    asm::inst::andq_rm::new(temp, operand).emit(sink, info, state);
                }
                RmwOp::Or => {
                    // orq %r_operand, %r_temp
                    asm::inst::orq_rm::new(temp, operand).emit(sink, info, state);
                }
                RmwOp::Xor => {
                    // xorq %r_operand, %r_temp
                    asm::inst::xorq_rm::new(temp, operand).emit(sink, info, state);
                }
            }

            // lock cmpxchg{b,w,l,q} %r_temp, (%r_address)
            // No need to call `add_trap` here, since the `i4` emit will do that.
            let temp = temp.to_reg();
            let dst_old = PairedGpr::from(dst_old);
            let inst: AsmInst = match *ty {
                types::I8 => asm::inst::lock_cmpxchgb_mr::new(mem, temp, dst_old).into(),
                types::I16 => asm::inst::lock_cmpxchgw_mr::new(mem, temp, dst_old).into(),
                types::I32 => asm::inst::lock_cmpxchgl_mr::new(mem, temp, dst_old).into(),
                types::I64 => asm::inst::lock_cmpxchgq_mr::new(mem, temp, dst_old).into(),
                _ => unreachable!(),
            };
            inst.emit(sink, info, state);

            // jnz again
            one_way_jmp(sink, CC::NZ, again_label);
        }

        Inst::Atomic128RmwSeq {
            op,
            mem,
            operand_low,
            operand_high,
            temp_low,
            temp_high,
            dst_old_low,
            dst_old_high,
        } => {
            let operand_low = *operand_low;
            let operand_high = *operand_high;
            let temp_low = *temp_low;
            let temp_high = *temp_high;
            let dst_old_low = *dst_old_low;
            let dst_old_high = *dst_old_high;
            debug_assert_eq!(temp_low.to_reg(), regs::rbx());
            debug_assert_eq!(temp_high.to_reg(), regs::rcx());
            debug_assert_eq!(dst_old_low.to_reg(), regs::rax());
            debug_assert_eq!(dst_old_high.to_reg(), regs::rdx());
            let mem = mem.finalize(state.frame_layout(), sink).clone();

            let again_label = sink.get_label();

            // Load the initial value.
            asm::inst::movq_rm::new(dst_old_low, mem.clone()).emit(sink, info, state);
            asm::inst::movq_rm::new(dst_old_high, mem.offset(8)).emit(sink, info, state);

            // again:
            sink.bind_label(again_label, state.ctrl_plane_mut());

            // Move old value to temp registers.
            asm::inst::movq_mr::new(temp_low, dst_old_low.to_reg()).emit(sink, info, state);
            asm::inst::movq_mr::new(temp_high, dst_old_high.to_reg()).emit(sink, info, state);

            // Perform the operation.
            use Atomic128RmwSeqOp as RmwOp;
            match op {
                RmwOp::Nand => {
                    // temp &= operand
                    asm::inst::andq_rm::new(temp_low, operand_low).emit(sink, info, state);
                    asm::inst::andq_rm::new(temp_high, operand_high).emit(sink, info, state);

                    // temp = !temp
                    asm::inst::notq_m::new(PairedGpr::from(temp_low)).emit(sink, info, state);
                    asm::inst::notq_m::new(PairedGpr::from(temp_high)).emit(sink, info, state);
                }
                RmwOp::Umin | RmwOp::Umax | RmwOp::Smin | RmwOp::Smax => {
                    // Do a comparison with LHS temp and RHS operand.
                    // Note the opposite argument orders.
                    asm::inst::cmpq_mr::new(temp_low.to_reg(), operand_low).emit(sink, info, state);
                    // This will clobber `temp_high`
                    asm::inst::sbbq_rm::new(temp_high, operand_high).emit(sink, info, state);
                    // Restore the clobbered value
                    asm::inst::movq_mr::new(temp_high, dst_old_high.to_reg())
                        .emit(sink, info, state);
                    match op {
                        RmwOp::Umin => {
                            asm::inst::cmovaeq_rm::new(temp_low, operand_low)
                                .emit(sink, info, state);
                            asm::inst::cmovaeq_rm::new(temp_high, operand_high)
                                .emit(sink, info, state);
                        }
                        RmwOp::Umax => {
                            asm::inst::cmovbq_rm::new(temp_low, operand_low)
                                .emit(sink, info, state);
                            asm::inst::cmovbq_rm::new(temp_high, operand_high)
                                .emit(sink, info, state);
                        }
                        RmwOp::Smin => {
                            asm::inst::cmovgeq_rm::new(temp_low, operand_low)
                                .emit(sink, info, state);
                            asm::inst::cmovgeq_rm::new(temp_high, operand_high)
                                .emit(sink, info, state);
                        }
                        RmwOp::Smax => {
                            asm::inst::cmovlq_rm::new(temp_low, operand_low)
                                .emit(sink, info, state);
                            asm::inst::cmovlq_rm::new(temp_high, operand_high)
                                .emit(sink, info, state);
                        }
                        _ => unreachable!(),
                    }
                }
                RmwOp::Add => {
                    asm::inst::addq_rm::new(temp_low, operand_low).emit(sink, info, state);
                    asm::inst::adcq_rm::new(temp_high, operand_high).emit(sink, info, state);
                }
                RmwOp::Sub => {
                    asm::inst::subq_rm::new(temp_low, operand_low).emit(sink, info, state);
                    asm::inst::sbbq_rm::new(temp_high, operand_high).emit(sink, info, state);
                }
                RmwOp::And => {
                    asm::inst::andq_rm::new(temp_low, operand_low).emit(sink, info, state);
                    asm::inst::andq_rm::new(temp_high, operand_high).emit(sink, info, state);
                }
                RmwOp::Or => {
                    asm::inst::orq_rm::new(temp_low, operand_low).emit(sink, info, state);
                    asm::inst::orq_rm::new(temp_high, operand_high).emit(sink, info, state);
                }
                RmwOp::Xor => {
                    asm::inst::xorq_rm::new(temp_low, operand_low).emit(sink, info, state);
                    asm::inst::xorq_rm::new(temp_high, operand_high).emit(sink, info, state);
                }
            }

            // cmpxchg16b (mem)
            asm::inst::lock_cmpxchg16b_m::new(
                PairedGpr::from(dst_old_low),
                PairedGpr::from(dst_old_high),
                temp_low.to_reg(),
                temp_high.to_reg(),
                mem,
            )
            .emit(sink, info, state);

            // jnz again
            one_way_jmp(sink, CC::NZ, again_label);
        }

        Inst::Atomic128XchgSeq {
            mem,
            operand_low,
            operand_high,
            dst_old_low,
            dst_old_high,
        } => {
            let operand_low = *operand_low;
            let operand_high = *operand_high;
            let dst_old_low = *dst_old_low;
            let dst_old_high = *dst_old_high;
            debug_assert_eq!(operand_low, regs::rbx());
            debug_assert_eq!(operand_high, regs::rcx());
            debug_assert_eq!(dst_old_low.to_reg(), regs::rax());
            debug_assert_eq!(dst_old_high.to_reg(), regs::rdx());
            let mem = mem.finalize(state.frame_layout(), sink).clone();

            let again_label = sink.get_label();

            // Load the initial value.
            asm::inst::movq_rm::new(dst_old_low, mem.clone()).emit(sink, info, state);
            asm::inst::movq_rm::new(dst_old_high, mem.offset(8)).emit(sink, info, state);

            // again:
            sink.bind_label(again_label, state.ctrl_plane_mut());

            // cmpxchg16b (mem)
            asm::inst::lock_cmpxchg16b_m::new(
                PairedGpr::from(dst_old_low),
                PairedGpr::from(dst_old_high),
                operand_low,
                operand_high,
                mem,
            )
            .emit(sink, info, state);

            // jnz again
            one_way_jmp(sink, CC::NZ, again_label);
        }

        Inst::ElfTlsGetAddr { symbol, dst } => {
            let dst = dst.to_reg().to_reg();
            debug_assert_eq!(dst, regs::rax());

            // N.B.: Must be exactly this byte sequence; the linker requires it,
            // because it must know how to rewrite the bytes.

            // data16 lea gv@tlsgd(%rip),%rdi
            sink.put1(0x66); // data16
            sink.put1(0b01001000); // REX.W
            sink.put1(0x8d); // LEA
            sink.put1(0x3d); // ModRM byte
            emit_reloc(sink, Reloc::ElfX86_64TlsGd, symbol, -4);
            sink.put4(0); // offset

            // data16 data16 callq __tls_get_addr-4
            sink.put1(0x66); // data16
            sink.put1(0x66); // data16
            sink.put1(0b01001000); // REX.W
            sink.put1(0xe8); // CALL
            emit_reloc(
                sink,
                Reloc::X86CallPLTRel4,
                &ExternalName::LibCall(LibCall::ElfTlsGetAddr),
                -4,
            );
            sink.put4(0); // offset
        }

        Inst::MachOTlsGetAddr { symbol, dst } => {
            let dst = dst.to_reg().to_reg();
            debug_assert_eq!(dst, regs::rax());

            // movq gv@tlv(%rip), %rdi
            sink.put1(0x48); // REX.w
            sink.put1(0x8b); // MOV
            sink.put1(0x3d); // ModRM byte
            emit_reloc(sink, Reloc::MachOX86_64Tlv, symbol, -4);
            sink.put4(0); // offset

            // callq *(%rdi)
            sink.put1(0xff);
            sink.put1(0x17);
        }

        Inst::CoffTlsGetAddr { symbol, dst, tmp } => {
            let dst = dst.to_reg().to_reg();
            debug_assert_eq!(dst, regs::rax());

            // tmp is used below directly as %rcx
            let tmp = tmp.to_reg().to_reg();
            debug_assert_eq!(tmp, regs::rcx());

            // See: https://gcc.godbolt.org/z/M8or9x6ss
            // And: https://github.com/bjorn3/rustc_codegen_cranelift/issues/388#issuecomment-532930282

            // Emit the following sequence
            // movl	(%rip), %eax          ; IMAGE_REL_AMD64_REL32	_tls_index
            // movq	%gs:88, %rcx
            // movq	(%rcx,%rax,8), %rax
            // leaq	(%rax), %rax          ; Reloc: IMAGE_REL_AMD64_SECREL	symbol

            // Load TLS index for current thread
            // movl	(%rip), %eax
            sink.put1(0x8b); // mov
            sink.put1(0x05);
            emit_reloc(
                sink,
                Reloc::X86PCRel4,
                &ExternalName::KnownSymbol(KnownSymbol::CoffTlsIndex),
                -4,
            );
            sink.put4(0); // offset

            // movq	%gs:88, %rcx
            // Load the TLS Storage Array pointer
            // The gs segment register refers to the base address of the TEB on x64.
            // 0x58 is the offset in the TEB for the ThreadLocalStoragePointer member on x64:
            sink.put_data(&[
                0x65, 0x48, // REX.W
                0x8b, // MOV
                0x0c, 0x25, 0x58, // 0x58 - ThreadLocalStoragePointer offset
                0x00, 0x00, 0x00,
            ]);

            // movq	(%rcx,%rax,8), %rax
            // Load the actual TLS entry for this thread.
            // Computes ThreadLocalStoragePointer + _tls_index*8
            sink.put_data(&[0x48, 0x8b, 0x04, 0xc1]);

            // leaq	(%rax), %rax
            sink.put1(0x48);
            sink.put1(0x8d);
            sink.put1(0x80);
            emit_reloc(sink, Reloc::X86SecRel, symbol, 0);
            sink.put4(0); // offset
        }

        Inst::Unwind { inst } => {
            sink.add_unwind(inst.clone());
        }

        Inst::DummyUse { .. } => {
            // Nothing.
        }

        Inst::External { inst } => {
            let mut known_offsets = [0, 0];
            // These values are transcribed from what is happening in
            // `SyntheticAmode::finalize`. This, plus the `Into` logic
            // converting a `SyntheticAmode` to its external counterpart, are
            // necessary to communicate Cranelift's internal offsets to the
            // assembler; due to when Cranelift determines these offsets, this
            // happens quite late (i.e., here during emission).
            let frame = state.frame_layout();
            known_offsets[usize::from(external::offsets::KEY_INCOMING_ARG)] =
                i32::try_from(frame.tail_args_size + frame.setup_area_size).unwrap();
            known_offsets[usize::from(external::offsets::KEY_SLOT_OFFSET)] =
                i32::try_from(frame.outgoing_args_size).unwrap();
            emit_maybe_shrink(inst, sink, &known_offsets);
        }
    }

    state.clear_post_insn();
}

/// Emit the common sequence used for both direct and indirect tail calls:
///
/// * Copy the new frame's stack arguments over the top of our current frame.
///
/// * Restore the old frame pointer.
///
/// * Initialize the tail callee's stack pointer (simultaneously deallocating
///   the temporary stack space we allocated when creating the new frame's stack
///   arguments).
///
/// * Move the return address into its stack slot.
fn emit_return_call_common_sequence<T>(
    sink: &mut MachBuffer<Inst>,
    info: &EmitInfo,
    state: &mut EmitState,
    call_info: &ReturnCallInfo<T>,
) {
    assert!(
        info.flags.preserve_frame_pointers(),
        "frame pointers aren't fundamentally required for tail calls, \
                 but the current implementation relies on them being present"
    );

    let tmp = call_info.tmp.to_writable_reg();

    for inst in
        X64ABIMachineSpec::gen_clobber_restore(CallConv::Tail, &info.flags, state.frame_layout())
    {
        inst.emit(sink, info, state);
    }

    for inst in X64ABIMachineSpec::gen_epilogue_frame_restore(
        CallConv::Tail,
        &info.flags,
        &info.isa_flags,
        state.frame_layout(),
    ) {
        inst.emit(sink, info, state);
    }

    let incoming_args_diff = state.frame_layout().tail_args_size - call_info.new_stack_arg_size;
    if incoming_args_diff > 0 {
        // Move the saved return address up by `incoming_args_diff`.
        let addr = Amode::imm_reg(0, regs::rsp());
        asm::inst::movq_rm::new(tmp, addr).emit(sink, info, state);
        asm::inst::movq_mr::new(
            Amode::imm_reg(i32::try_from(incoming_args_diff).unwrap(), regs::rsp()),
            Gpr::unwrap_new(tmp.to_reg()),
        )
        .emit(sink, info, state);

        // Increment the stack pointer to shrink the argument area for the new
        // call.
        let rsp = Writable::from_reg(regs::rsp());
        let incoming_args_diff = i32::try_from(incoming_args_diff)
            .expect("`incoming_args_diff` is too large to fit in a 32-bit signed immediate");
        Inst::addq_mi(rsp, incoming_args_diff).emit(sink, info, state);
    }
}

/// Conveniene trait to have an `emit` method on all `asm::inst::*` variants.
trait ExternalEmit {
    fn emit(self, sink: &mut MachBuffer<Inst>, info: &EmitInfo, state: &mut EmitState);
}

impl<I> ExternalEmit for I
where
    I: Into<asm::inst::Inst<CraneliftRegisters>>,
{
    fn emit(self, sink: &mut MachBuffer<Inst>, info: &EmitInfo, state: &mut EmitState) {
        Inst::External { inst: self.into() }.emit(sink, info, state)
    }
}

/// Attempt to "shrink" the provided `inst`.
///
/// This function will inspect `inst` and attempt to return a new instruction
/// which is equivalent semantically but will encode to a smaller binary
/// representation. This is only done for instructions which require register
/// allocation to have already happened, for example shrinking immediates should
/// be done during instruction selection not at this point.
///
/// An example of this optimization is the `AND` instruction. The Intel manual
/// has a smaller encoding for `AND AL, imm8` than it does for `AND r/m8, imm8`.
/// Here the instructions are matched against and if regalloc state indicates
/// that a smaller variant is available then that's swapped to instead.
fn emit_maybe_shrink(inst: &AsmInst, sink: &mut MachBuffer<Inst>, table: &[i32; 2]) {
    use cranelift_assembler_x64::GprMem;
    use cranelift_assembler_x64::inst::*;

    type R = CraneliftRegisters;
    const RAX: PairedGpr = PairedGpr {
        read: Gpr::RAX,
        write: Writable::from_reg(Gpr::RAX),
    };
    const RAX_RM: GprMem<PairedGpr, Gpr> = GprMem::Gpr(RAX);

    match *inst {
        // and
        Inst::andb_mi(andb_mi { rm8: RAX_RM, imm8 }) => {
            andb_i::<R>::new(RAX, imm8).encode(sink, table)
        }
        Inst::andw_mi(andw_mi {
            rm16: RAX_RM,
            imm16,
        }) => andw_i::<R>::new(RAX, imm16).encode(sink, table),
        Inst::andl_mi(andl_mi {
            rm32: RAX_RM,
            imm32,
        }) => andl_i::<R>::new(RAX, imm32).encode(sink, table),
        Inst::andq_mi_sxl(andq_mi_sxl {
            rm64: RAX_RM,
            imm32,
        }) => andq_i_sxl::<R>::new(RAX, imm32).encode(sink, table),

        // or
        Inst::orb_mi(orb_mi { rm8: RAX_RM, imm8 }) => {
            orb_i::<R>::new(RAX, imm8).encode(sink, table)
        }
        Inst::orw_mi(orw_mi {
            rm16: RAX_RM,
            imm16,
        }) => orw_i::<R>::new(RAX, imm16).encode(sink, table),
        Inst::orl_mi(orl_mi {
            rm32: RAX_RM,
            imm32,
        }) => orl_i::<R>::new(RAX, imm32).encode(sink, table),
        Inst::orq_mi_sxl(orq_mi_sxl {
            rm64: RAX_RM,
            imm32,
        }) => orq_i_sxl::<R>::new(RAX, imm32).encode(sink, table),

        // xor
        Inst::xorb_mi(xorb_mi { rm8: RAX_RM, imm8 }) => {
            xorb_i::<R>::new(RAX, imm8).encode(sink, table)
        }
        Inst::xorw_mi(xorw_mi {
            rm16: RAX_RM,
            imm16,
        }) => xorw_i::<R>::new(RAX, imm16).encode(sink, table),
        Inst::xorl_mi(xorl_mi {
            rm32: RAX_RM,
            imm32,
        }) => xorl_i::<R>::new(RAX, imm32).encode(sink, table),
        Inst::xorq_mi_sxl(xorq_mi_sxl {
            rm64: RAX_RM,
            imm32,
        }) => xorq_i_sxl::<R>::new(RAX, imm32).encode(sink, table),

        // add
        Inst::addb_mi(addb_mi { rm8: RAX_RM, imm8 }) => {
            addb_i::<R>::new(RAX, imm8).encode(sink, table)
        }
        Inst::addw_mi(addw_mi {
            rm16: RAX_RM,
            imm16,
        }) => addw_i::<R>::new(RAX, imm16).encode(sink, table),
        Inst::addl_mi(addl_mi {
            rm32: RAX_RM,
            imm32,
        }) => addl_i::<R>::new(RAX, imm32).encode(sink, table),
        Inst::addq_mi_sxl(addq_mi_sxl {
            rm64: RAX_RM,
            imm32,
        }) => addq_i_sxl::<R>::new(RAX, imm32).encode(sink, table),

        // adc
        Inst::adcb_mi(adcb_mi { rm8: RAX_RM, imm8 }) => {
            adcb_i::<R>::new(RAX, imm8).encode(sink, table)
        }
        Inst::adcw_mi(adcw_mi {
            rm16: RAX_RM,
            imm16,
        }) => adcw_i::<R>::new(RAX, imm16).encode(sink, table),
        Inst::adcl_mi(adcl_mi {
            rm32: RAX_RM,
            imm32,
        }) => adcl_i::<R>::new(RAX, imm32).encode(sink, table),
        Inst::adcq_mi_sxl(adcq_mi_sxl {
            rm64: RAX_RM,
            imm32,
        }) => adcq_i_sxl::<R>::new(RAX, imm32).encode(sink, table),

        // sub
        Inst::subb_mi(subb_mi { rm8: RAX_RM, imm8 }) => {
            subb_i::<R>::new(RAX, imm8).encode(sink, table)
        }
        Inst::subw_mi(subw_mi {
            rm16: RAX_RM,
            imm16,
        }) => subw_i::<R>::new(RAX, imm16).encode(sink, table),
        Inst::subl_mi(subl_mi {
            rm32: RAX_RM,
            imm32,
        }) => subl_i::<R>::new(RAX, imm32).encode(sink, table),
        Inst::subq_mi_sxl(subq_mi_sxl {
            rm64: RAX_RM,
            imm32,
        }) => subq_i_sxl::<R>::new(RAX, imm32).encode(sink, table),

        // sbb
        Inst::sbbb_mi(sbbb_mi { rm8: RAX_RM, imm8 }) => {
            sbbb_i::<R>::new(RAX, imm8).encode(sink, table)
        }
        Inst::sbbw_mi(sbbw_mi {
            rm16: RAX_RM,
            imm16,
        }) => sbbw_i::<R>::new(RAX, imm16).encode(sink, table),
        Inst::sbbl_mi(sbbl_mi {
            rm32: RAX_RM,
            imm32,
        }) => sbbl_i::<R>::new(RAX, imm32).encode(sink, table),
        Inst::sbbq_mi_sxl(sbbq_mi_sxl {
            rm64: RAX_RM,
            imm32,
        }) => sbbq_i_sxl::<R>::new(RAX, imm32).encode(sink, table),

        // cmp
        Inst::cmpb_mi(cmpb_mi {
            rm8: GprMem::Gpr(Gpr::RAX),
            imm8,
        }) => cmpb_i::<R>::new(Gpr::RAX, imm8).encode(sink, table),
        Inst::cmpw_mi(cmpw_mi {
            rm16: GprMem::Gpr(Gpr::RAX),
            imm16,
        }) => cmpw_i::<R>::new(Gpr::RAX, imm16).encode(sink, table),
        Inst::cmpl_mi(cmpl_mi {
            rm32: GprMem::Gpr(Gpr::RAX),
            imm32,
        }) => cmpl_i::<R>::new(Gpr::RAX, imm32).encode(sink, table),
        Inst::cmpq_mi(cmpq_mi {
            rm64: GprMem::Gpr(Gpr::RAX),
            imm32,
        }) => cmpq_i::<R>::new(Gpr::RAX, imm32).encode(sink, table),

        // test
        Inst::testb_mi(testb_mi {
            rm8: GprMem::Gpr(Gpr::RAX),
            imm8,
        }) => testb_i::<R>::new(Gpr::RAX, imm8).encode(sink, table),
        Inst::testw_mi(testw_mi {
            rm16: GprMem::Gpr(Gpr::RAX),
            imm16,
        }) => testw_i::<R>::new(Gpr::RAX, imm16).encode(sink, table),
        Inst::testl_mi(testl_mi {
            rm32: GprMem::Gpr(Gpr::RAX),
            imm32,
        }) => testl_i::<R>::new(Gpr::RAX, imm32).encode(sink, table),
        Inst::testq_mi(testq_mi {
            rm64: GprMem::Gpr(Gpr::RAX),
            imm32,
        }) => testq_i::<R>::new(Gpr::RAX, imm32).encode(sink, table),

        // lea
        Inst::leal_rm(leal_rm { r32, m32 }) => emit_lea(
            r32,
            m32,
            sink,
            table,
            |dst, amode, s, t| leal_rm::<R>::new(dst, amode).encode(s, t),
            |dst, simm32, s, t| addl_mi::<R>::new(dst, simm32.unsigned()).encode(s, t),
            |dst, reg, s, t| addl_rm::<R>::new(dst, reg).encode(s, t),
        ),
        Inst::leaq_rm(leaq_rm { r64, m64 }) => emit_lea(
            r64,
            m64,
            sink,
            table,
            |dst, amode, s, t| leaq_rm::<R>::new(dst, amode).encode(s, t),
            |dst, simm32, s, t| addq_mi_sxl::<R>::new(dst, simm32).encode(s, t),
            |dst, reg, s, t| addq_rm::<R>::new(dst, reg).encode(s, t),
        ),

        // All other instructions fall through to here and cannot be shrunk, so
        // return `false` to emit them as usual.
        _ => inst.encode(sink, table),
    }
}

/// If `lea` can actually get encoded as an `add` then do that instead.
/// Currently all candidate `iadd`s become an `lea` pseudo-instruction here but
/// maximizing the use of `lea` is not necessarily optimal. The `lea`
/// instruction goes through dedicated address units on cores which are finite
/// and disjoint from the general ALU, so if everything uses `lea` then those
/// units can get saturated while leaving the ALU idle.
///
/// To help make use of more parts of a CPU, this attempts to use `add` when
/// it's semantically equivalent to `lea`, or otherwise when the `dst` register
/// is the same as the `base` or `index` register.
///
/// FIXME: ideally regalloc is informed of this constraint. Register allocation
/// of `lea` should "attempt" to put the `base` in the same register as `dst`
/// but not at the expense of generating a `mov` instruction. Currently that's
/// not possible but perhaps one day it may be worth it.
fn emit_lea(
    dst: asm::Gpr<WritableGpr>,
    addr: asm::Amode<Gpr>,
    sink: &mut MachBuffer<Inst>,
    table: &[i32; 2],
    lea: fn(WritableGpr, asm::Amode<Gpr>, &mut MachBuffer<Inst>, &[i32; 2]),
    add_mi: fn(PairedGpr, i32, &mut MachBuffer<Inst>, &[i32; 2]),
    add_rm: fn(PairedGpr, Gpr, &mut MachBuffer<Inst>, &[i32; 2]),
) {
    match addr {
        // If `base == dst` then this is `add dst, $imm`, so encode that
        // instead.
        asm::Amode::ImmReg {
            base,
            simm32:
                asm::AmodeOffsetPlusKnownOffset {
                    simm32,
                    offset: None,
                },
            trap: None,
        } if dst.as_ref().to_reg() == base => add_mi(
            PairedGpr {
                read: base,
                write: *dst.as_ref(),
            },
            simm32.value(),
            sink,
            table,
        ),

        // If the offset is 0 and the shift is a scale of 1, then:
        //
        // * If `base == dst`, then this is `addq dst, index`
        // * If `index == dst`, then this is `addq dst, base`
        asm::Amode::ImmRegRegShift {
            base,
            index,
            scale: asm::Scale::One,
            simm32: asm::AmodeOffset::ZERO,
            trap: None,
        } => {
            if dst.as_ref().to_reg() == base {
                add_rm(
                    PairedGpr {
                        read: base,
                        write: *dst.as_ref(),
                    },
                    *index.as_ref(),
                    sink,
                    table,
                )
            } else if dst.as_ref().to_reg() == *index.as_ref() {
                add_rm(
                    PairedGpr {
                        read: *index.as_ref(),
                        write: *dst.as_ref(),
                    },
                    base,
                    sink,
                    table,
                )
            } else {
                lea(*dst.as_ref(), addr, sink, table)
            }
        }

        _ => lea(*dst.as_ref(), addr, sink, table),
    }
}
