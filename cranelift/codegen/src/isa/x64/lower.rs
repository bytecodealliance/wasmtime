//! Lowering rules for X64.

// ISLE integration glue.
pub(super) mod isle;

use crate::ir::{types, ExternalName, Inst as IRInst, InstructionData, LibCall, Opcode, Type};
use crate::isa::x64::abi::*;
use crate::isa::x64::inst::args::*;
use crate::isa::x64::inst::*;
use crate::isa::{x64::settings as x64_settings, x64::X64Backend, CallConv};
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::result::CodegenResult;
use crate::settings::{Flags, TlsModel};
use smallvec::SmallVec;
use target_lexicon::Triple;

//=============================================================================
// Helpers for instruction lowering.

fn is_int_or_ref_ty(ty: Type) -> bool {
    match ty {
        types::I8 | types::I16 | types::I32 | types::I64 | types::R64 => true,
        types::B1 | types::B8 | types::B16 | types::B32 | types::B64 => true,
        types::R32 => panic!("shouldn't have 32-bits refs on x64"),
        _ => false,
    }
}

/// Returns whether the given specified `input` is a result produced by an instruction with Opcode
/// `op`.
// TODO investigate failures with checking against the result index.
fn matches_input(ctx: &mut Lower<Inst>, input: InsnInput, op: Opcode) -> Option<IRInst> {
    let inputs = ctx.get_input_as_source_or_const(input.insn, input.input);
    inputs.inst.as_inst().and_then(|(src_inst, _)| {
        let data = ctx.data(src_inst);
        if data.opcode() == op {
            return Some(src_inst);
        }
        None
    })
}

/// Emits instruction(s) to generate the given 64-bit constant value into a newly-allocated
/// temporary register, returning that register.
fn generate_constant(ctx: &mut Lower<Inst>, ty: Type, c: u64) -> ValueRegs<Reg> {
    let from_bits = ty_bits(ty);
    let masked = if from_bits < 64 {
        c & ((1u64 << from_bits) - 1)
    } else {
        c
    };

    let cst_copy = ctx.alloc_tmp(ty);
    for inst in Inst::gen_constant(cst_copy, masked as u128, ty, |ty| {
        ctx.alloc_tmp(ty).only_reg().unwrap()
    })
    .into_iter()
    {
        ctx.emit(inst);
    }
    non_writable_value_regs(cst_copy)
}

/// Put the given input into possibly multiple registers, and mark it as used (side-effect).
fn put_input_in_regs(ctx: &mut Lower<Inst>, spec: InsnInput) -> ValueRegs<Reg> {
    let ty = ctx.input_ty(spec.insn, spec.input);
    let input = ctx.get_input_as_source_or_const(spec.insn, spec.input);

    if let Some(c) = input.constant {
        // Generate constants fresh at each use to minimize long-range register pressure.
        generate_constant(ctx, ty, c)
    } else {
        ctx.put_input_in_regs(spec.insn, spec.input)
    }
}

/// Put the given input into a register, and mark it as used (side-effect).
fn put_input_in_reg(ctx: &mut Lower<Inst>, spec: InsnInput) -> Reg {
    put_input_in_regs(ctx, spec)
        .only_reg()
        .expect("Multi-register value not expected")
}

/// Determines whether a load operation (indicated by `src_insn`) can be merged
/// into the current lowering point. If so, returns the address-base source (as
/// an `InsnInput`) and an offset from that address from which to perform the
/// load.
fn is_mergeable_load(ctx: &mut Lower<Inst>, src_insn: IRInst) -> Option<(InsnInput, i32)> {
    let insn_data = ctx.data(src_insn);
    let inputs = ctx.num_inputs(src_insn);
    if inputs != 1 {
        return None;
    }

    let load_ty = ctx.output_ty(src_insn, 0);
    if ty_bits(load_ty) < 32 {
        // Narrower values are handled by ALU insts that are at least 32 bits
        // wide, which is normally OK as we ignore upper buts; but, if we
        // generate, e.g., a direct-from-memory 32-bit add for a byte value and
        // the byte is the last byte in a page, the extra data that we load is
        // incorrectly accessed. So we only allow loads to merge for
        // 32-bit-and-above widths.
        return None;
    }

    // SIMD instructions can only be load-coalesced when the loaded value comes
    // from an aligned address.
    if load_ty.is_vector() && !insn_data.memflags().map_or(false, |f| f.aligned()) {
        return None;
    }

    // Just testing the opcode is enough, because the width will always match if
    // the type does (and the type should match if the CLIF is properly
    // constructed).
    if insn_data.opcode() == Opcode::Load {
        let offset = insn_data
            .load_store_offset()
            .expect("load should have offset");
        Some((
            InsnInput {
                insn: src_insn,
                input: 0,
            },
            offset,
        ))
    } else {
        None
    }
}

/// Put the given input into a register or a memory operand.
/// Effectful: may mark the given input as used, when returning the register form.
fn input_to_reg_mem(ctx: &mut Lower<Inst>, spec: InsnInput) -> RegMem {
    let inputs = ctx.get_input_as_source_or_const(spec.insn, spec.input);

    if let Some(c) = inputs.constant {
        // Generate constants fresh at each use to minimize long-range register pressure.
        let ty = ctx.input_ty(spec.insn, spec.input);
        return RegMem::reg(generate_constant(ctx, ty, c).only_reg().unwrap());
    }

    if let InputSourceInst::UniqueUse(src_insn, 0) = inputs.inst {
        if let Some((addr_input, offset)) = is_mergeable_load(ctx, src_insn) {
            ctx.sink_inst(src_insn);
            let amode = lower_to_amode(ctx, addr_input, offset);
            return RegMem::mem(amode);
        }
    }

    RegMem::reg(
        ctx.put_input_in_regs(spec.insn, spec.input)
            .only_reg()
            .unwrap(),
    )
}

fn input_to_imm(ctx: &mut Lower<Inst>, spec: InsnInput) -> Option<u64> {
    ctx.get_input_as_source_or_const(spec.insn, spec.input)
        .constant
}

/// Emit an instruction to insert a value `src` into a lane of `dst`.
fn emit_insert_lane(ctx: &mut Lower<Inst>, src: RegMem, dst: Writable<Reg>, lane: u8, ty: Type) {
    if !ty.is_float() {
        let (sse_op, size) = match ty.lane_bits() {
            8 => (SseOpcode::Pinsrb, OperandSize::Size32),
            16 => (SseOpcode::Pinsrw, OperandSize::Size32),
            32 => (SseOpcode::Pinsrd, OperandSize::Size32),
            64 => (SseOpcode::Pinsrd, OperandSize::Size64),
            _ => panic!("Unable to insertlane for lane size: {}", ty.lane_bits()),
        };
        ctx.emit(Inst::xmm_rm_r_imm(sse_op, src, dst, lane, size));
    } else if ty == types::F32 {
        let sse_op = SseOpcode::Insertps;
        // Insert 32-bits from replacement (at index 00, bits 7:8) to vector (lane
        // shifted into bits 5:6).
        let lane = 0b00_00_00_00 | lane << 4;
        ctx.emit(Inst::xmm_rm_r_imm(
            sse_op,
            src,
            dst,
            lane,
            OperandSize::Size32,
        ));
    } else if ty == types::F64 {
        let sse_op = match lane {
            // Move the lowest quadword in replacement to vector without changing
            // the upper bits.
            0 => SseOpcode::Movsd,
            // Move the low 64 bits of replacement vector to the high 64 bits of the
            // vector.
            1 => SseOpcode::Movlhps,
            _ => unreachable!(),
        };
        // Here we use the `xmm_rm_r` encoding because it correctly tells the register
        // allocator how we are using `dst`: we are using `dst` as a `mod` whereas other
        // encoding formats like `xmm_unary_rm_r` treat it as a `def`.
        ctx.emit(Inst::xmm_rm_r(sse_op, src, dst));
    } else {
        panic!("unable to emit insertlane for type: {}", ty)
    }
}

/// Emit an instruction to extract a lane of `src` into `dst`.
fn emit_extract_lane(ctx: &mut Lower<Inst>, src: Reg, dst: Writable<Reg>, lane: u8, ty: Type) {
    if !ty.is_float() {
        let (sse_op, size) = match ty.lane_bits() {
            8 => (SseOpcode::Pextrb, OperandSize::Size32),
            16 => (SseOpcode::Pextrw, OperandSize::Size32),
            32 => (SseOpcode::Pextrd, OperandSize::Size32),
            64 => (SseOpcode::Pextrd, OperandSize::Size64),
            _ => panic!("Unable to extractlane for lane size: {}", ty.lane_bits()),
        };
        let src = RegMem::reg(src);
        ctx.emit(Inst::xmm_rm_r_imm(sse_op, src, dst, lane, size));
    } else if ty == types::F32 || ty == types::F64 {
        if lane == 0 {
            // Remove the extractlane instruction, leaving the float where it is. The upper
            // bits will remain unchanged; for correctness, this relies on Cranelift type
            // checking to avoid using those bits.
            ctx.emit(Inst::gen_move(dst, src, ty));
        } else {
            // Otherwise, shuffle the bits in `lane` to the lowest lane.
            let sse_op = SseOpcode::Pshufd;
            let mask = match ty {
                // Move the value at `lane` to lane 0, copying existing value at lane 0 to
                // other lanes. Again, this relies on Cranelift type checking to avoid
                // using those bits.
                types::F32 => {
                    assert!(lane > 0 && lane < 4);
                    0b00_00_00_00 | lane
                }
                // Move the value at `lane` 1 (we know it must be 1 because of the `if`
                // statement above) to lane 0 and leave lane 1 unchanged. The Cranelift type
                // checking assumption also applies here.
                types::F64 => {
                    assert!(lane == 1);
                    0b11_10_11_10
                }
                _ => unreachable!(),
            };
            let src = RegMem::reg(src);
            ctx.emit(Inst::xmm_rm_r_imm(
                sse_op,
                src,
                dst,
                mask,
                OperandSize::Size32,
            ));
        }
    } else {
        panic!("unable to emit extractlane for type: {}", ty)
    }
}

fn emit_vm_call(
    ctx: &mut Lower<Inst>,
    flags: &Flags,
    triple: &Triple,
    libcall: LibCall,
    inputs: &[Reg],
    outputs: &[Writable<Reg>],
) -> CodegenResult<()> {
    let extname = ExternalName::LibCall(libcall);

    let dist = if flags.use_colocated_libcalls() {
        RelocDistance::Near
    } else {
        RelocDistance::Far
    };

    // TODO avoid recreating signatures for every single Libcall function.
    let call_conv = CallConv::for_libcall(flags, CallConv::triple_default(triple));
    let sig = libcall.signature(call_conv);
    let caller_conv = ctx.abi().call_conv();

    let mut abi = X64Caller::from_func(&sig, &extname, dist, caller_conv, flags)?;

    abi.emit_stack_pre_adjust(ctx);

    assert_eq!(inputs.len(), abi.num_args());

    for (i, input) in inputs.iter().enumerate() {
        abi.emit_copy_regs_to_arg(ctx, i, ValueRegs::one(*input));
    }

    abi.emit_call(ctx);
    for (i, output) in outputs.iter().enumerate() {
        abi.emit_copy_retval_to_regs(ctx, i, ValueRegs::one(*output));
    }
    abi.emit_stack_post_adjust(ctx);

    Ok(())
}

/// Returns whether the given input is a shift by a constant value less or equal than 3.
/// The goal is to embed it within an address mode.
fn matches_small_constant_shift(ctx: &mut Lower<Inst>, spec: InsnInput) -> Option<(InsnInput, u8)> {
    matches_input(ctx, spec, Opcode::Ishl).and_then(|shift| {
        match input_to_imm(
            ctx,
            InsnInput {
                insn: shift,
                input: 1,
            },
        ) {
            Some(shift_amt) if shift_amt <= 3 => Some((
                InsnInput {
                    insn: shift,
                    input: 0,
                },
                shift_amt as u8,
            )),
            _ => None,
        }
    })
}

/// Lowers an instruction to one of the x86 addressing modes.
///
/// Note: the 32-bit offset in Cranelift has to be sign-extended, which maps x86's behavior.
fn lower_to_amode(ctx: &mut Lower<Inst>, spec: InsnInput, offset: i32) -> Amode {
    let flags = ctx
        .memflags(spec.insn)
        .expect("Instruction with amode should have memflags");

    // We now either have an add that we must materialize, or some other input; as well as the
    // final offset.
    if let Some(add) = matches_input(ctx, spec, Opcode::Iadd) {
        debug_assert_eq!(ctx.output_ty(add, 0), types::I64);
        let add_inputs = &[
            InsnInput {
                insn: add,
                input: 0,
            },
            InsnInput {
                insn: add,
                input: 1,
            },
        ];

        // TODO heap_addr legalization generates a uext64 *after* the shift, so these optimizations
        // aren't happening in the wasm case. We could do better, given some range analysis.
        let (base, index, shift) = if let Some((shift_input, shift_amt)) =
            matches_small_constant_shift(ctx, add_inputs[0])
        {
            (
                put_input_in_reg(ctx, add_inputs[1]),
                put_input_in_reg(ctx, shift_input),
                shift_amt,
            )
        } else if let Some((shift_input, shift_amt)) =
            matches_small_constant_shift(ctx, add_inputs[1])
        {
            (
                put_input_in_reg(ctx, add_inputs[0]),
                put_input_in_reg(ctx, shift_input),
                shift_amt,
            )
        } else {
            for i in 0..=1 {
                // Try to pierce through uextend.
                if let Some(uextend) = matches_input(
                    ctx,
                    InsnInput {
                        insn: add,
                        input: i,
                    },
                    Opcode::Uextend,
                ) {
                    if let Some(cst) = ctx.get_input_as_source_or_const(uextend, 0).constant {
                        // Zero the upper bits.
                        let input_size = ctx.input_ty(uextend, 0).bits() as u64;
                        let shift: u64 = 64 - input_size;
                        let uext_cst: u64 = (cst << shift) >> shift;

                        let final_offset = (offset as i64).wrapping_add(uext_cst as i64);
                        if low32_will_sign_extend_to_64(final_offset as u64) {
                            let base = put_input_in_reg(ctx, add_inputs[1 - i]);
                            return Amode::imm_reg(final_offset as u32, base).with_flags(flags);
                        }
                    }
                }

                // If it's a constant, add it directly!
                if let Some(cst) = ctx.get_input_as_source_or_const(add, i).constant {
                    let final_offset = (offset as i64).wrapping_add(cst as i64);
                    if low32_will_sign_extend_to_64(final_offset as u64) {
                        let base = put_input_in_reg(ctx, add_inputs[1 - i]);
                        return Amode::imm_reg(final_offset as u32, base).with_flags(flags);
                    }
                }
            }

            (
                put_input_in_reg(ctx, add_inputs[0]),
                put_input_in_reg(ctx, add_inputs[1]),
                0,
            )
        };

        return Amode::imm_reg_reg_shift(
            offset as u32,
            Gpr::new(base).unwrap(),
            Gpr::new(index).unwrap(),
            shift,
        )
        .with_flags(flags);
    }

    let input = put_input_in_reg(ctx, spec);
    Amode::imm_reg(offset as u32, input).with_flags(flags)
}

//=============================================================================
// Top-level instruction lowering entry point, for one instruction.

/// Actually codegen an instruction's results into registers.
fn lower_insn_to_regs(
    ctx: &mut Lower<Inst>,
    insn: IRInst,
    flags: &Flags,
    isa_flags: &x64_settings::Flags,
    triple: &Triple,
) -> CodegenResult<()> {
    let op = ctx.data(insn).opcode();

    let inputs: SmallVec<[InsnInput; 4]> = (0..ctx.num_inputs(insn))
        .map(|i| InsnInput { insn, input: i })
        .collect();
    let outputs: SmallVec<[InsnOutput; 2]> = (0..ctx.num_outputs(insn))
        .map(|i| InsnOutput { insn, output: i })
        .collect();

    let ty = if outputs.len() > 0 {
        Some(ctx.output_ty(insn, 0))
    } else {
        None
    };

    if let Ok(()) = isle::lower(ctx, triple, flags, isa_flags, &outputs, insn) {
        return Ok(());
    }

    let implemented_in_isle = |ctx: &mut Lower<Inst>| {
        unreachable!(
            "implemented in ISLE: inst = `{}`, type = `{:?}`",
            ctx.dfg().display_inst(insn),
            ty
        )
    };

    match op {
        Opcode::Iconst
        | Opcode::Bconst
        | Opcode::F32const
        | Opcode::F64const
        | Opcode::Null
        | Opcode::Iadd
        | Opcode::IaddIfcout
        | Opcode::SaddSat
        | Opcode::UaddSat
        | Opcode::Isub
        | Opcode::SsubSat
        | Opcode::UsubSat
        | Opcode::AvgRound
        | Opcode::Band
        | Opcode::Bor
        | Opcode::Bxor
        | Opcode::Imul
        | Opcode::BandNot
        | Opcode::Iabs
        | Opcode::Imax
        | Opcode::Umax
        | Opcode::Imin
        | Opcode::Umin
        | Opcode::Bnot
        | Opcode::Bitselect
        | Opcode::Vselect
        | Opcode::Ushr
        | Opcode::Sshr
        | Opcode::Ishl
        | Opcode::Rotl
        | Opcode::Rotr
        | Opcode::Ineg
        | Opcode::Trap
        | Opcode::ResumableTrap
        | Opcode::Clz
        | Opcode::Ctz
        | Opcode::Popcnt
        | Opcode::Bitrev
        | Opcode::IsNull
        | Opcode::IsInvalid
        | Opcode::Uextend
        | Opcode::Sextend
        | Opcode::Breduce
        | Opcode::Bextend
        | Opcode::Ireduce
        | Opcode::Bint
        | Opcode::Debugtrap
        | Opcode::WideningPairwiseDotProductS
        | Opcode::Fadd
        | Opcode::Fsub
        | Opcode::Fmul
        | Opcode::Fdiv
        | Opcode::Fmin
        | Opcode::Fmax
        | Opcode::FminPseudo
        | Opcode::FmaxPseudo
        | Opcode::Sqrt
        | Opcode::Fpromote
        | Opcode::FvpromoteLow
        | Opcode::Fdemote
        | Opcode::Fvdemote
        | Opcode::Fma
        | Opcode::Icmp
        | Opcode::Fcmp
        | Opcode::Load
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
        | Opcode::Uload32x2
        | Opcode::Store
        | Opcode::Istore8
        | Opcode::Istore16
        | Opcode::Istore32
        | Opcode::AtomicRmw
        | Opcode::AtomicCas
        | Opcode::AtomicLoad
        | Opcode::AtomicStore
        | Opcode::Fence
        | Opcode::FuncAddr
        | Opcode::SymbolValue
        | Opcode::Return
        | Opcode::Call
        | Opcode::CallIndirect
        | Opcode::Trapif
        | Opcode::Trapff
        | Opcode::GetFramePointer
        | Opcode::GetStackPointer
        | Opcode::GetReturnAddress
        | Opcode::Select
        | Opcode::Selectif
        | Opcode::SelectifSpectreGuard
        | Opcode::FcvtFromSint
        | Opcode::FcvtLowFromSint
        | Opcode::FcvtFromUint
        | Opcode::FcvtToUint
        | Opcode::FcvtToSint
        | Opcode::FcvtToUintSat
        | Opcode::FcvtToSintSat
        | Opcode::IaddPairwise
        | Opcode::UwidenHigh
        | Opcode::UwidenLow
        | Opcode::SwidenHigh
        | Opcode::SwidenLow
        | Opcode::Snarrow
        | Opcode::Unarrow
        | Opcode::Bitcast
        | Opcode::Fabs
        | Opcode::Fneg
        | Opcode::Fcopysign
        | Opcode::Ceil
        | Opcode::Floor
        | Opcode::Nearest
        | Opcode::Trunc
        | Opcode::StackAddr
        | Opcode::Udiv
        | Opcode::Urem
        | Opcode::Sdiv
        | Opcode::Srem
        | Opcode::Umulhi
        | Opcode::Smulhi
        | Opcode::GetPinnedReg
        | Opcode::SetPinnedReg
        | Opcode::Vconst
        | Opcode::RawBitcast
        | Opcode::Insertlane
        | Opcode::Shuffle
        | Opcode::Swizzle => {
            implemented_in_isle(ctx);
        }

        Opcode::DynamicStackAddr => unimplemented!("DynamicStackAddr"),

        Opcode::Extractlane => {
            // The instruction format maps to variables like: %dst = extractlane %src, %lane
            let ty = ty.unwrap();
            let dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let src_ty = ctx.input_ty(insn, 0);
            assert_eq!(src_ty.bits(), 128);
            let src = put_input_in_reg(ctx, inputs[0]);
            let lane = if let InstructionData::BinaryImm8 { imm, .. } = ctx.data(insn) {
                *imm
            } else {
                unreachable!();
            };
            debug_assert!(lane < src_ty.lane_count() as u8);

            emit_extract_lane(ctx, src, dst, lane, ty);
        }

        Opcode::ScalarToVector => {
            // When moving a scalar value to a vector register, we must be handle several
            // situations:
            //  1. a scalar float is already in an XMM register, so we simply move it
            //  2. a scalar of any other type resides in a GPR register: MOVD moves the bits to an
            //     XMM register and zeroes the upper bits
            //  3. a scalar (float or otherwise) that has previously been loaded from memory (e.g.
            //     the default lowering of Wasm's `load[32|64]_zero`) can be lowered to a single
            //     MOVSS/MOVSD instruction; to do this, we rely on `input_to_reg_mem` to sink the
            //     unused load.
            let src = input_to_reg_mem(ctx, inputs[0]);
            let src_ty = ctx.input_ty(insn, 0);
            let dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let dst_ty = ty.unwrap();
            assert!(src_ty == dst_ty.lane_type() && dst_ty.bits() == 128);
            match src {
                RegMem::Reg { reg } => {
                    if src_ty.is_float() {
                        // Case 1: when moving a scalar float, we simply move from one XMM register
                        // to another, expecting the register allocator to elide this. Here we
                        // assume that the upper bits of a scalar float have not been munged with
                        // (the same assumption the old backend makes).
                        ctx.emit(Inst::gen_move(dst, reg, dst_ty));
                    } else {
                        // Case 2: when moving a scalar value of any other type, use MOVD to zero
                        // the upper lanes.
                        let src_size = match src_ty.bits() {
                            32 => OperandSize::Size32,
                            64 => OperandSize::Size64,
                            _ => unimplemented!("invalid source size for type: {}", src_ty),
                        };
                        ctx.emit(Inst::gpr_to_xmm(SseOpcode::Movd, src, src_size, dst));
                    }
                }
                RegMem::Mem { .. } => {
                    // Case 3: when presented with `load + scalar_to_vector`, coalesce into a single
                    // MOVSS/MOVSD instruction.
                    let opcode = match src_ty.bits() {
                        32 => SseOpcode::Movss,
                        64 => SseOpcode::Movsd,
                        _ => unimplemented!("unable to move scalar to vector for type: {}", src_ty),
                    };
                    ctx.emit(Inst::xmm_mov(opcode, src, dst));
                }
            }
        }

        Opcode::Splat => {
            let ty = ty.unwrap();
            assert_eq!(ty.bits(), 128);
            let src_ty = ctx.input_ty(insn, 0);
            assert!(src_ty.bits() < 128);

            let src = input_to_reg_mem(ctx, inputs[0]);
            let dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();

            // We know that splat will overwrite all of the lanes of `dst` but it takes several
            // instructions to do so. Because of the multiple instructions, there is no good way to
            // declare `dst` a `def` except with the following pseudo-instruction.
            ctx.emit(Inst::xmm_uninit_value(dst));

            // TODO: eventually many of these sequences could be optimized with AVX's VBROADCAST*
            // and VPBROADCAST*.
            match ty.lane_bits() {
                8 => {
                    emit_insert_lane(ctx, src, dst, 0, ty.lane_type());
                    // Initialize a register with all 0s.
                    let tmp = ctx.alloc_tmp(ty).only_reg().unwrap();
                    ctx.emit(Inst::xmm_rm_r(SseOpcode::Pxor, RegMem::from(tmp), tmp));
                    // Shuffle the lowest byte lane to all other lanes.
                    ctx.emit(Inst::xmm_rm_r(SseOpcode::Pshufb, RegMem::from(tmp), dst))
                }
                16 => {
                    emit_insert_lane(ctx, src.clone(), dst, 0, ty.lane_type());
                    emit_insert_lane(ctx, src, dst, 1, ty.lane_type());
                    // Shuffle the lowest two lanes to all other lanes.
                    ctx.emit(Inst::xmm_rm_r_imm(
                        SseOpcode::Pshufd,
                        RegMem::from(dst),
                        dst,
                        0,
                        OperandSize::Size32,
                    ))
                }
                32 => {
                    emit_insert_lane(ctx, src, dst, 0, ty.lane_type());
                    // Shuffle the lowest lane to all other lanes.
                    ctx.emit(Inst::xmm_rm_r_imm(
                        SseOpcode::Pshufd,
                        RegMem::from(dst),
                        dst,
                        0,
                        OperandSize::Size32,
                    ))
                }
                64 => {
                    emit_insert_lane(ctx, src.clone(), dst, 0, ty.lane_type());
                    emit_insert_lane(ctx, src, dst, 1, ty.lane_type());
                }
                _ => panic!("Invalid type to splat: {}", ty),
            }
        }

        Opcode::VanyTrue => {
            let dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let src_ty = ctx.input_ty(insn, 0);
            assert_eq!(src_ty.bits(), 128);
            let src = put_input_in_reg(ctx, inputs[0]);
            // Set the ZF if the result is all zeroes.
            ctx.emit(Inst::xmm_cmp_rm_r(SseOpcode::Ptest, RegMem::reg(src), src));
            // If the ZF is not set, place a 1 in `dst`.
            ctx.emit(Inst::setcc(CC::NZ, dst));
        }

        Opcode::VallTrue => {
            let dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let src_ty = ctx.input_ty(insn, 0);
            assert_eq!(src_ty.bits(), 128);
            let src = input_to_reg_mem(ctx, inputs[0]);

            let eq = |ty: Type| match ty.lane_bits() {
                8 => SseOpcode::Pcmpeqb,
                16 => SseOpcode::Pcmpeqw,
                32 => SseOpcode::Pcmpeqd,
                64 => SseOpcode::Pcmpeqq,
                _ => panic!("Unable to find an instruction for {} for type: {}", op, ty),
            };

            // Initialize a register with all 0s.
            let tmp = ctx.alloc_tmp(src_ty).only_reg().unwrap();
            ctx.emit(Inst::xmm_rm_r(SseOpcode::Pxor, RegMem::from(tmp), tmp));
            // Compare to see what lanes are filled with all 1s.
            ctx.emit(Inst::xmm_rm_r(eq(src_ty), src, tmp));
            // Set the ZF if the result is all zeroes.
            ctx.emit(Inst::xmm_cmp_rm_r(
                SseOpcode::Ptest,
                RegMem::from(tmp),
                tmp.to_reg(),
            ));
            // If the ZF is set, place a 1 in `dst`.
            ctx.emit(Inst::setcc(CC::Z, dst));
        }

        Opcode::VhighBits => {
            let src = put_input_in_reg(ctx, inputs[0]);
            let src_ty = ctx.input_ty(insn, 0);
            debug_assert!(src_ty.is_vector() && src_ty.bits() == 128);
            let dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            debug_assert!(dst.to_reg().class() == RegClass::Int);

            // The Intel specification allows using both 32-bit and 64-bit GPRs as destination for
            // the "move mask" instructions. This is controlled by the REX.R bit: "In 64-bit mode,
            // the instruction can access additional registers when used with a REX.R prefix. The
            // default operand size is 64-bit in 64-bit mode" (PMOVMSKB in IA Software Development
            // Manual, vol. 2). This being the case, we will always clear REX.W since its use is
            // unnecessary (`OperandSize` is used for setting/clearing REX.W).
            let size = OperandSize::Size32;

            match src_ty {
                types::I8X16 | types::B8X16 => {
                    ctx.emit(Inst::xmm_to_gpr(SseOpcode::Pmovmskb, src, dst, size))
                }
                types::I32X4 | types::B32X4 | types::F32X4 => {
                    ctx.emit(Inst::xmm_to_gpr(SseOpcode::Movmskps, src, dst, size))
                }
                types::I64X2 | types::B64X2 | types::F64X2 => {
                    ctx.emit(Inst::xmm_to_gpr(SseOpcode::Movmskpd, src, dst, size))
                }
                types::I16X8 | types::B16X8 => {
                    // There is no x86 instruction for extracting the high bit of 16-bit lanes so
                    // here we:
                    // - duplicate the 16-bit lanes of `src` into 8-bit lanes:
                    //     PACKSSWB([x1, x2, ...], [x1, x2, ...]) = [x1', x2', ..., x1', x2', ...]
                    // - use PMOVMSKB to gather the high bits; now we have duplicates, though
                    // - shift away the bottom 8 high bits to remove the duplicates.
                    let tmp = ctx.alloc_tmp(src_ty).only_reg().unwrap();
                    ctx.emit(Inst::gen_move(tmp, src, src_ty));
                    ctx.emit(Inst::xmm_rm_r(SseOpcode::Packsswb, RegMem::reg(src), tmp));
                    ctx.emit(Inst::xmm_to_gpr(
                        SseOpcode::Pmovmskb,
                        tmp.to_reg(),
                        dst,
                        size,
                    ));
                    ctx.emit(Inst::shift_r(
                        OperandSize::Size64,
                        ShiftKind::ShiftRightLogical,
                        Some(8),
                        dst,
                    ));
                }
                _ => unimplemented!("unknown input type {} for {}", src_ty, op),
            }
        }

        Opcode::Iconcat => {
            let ty = ctx.output_ty(insn, 0);
            assert_eq!(
                ty,
                types::I128,
                "Iconcat not expected to be used for non-128-bit type"
            );
            assert_eq!(ctx.input_ty(insn, 0), types::I64);
            assert_eq!(ctx.input_ty(insn, 1), types::I64);
            let lo = put_input_in_reg(ctx, inputs[0]);
            let hi = put_input_in_reg(ctx, inputs[1]);
            let dst = get_output_reg(ctx, outputs[0]);
            ctx.emit(Inst::gen_move(dst.regs()[0], lo, types::I64));
            ctx.emit(Inst::gen_move(dst.regs()[1], hi, types::I64));
        }

        Opcode::Isplit => {
            let ty = ctx.input_ty(insn, 0);
            assert_eq!(
                ty,
                types::I128,
                "Isplit not expected to be used for non-128-bit type"
            );
            assert_eq!(ctx.output_ty(insn, 0), types::I64);
            assert_eq!(ctx.output_ty(insn, 1), types::I64);
            let src = put_input_in_regs(ctx, inputs[0]);
            let dst_lo = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let dst_hi = get_output_reg(ctx, outputs[1]).only_reg().unwrap();
            ctx.emit(Inst::gen_move(dst_lo, src.regs()[0], types::I64));
            ctx.emit(Inst::gen_move(dst_hi, src.regs()[1], types::I64));
        }

        Opcode::TlsValue => {
            let dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let (name, _, _) = ctx.symbol_value(insn).unwrap();
            let symbol = name.clone();

            match flags.tls_model() {
                TlsModel::ElfGd => {
                    ctx.emit(Inst::ElfTlsGetAddr { symbol });
                    ctx.emit(Inst::gen_move(dst, regs::rax(), types::I64));
                }
                TlsModel::Macho => {
                    ctx.emit(Inst::MachOTlsGetAddr { symbol });
                    ctx.emit(Inst::gen_move(dst, regs::rax(), types::I64));
                }
                TlsModel::Coff => {
                    ctx.emit(Inst::CoffTlsGetAddr { symbol });
                    ctx.emit(Inst::gen_move(dst, regs::rax(), types::I64));
                }
                _ => todo!(
                    "Unimplemented TLS model in x64 backend: {:?}",
                    flags.tls_model()
                ),
            }
        }

        Opcode::SqmulRoundSat => {
            // Lane-wise saturating rounding multiplication in Q15 format
            // Optimal lowering taken from instruction proposal https://github.com/WebAssembly/simd/pull/365
            // y = i16x8.q15mulr_sat_s(a, b) is lowered to:
            //MOVDQA xmm_y, xmm_a
            //MOVDQA xmm_tmp, wasm_i16x8_splat(0x8000)
            //PMULHRSW xmm_y, xmm_b
            //PCMPEQW xmm_tmp, xmm_y
            //PXOR xmm_y, xmm_tmp
            let input_ty = ctx.input_ty(insn, 0);
            let src1 = put_input_in_reg(ctx, inputs[0]);
            let src2 = put_input_in_reg(ctx, inputs[1]);
            let dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();

            ctx.emit(Inst::gen_move(dst, src1, input_ty));
            static SAT_MASK: [u8; 16] = [
                0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80,
                0x00, 0x80,
            ];
            let mask_const = ctx.use_constant(VCodeConstantData::WellKnown(&SAT_MASK));
            let mask = ctx.alloc_tmp(types::I16X8).only_reg().unwrap();
            ctx.emit(Inst::xmm_load_const(mask_const, mask, types::I16X8));

            ctx.emit(Inst::xmm_rm_r(SseOpcode::Pmulhrsw, RegMem::reg(src2), dst));
            ctx.emit(Inst::xmm_rm_r(
                SseOpcode::Pcmpeqw,
                RegMem::reg(dst.to_reg()),
                mask,
            ));
            ctx.emit(Inst::xmm_rm_r(
                SseOpcode::Pxor,
                RegMem::reg(mask.to_reg()),
                dst,
            ));
        }

        Opcode::Uunarrow => {
            if let Some(fcvt_inst) = matches_input(ctx, inputs[0], Opcode::FcvtToUintSat) {
                //y = i32x4.trunc_sat_f64x2_u_zero(x) is lowered to:
                //MOVAPD xmm_y, xmm_x
                //XORPD xmm_tmp, xmm_tmp
                //MAXPD xmm_y, xmm_tmp
                //MINPD xmm_y, [wasm_f64x2_splat(4294967295.0)]
                //ROUNDPD xmm_y, xmm_y, 0x0B
                //ADDPD xmm_y, [wasm_f64x2_splat(0x1.0p+52)]
                //SHUFPS xmm_y, xmm_xmp, 0x88

                let fcvt_input = InsnInput {
                    insn: fcvt_inst,
                    input: 0,
                };
                let input_ty = ctx.input_ty(fcvt_inst, 0);
                let output_ty = ctx.output_ty(insn, 0);
                let src = put_input_in_reg(ctx, fcvt_input);
                let dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();

                ctx.emit(Inst::gen_move(dst, src, input_ty));
                let tmp1 = ctx.alloc_tmp(output_ty).only_reg().unwrap();
                ctx.emit(Inst::xmm_rm_r(SseOpcode::Xorpd, RegMem::from(tmp1), tmp1));
                ctx.emit(Inst::xmm_rm_r(SseOpcode::Maxpd, RegMem::from(tmp1), dst));

                // 4294967295.0 is equivalent to 0x41EFFFFFFFE00000
                static UMAX_MASK: [u8; 16] = [
                    0x00, 0x00, 0xE0, 0xFF, 0xFF, 0xFF, 0xEF, 0x41, 0x00, 0x00, 0xE0, 0xFF, 0xFF,
                    0xFF, 0xEF, 0x41,
                ];
                let umax_const = ctx.use_constant(VCodeConstantData::WellKnown(&UMAX_MASK));
                let umax_mask = ctx.alloc_tmp(types::F64X2).only_reg().unwrap();
                ctx.emit(Inst::xmm_load_const(umax_const, umax_mask, types::F64X2));

                //MINPD xmm_y, [wasm_f64x2_splat(4294967295.0)]
                ctx.emit(Inst::xmm_rm_r(
                    SseOpcode::Minpd,
                    RegMem::from(umax_mask),
                    dst,
                ));
                //ROUNDPD xmm_y, xmm_y, 0x0B
                ctx.emit(Inst::xmm_rm_r_imm(
                    SseOpcode::Roundpd,
                    RegMem::reg(dst.to_reg()),
                    dst,
                    RoundImm::RoundZero.encode(),
                    OperandSize::Size32,
                ));
                //ADDPD xmm_y, [wasm_f64x2_splat(0x1.0p+52)]
                static UINT_MASK: [u8; 16] = [
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x30, 0x43,
                ];
                let uint_mask_const = ctx.use_constant(VCodeConstantData::WellKnown(&UINT_MASK));
                let uint_mask = ctx.alloc_tmp(types::F64X2).only_reg().unwrap();
                ctx.emit(Inst::xmm_load_const(
                    uint_mask_const,
                    uint_mask,
                    types::F64X2,
                ));
                ctx.emit(Inst::xmm_rm_r(
                    SseOpcode::Addpd,
                    RegMem::from(uint_mask),
                    dst,
                ));

                //SHUFPS xmm_y, xmm_xmp, 0x88
                ctx.emit(Inst::xmm_rm_r_imm(
                    SseOpcode::Shufps,
                    RegMem::reg(tmp1.to_reg()),
                    dst,
                    0x88,
                    OperandSize::Size32,
                ));
            } else {
                println!("Did not match fcvt input!");
            }
        }

        // Unimplemented opcodes below. These are not currently used by Wasm
        // lowering or other known embeddings, but should be either supported or
        // removed eventually
        Opcode::ExtractVector => {
            unimplemented!("ExtractVector not supported");
        }

        Opcode::Cls => unimplemented!("Cls not supported"),

        Opcode::BorNot | Opcode::BxorNot => {
            unimplemented!("or-not / xor-not opcodes not implemented");
        }

        Opcode::Bmask => unimplemented!("Bmask not implemented"),

        Opcode::Trueif | Opcode::Trueff => unimplemented!("trueif / trueff not implemented"),

        Opcode::ConstAddr => unimplemented!("ConstAddr not implemented"),

        Opcode::Vsplit | Opcode::Vconcat => {
            unimplemented!("Vector split/concat ops not implemented.");
        }

        // Opcodes that should be removed by legalization. These should
        // eventually be removed if/when we replace in-situ legalization with
        // something better.
        Opcode::Ifcmp | Opcode::Ffcmp => {
            panic!("Should never reach ifcmp/ffcmp as isel root!");
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
            panic!("ALU+imm and ALU+carry ops should not appear here!");
        }

        Opcode::StackLoad
        | Opcode::StackStore
        | Opcode::DynamicStackStore
        | Opcode::DynamicStackLoad => {
            panic!("Direct stack memory access not supported; should have been legalized");
        }

        Opcode::GlobalValue => {
            panic!("global_value should have been removed by legalization!");
        }

        Opcode::HeapAddr => {
            panic!("heap_addr should have been removed by legalization!");
        }

        Opcode::TableAddr => {
            panic!("table_addr should have been removed by legalization!");
        }

        Opcode::Copy => {
            panic!("Unused opcode should not be encountered.");
        }

        Opcode::Trapz | Opcode::Trapnz | Opcode::ResumableTrapnz => {
            panic!("trapz / trapnz / resumable_trapnz should have been removed by legalization!");
        }

        Opcode::Jump
        | Opcode::Brz
        | Opcode::Brnz
        | Opcode::BrIcmp
        | Opcode::Brif
        | Opcode::Brff
        | Opcode::BrTable => {
            panic!("Branch opcode reached non-branch lowering logic!");
        }

        Opcode::Nop => {
            // Nothing.
        }
    }

    Ok(())
}

//=============================================================================
// Lowering-backend trait implementation.

impl LowerBackend for X64Backend {
    type MInst = Inst;

    fn lower(&self, ctx: &mut Lower<Inst>, ir_inst: IRInst) -> CodegenResult<()> {
        lower_insn_to_regs(ctx, ir_inst, &self.flags, &self.x64_flags, &self.triple)
    }

    fn lower_branch_group(
        &self,
        ctx: &mut Lower<Inst>,
        branches: &[IRInst],
        targets: &[MachLabel],
    ) -> CodegenResult<()> {
        // A block should end with at most two branches. The first may be a
        // conditional branch; a conditional branch can be followed only by an
        // unconditional branch or fallthrough. Otherwise, if only one branch,
        // it may be an unconditional branch, a fallthrough, a return, or a
        // trap. These conditions are verified by `is_ebb_basic()` during the
        // verifier pass.
        assert!(branches.len() <= 2);
        if branches.len() == 2 {
            let op1 = ctx.data(branches[1]).opcode();
            assert!(op1 == Opcode::Jump);
        }

        if let Ok(()) = isle::lower_branch(
            ctx,
            &self.triple,
            &self.flags,
            &self.x64_flags,
            branches[0],
            targets,
        ) {
            return Ok(());
        }

        unreachable!(
            "implemented in ISLE: branch = `{}`",
            ctx.dfg().display_inst(branches[0]),
        );
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        Some(regs::pinned_reg())
    }
}
