use crate::config::Config;
use crate::cranelift_arbitrary::CraneliftArbitrary;
use crate::target_isa_extras::TargetIsaExtras;
use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use cranelift::codegen::data_value::DataValue;
use cranelift::codegen::ir::immediates::Offset32;
use cranelift::codegen::ir::instructions::{InstructionFormat, ResolvedConstraint};
use cranelift::codegen::ir::stackslot::StackSize;

use cranelift::codegen::ir::{
    types::*, AliasRegion, AtomicRmwOp, Block, ConstantData, Endianness, ExternalName, FuncRef,
    Function, LibCall, Opcode, SigRef, Signature, StackSlot, UserExternalName, UserFuncName, Value,
};
use cranelift::codegen::isa::CallConv;
use cranelift::frontend::{FunctionBuilder, FunctionBuilderContext, Switch, Variable};
use cranelift::prelude::isa::OwnedTargetIsa;
use cranelift::prelude::{
    EntityRef, ExtFuncData, FloatCC, InstBuilder, IntCC, JumpTableData, MemFlags, StackSlotData,
    StackSlotKind,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::str::FromStr;
use target_lexicon::{Architecture, Triple};

type BlockSignature = Vec<Type>;

fn insert_opcode(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    args: &[Type],
    rets: &[Type],
) -> Result<()> {
    let mut vals = Vec::with_capacity(args.len());
    for &arg in args.into_iter() {
        let var = fgen.get_variable_of_type(arg)?;
        let val = builder.use_var(var);
        vals.push(val);
    }

    // Some opcodes require us to look at their input arguments to determine the
    // controlling type. This is not the general case, but we can neatly check this
    // using `requires_typevar_operand`.
    let ctrl_type = if opcode.constraints().requires_typevar_operand() {
        args.first()
    } else {
        rets.first()
    }
    .copied()
    .unwrap_or(INVALID);

    // Choose the appropriate instruction format for this opcode
    let (inst, dfg) = match opcode.format() {
        InstructionFormat::NullAry => builder.ins().NullAry(opcode, ctrl_type),
        InstructionFormat::Unary => builder.ins().Unary(opcode, ctrl_type, vals[0]),
        InstructionFormat::Binary => builder.ins().Binary(opcode, ctrl_type, vals[0], vals[1]),
        InstructionFormat::Ternary => builder
            .ins()
            .Ternary(opcode, ctrl_type, vals[0], vals[1], vals[2]),
        _ => unimplemented!(),
    };
    let results = dfg.inst_results(inst).to_vec();

    for (val, &ty) in results.into_iter().zip(rets) {
        let var = fgen.get_variable_of_type(ty)?;
        builder.def_var(var, val);
    }
    Ok(())
}

fn insert_call_to_function(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    call_opcode: Opcode,
    sig: &Signature,
    sig_ref: SigRef,
    func_ref: FuncRef,
) -> Result<()> {
    let actuals = fgen.generate_values_for_signature(
        builder,
        sig.params.iter().map(|abi_param| abi_param.value_type),
    )?;

    let addr_ty = fgen.isa.pointer_type();
    let call = match call_opcode {
        Opcode::Call => builder.ins().call(func_ref, &actuals),
        Opcode::ReturnCall => builder.ins().return_call(func_ref, &actuals),
        Opcode::CallIndirect => {
            let addr = builder.ins().func_addr(addr_ty, func_ref);
            builder.ins().call_indirect(sig_ref, addr, &actuals)
        }
        Opcode::ReturnCallIndirect => {
            let addr = builder.ins().func_addr(addr_ty, func_ref);
            builder.ins().return_call_indirect(sig_ref, addr, &actuals)
        }
        _ => unreachable!(),
    };

    // Assign the return values to random variables
    let ret_values = builder.inst_results(call).to_vec();
    let ret_types = sig.returns.iter().map(|p| p.value_type);
    for (ty, val) in ret_types.zip(ret_values) {
        let var = fgen.get_variable_of_type(ty)?;
        builder.def_var(var, val);
    }

    Ok(())
}

fn insert_call(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    _args: &[Type],
    _rets: &[Type],
) -> Result<()> {
    assert!(matches!(opcode, Opcode::Call | Opcode::CallIndirect));
    let (sig, sig_ref, func_ref) = fgen.u.choose(&fgen.resources.func_refs)?.clone();

    insert_call_to_function(fgen, builder, opcode, &sig, sig_ref, func_ref)
}

fn insert_stack_load(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    _opcode: Opcode,
    _args: &[Type],
    rets: &[Type],
) -> Result<()> {
    let typevar = rets[0];
    let type_size = typevar.bytes();
    let (slot, slot_size, _align, category) = fgen.stack_slot_with_size(type_size)?;

    // `stack_load` doesn't support setting MemFlags, and it does not set any
    // alias analysis bits, so we can only emit it for `Other` slots.
    if category != AACategory::Other {
        return Err(arbitrary::Error::IncorrectFormat.into());
    }

    let offset = fgen.u.int_in_range(0..=(slot_size - type_size))? as i32;

    let val = builder.ins().stack_load(typevar, slot, offset);
    let var = fgen.get_variable_of_type(typevar)?;
    builder.def_var(var, val);

    Ok(())
}

fn insert_stack_store(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    _opcode: Opcode,
    args: &[Type],
    _rets: &[Type],
) -> Result<()> {
    let typevar = args[0];
    let type_size = typevar.bytes();

    let (slot, slot_size, _align, category) = fgen.stack_slot_with_size(type_size)?;

    // `stack_store` doesn't support setting MemFlags, and it does not set any
    // alias analysis bits, so we can only emit it for `Other` slots.
    if category != AACategory::Other {
        return Err(arbitrary::Error::IncorrectFormat.into());
    }

    let offset = fgen.u.int_in_range(0..=(slot_size - type_size))? as i32;

    let arg0 = fgen.get_variable_of_type(typevar)?;
    let arg0 = builder.use_var(arg0);

    builder.ins().stack_store(arg0, slot, offset);
    Ok(())
}

fn insert_cmp(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    args: &[Type],
    rets: &[Type],
) -> Result<()> {
    let lhs = fgen.get_variable_of_type(args[0])?;
    let lhs = builder.use_var(lhs);

    let rhs = fgen.get_variable_of_type(args[1])?;
    let rhs = builder.use_var(rhs);

    let res = if opcode == Opcode::Fcmp {
        let cc = *fgen.u.choose(FloatCC::all())?;

        // We filter out condition codes that aren't supported by the target at
        // this point after randomly choosing one, instead of randomly choosing a
        // supported one, to avoid invalidating the corpus when these get implemented.
        let unimplemented_cc = match (fgen.isa.triple().architecture, cc) {
            // Some FloatCC's are not implemented on AArch64, see:
            // https://github.com/bytecodealliance/wasmtime/issues/4850
            (Architecture::Aarch64(_), FloatCC::OrderedNotEqual) => true,
            (Architecture::Aarch64(_), FloatCC::UnorderedOrEqual) => true,
            (Architecture::Aarch64(_), FloatCC::UnorderedOrLessThan) => true,
            (Architecture::Aarch64(_), FloatCC::UnorderedOrLessThanOrEqual) => true,
            (Architecture::Aarch64(_), FloatCC::UnorderedOrGreaterThan) => true,
            (Architecture::Aarch64(_), FloatCC::UnorderedOrGreaterThanOrEqual) => true,

            // These are not implemented on x86_64, for vectors.
            (Architecture::X86_64, FloatCC::UnorderedOrEqual | FloatCC::OrderedNotEqual) => {
                args[0].is_vector()
            }
            _ => false,
        };
        if unimplemented_cc {
            return Err(arbitrary::Error::IncorrectFormat.into());
        }

        builder.ins().fcmp(cc, lhs, rhs)
    } else {
        let cc = *fgen.u.choose(IntCC::all())?;
        builder.ins().icmp(cc, lhs, rhs)
    };

    let var = fgen.get_variable_of_type(rets[0])?;
    builder.def_var(var, res);

    Ok(())
}

fn insert_const(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    _opcode: Opcode,
    _args: &[Type],
    rets: &[Type],
) -> Result<()> {
    let typevar = rets[0];
    let var = fgen.get_variable_of_type(typevar)?;
    let val = fgen.generate_const(builder, typevar)?;
    builder.def_var(var, val);
    Ok(())
}

fn insert_bitcast(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    args: &[Type],
    rets: &[Type],
) -> Result<()> {
    let from_var = fgen.get_variable_of_type(args[0])?;
    let from_val = builder.use_var(from_var);

    let to_var = fgen.get_variable_of_type(rets[0])?;

    // TODO: We can generate little/big endian flags here.
    let mut memflags = MemFlags::new();

    // When bitcasting between vectors of different lane counts, we need to
    // specify the endianness.
    if args[0].lane_count() != rets[0].lane_count() {
        memflags.set_endianness(Endianness::Little);
    }

    let res = builder.ins().bitcast(rets[0], memflags, from_val);
    builder.def_var(to_var, res);
    Ok(())
}

fn insert_load_store(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    args: &[Type],
    rets: &[Type],
) -> Result<()> {
    if opcode == Opcode::Bitcast {
        return insert_bitcast(fgen, builder, args, rets);
    }

    let ctrl_type = *rets.first().or(args.first()).unwrap();
    let type_size = ctrl_type.bytes();

    let is_atomic = [Opcode::AtomicLoad, Opcode::AtomicStore].contains(&opcode);
    let (address, flags, offset) =
        fgen.generate_address_and_memflags(builder, type_size, is_atomic)?;

    // The variable being loaded or stored into
    let var = fgen.get_variable_of_type(ctrl_type)?;

    match opcode.format() {
        InstructionFormat::LoadNoOffset => {
            let (inst, dfg) = builder
                .ins()
                .LoadNoOffset(opcode, ctrl_type, flags, address);

            let new_val = dfg.first_result(inst);
            builder.def_var(var, new_val);
        }
        InstructionFormat::StoreNoOffset => {
            let val = builder.use_var(var);

            builder
                .ins()
                .StoreNoOffset(opcode, ctrl_type, flags, val, address);
        }
        InstructionFormat::Store => {
            let val = builder.use_var(var);

            builder
                .ins()
                .Store(opcode, ctrl_type, flags, offset, val, address);
        }
        InstructionFormat::Load => {
            let (inst, dfg) = builder
                .ins()
                .Load(opcode, ctrl_type, flags, offset, address);

            let new_val = dfg.first_result(inst);
            builder.def_var(var, new_val);
        }
        _ => unimplemented!(),
    }

    Ok(())
}

fn insert_atomic_rmw(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    _: Opcode,
    _: &[Type],
    rets: &[Type],
) -> Result<()> {
    let ctrl_type = *rets.first().unwrap();
    let type_size = ctrl_type.bytes();

    let rmw_op = *fgen.u.choose(AtomicRmwOp::all())?;

    let (address, flags, offset) = fgen.generate_address_and_memflags(builder, type_size, true)?;

    // AtomicRMW does not directly support offsets, so add the offset to the address separately.
    let address = builder.ins().iadd_imm(address, i64::from(offset));

    // Load and store target variables
    let source_var = fgen.get_variable_of_type(ctrl_type)?;
    let target_var = fgen.get_variable_of_type(ctrl_type)?;

    let source_val = builder.use_var(source_var);
    let new_val = builder
        .ins()
        .atomic_rmw(ctrl_type, flags, rmw_op, address, source_val);

    builder.def_var(target_var, new_val);
    Ok(())
}

fn insert_atomic_cas(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    _: Opcode,
    _: &[Type],
    rets: &[Type],
) -> Result<()> {
    let ctrl_type = *rets.first().unwrap();
    let type_size = ctrl_type.bytes();

    let (address, flags, offset) = fgen.generate_address_and_memflags(builder, type_size, true)?;

    // AtomicCas does not directly support offsets, so add the offset to the address separately.
    let address = builder.ins().iadd_imm(address, i64::from(offset));

    // Source and Target variables
    let expected_var = fgen.get_variable_of_type(ctrl_type)?;
    let store_var = fgen.get_variable_of_type(ctrl_type)?;
    let loaded_var = fgen.get_variable_of_type(ctrl_type)?;

    let expected_val = builder.use_var(expected_var);
    let store_val = builder.use_var(store_var);
    let new_val = builder
        .ins()
        .atomic_cas(flags, address, expected_val, store_val);

    builder.def_var(loaded_var, new_val);
    Ok(())
}

fn insert_shuffle(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    _: &[Type],
    rets: &[Type],
) -> Result<()> {
    let ctrl_type = *rets.first().unwrap();

    let lhs = builder.use_var(fgen.get_variable_of_type(ctrl_type)?);
    let rhs = builder.use_var(fgen.get_variable_of_type(ctrl_type)?);

    let mask = {
        let mut lanes = [0u8; 16];
        for lane in lanes.iter_mut() {
            *lane = fgen.u.int_in_range(0..=31)?;
        }
        let lanes = ConstantData::from(lanes.as_ref());
        builder.func.dfg.immediates.push(lanes)
    };

    // This function is called for any `InstructionFormat::Shuffle`. Which today is just
    // `shuffle`, but lets assert that, just to be sure we don't accidentally insert
    // something else.
    assert_eq!(opcode, Opcode::Shuffle);
    let res = builder.ins().shuffle(lhs, rhs, mask);

    let target_var = fgen.get_variable_of_type(ctrl_type)?;
    builder.def_var(target_var, res);

    Ok(())
}

fn insert_ins_ext_lane(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    args: &[Type],
    rets: &[Type],
) -> Result<()> {
    let vector_type = *args.first().unwrap();
    let ret_type = *rets.first().unwrap();

    let lhs = builder.use_var(fgen.get_variable_of_type(vector_type)?);
    let max_lane = (vector_type.lane_count() as u8) - 1;
    let lane = fgen.u.int_in_range(0..=max_lane)?;

    let res = match opcode {
        Opcode::Insertlane => {
            let rhs = builder.use_var(fgen.get_variable_of_type(args[1])?);
            builder.ins().insertlane(lhs, rhs, lane)
        }
        Opcode::Extractlane => builder.ins().extractlane(lhs, lane),
        _ => todo!(),
    };

    let target_var = fgen.get_variable_of_type(ret_type)?;
    builder.def_var(target_var, res);

    Ok(())
}

type OpcodeInserter = fn(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    Opcode,
    &[Type],
    &[Type],
) -> Result<()>;

macro_rules! exceptions {
    ($op:expr, $args:expr, $rets:expr, $(($($cases:pat),*)),* $(,)?) => {
        match ($op, $args, $rets) {
            $( ($($cases,)* ..) => return false, )*
            _ => true,
        }
    }
}

/// Returns true if we believe this `OpcodeSignature` should compile correctly
/// for the given target triple. We currently have a range of known issues
/// with specific lowerings on specific backends, and we don't want to get
/// fuzz bug reports for those. Over time our goal is to eliminate all of these
/// exceptions.
fn valid_for_target(triple: &Triple, op: Opcode, args: &[Type], rets: &[Type]) -> bool {
    // Rule out invalid combinations that we don't yet have a good way of rejecting with the
    // instruction DSL type constraints.
    match op {
        Opcode::FcvtToUintSat | Opcode::FcvtToSintSat => {
            assert_eq!(args.len(), 1);
            assert_eq!(rets.len(), 1);

            let arg = args[0];
            let ret = rets[0];

            // Vector arguments must produce vector results, and scalar arguments must produce
            // scalar results.
            if arg.is_vector() != ret.is_vector() {
                return false;
            }

            if arg.is_vector() && ret.is_vector() {
                // Vector conversions must have the same number of lanes, and the lanes must be the
                // same bit-width.
                if arg.lane_count() != ret.lane_count() {
                    return false;
                }

                if arg.lane_of().bits() != ret.lane_of().bits() {
                    return false;
                }
            }
        }

        Opcode::Bitcast => {
            assert_eq!(args.len(), 1);
            assert_eq!(rets.len(), 1);

            let arg = args[0];
            let ret = rets[0];

            // The opcode generator still allows bitcasts between different sized types, but these
            // are rejected in the verifier.
            if arg.bits() != ret.bits() {
                return false;
            }
        }

        _ => {}
    }

    match triple.architecture {
        Architecture::X86_64 => {
            exceptions!(
                op,
                args,
                rets,
                (Opcode::UmulOverflow | Opcode::SmulOverflow, &[I128, I128]),
                (Opcode::Imul, &[I8X16, I8X16]),
                // https://github.com/bytecodealliance/wasmtime/issues/4756
                (Opcode::Udiv | Opcode::Sdiv, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5474
                (Opcode::Urem | Opcode::Srem, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/3370
                (
                    Opcode::Smin | Opcode::Umin | Opcode::Smax | Opcode::Umax,
                    &[I128, I128]
                ),
                // https://github.com/bytecodealliance/wasmtime/issues/5107
                (Opcode::Cls, &[I8], &[I8]),
                (Opcode::Cls, &[I16], &[I16]),
                (Opcode::Cls, &[I32], &[I32]),
                (Opcode::Cls, &[I64], &[I64]),
                (Opcode::Cls, &[I128], &[I128]),
                // TODO
                (Opcode::Bitselect, &[_, _, _], &[F32 | F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4897
                // https://github.com/bytecodealliance/wasmtime/issues/4899
                (
                    Opcode::FcvtToUint
                        | Opcode::FcvtToUintSat
                        | Opcode::FcvtToSint
                        | Opcode::FcvtToSintSat,
                    &[F32 | F64],
                    &[I8 | I16 | I128]
                ),
                (Opcode::FcvtToUint | Opcode::FcvtToSint, &[F32X4], &[I32X4]),
                (
                    Opcode::FcvtToUint
                        | Opcode::FcvtToUintSat
                        | Opcode::FcvtToSint
                        | Opcode::FcvtToSintSat,
                    &[F64X2],
                    &[I64X2]
                ),
                // https://github.com/bytecodealliance/wasmtime/issues/4900
                (Opcode::FcvtFromUint, &[I128], &[F32 | F64]),
                // This has a lowering, but only when preceded by `uwiden_low`.
                (Opcode::FcvtFromUint, &[I64X2], &[F64X2]),
                // https://github.com/bytecodealliance/wasmtime/issues/4900
                (Opcode::FcvtFromSint, &[I128], &[F32 | F64]),
                (Opcode::FcvtFromSint, &[I64X2], &[F64X2]),
                (
                    Opcode::Umulhi | Opcode::Smulhi,
                    &([I8X16, I8X16] | [I16X8, I16X8] | [I32X4, I32X4] | [I64X2, I64X2])
                ),
                (
                    Opcode::UaddSat | Opcode::SaddSat | Opcode::UsubSat | Opcode::SsubSat,
                    &([I32X4, I32X4] | [I64X2, I64X2])
                ),
                (Opcode::Fcopysign, &([F32X4, F32X4] | [F64X2, F64X2])),
                (Opcode::Popcnt, &([I8X16] | [I16X8] | [I32X4] | [I64X2])),
                (
                    Opcode::Umax | Opcode::Smax | Opcode::Umin | Opcode::Smin,
                    &[I64X2, I64X2]
                ),
                // https://github.com/bytecodealliance/wasmtime/issues/6104
                (Opcode::Bitcast, &[I128], &[_]),
                (Opcode::Bitcast, &[_], &[I128]),
                (Opcode::Uunarrow),
                (Opcode::Snarrow | Opcode::Unarrow, &[I64X2, I64X2]),
                (Opcode::SqmulRoundSat, &[I32X4, I32X4]),
                // This Icmp is not implemented: #5529
                (Opcode::Icmp, &[I64X2, I64X2]),
                // IaddPairwise is implemented, but only for some types, and with some preceding ops.
                (Opcode::IaddPairwise),
                // Nothing wrong with this select. But we have an isle rule that can optimize it
                // into a `min`/`max` instructions, which we don't have implemented yet.
                (Opcode::Select, &[_, I128, I128]),
                // These stack accesses can cause segfaults if they are merged into an SSE instruction.
                // See: #5922
                (
                    Opcode::StackStore,
                    &[I8X16 | I16X8 | I32X4 | I64X2 | F32X4 | F64X2]
                ),
                (
                    Opcode::StackLoad,
                    &[],
                    &[I8X16 | I16X8 | I32X4 | I64X2 | F32X4 | F64X2]
                ),
                // TODO
                (
                    Opcode::Sshr | Opcode::Ushr | Opcode::Ishl,
                    &[I8X16 | I16X8 | I32X4 | I64X2, I128]
                ),
                (
                    Opcode::Rotr | Opcode::Rotl,
                    &[I8X16 | I16X8 | I32X4 | I64X2, _]
                ),
            )
        }

        Architecture::Aarch64(_) => {
            exceptions!(
                op,
                args,
                rets,
                (Opcode::UmulOverflow | Opcode::SmulOverflow, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4864
                (Opcode::Udiv | Opcode::Sdiv, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5472
                (Opcode::Urem | Opcode::Srem, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4313
                (
                    Opcode::Smin | Opcode::Umin | Opcode::Smax | Opcode::Umax,
                    &[I128, I128]
                ),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::Bnot, &[F32 | F64]),
                (
                    Opcode::Band
                        | Opcode::Bor
                        | Opcode::Bxor
                        | Opcode::BandNot
                        | Opcode::BorNot
                        | Opcode::BxorNot,
                    &([F32, F32] | [F64, F64])
                ),
                // https://github.com/bytecodealliance/wasmtime/issues/5198
                (Opcode::Bitselect, &[I128, I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4934
                (
                    Opcode::FcvtToUint
                        | Opcode::FcvtToUintSat
                        | Opcode::FcvtToSint
                        | Opcode::FcvtToSintSat,
                    &[F32 | F64],
                    &[I128]
                ),
                // https://github.com/bytecodealliance/wasmtime/issues/4933
                (
                    Opcode::FcvtFromUint | Opcode::FcvtFromSint,
                    &[I128],
                    &[F32 | F64]
                ),
                (
                    Opcode::Umulhi | Opcode::Smulhi,
                    &([I8X16, I8X16] | [I16X8, I16X8] | [I32X4, I32X4] | [I64X2, I64X2])
                ),
                (Opcode::Popcnt, &[I16X8 | I32X4 | I64X2]),
                // Nothing wrong with this select. But we have an isle rule that can optimize it
                // into a `min`/`max` instructions, which we don't have implemented yet.
                (Opcode::Select, &[I8, I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/6104
                (Opcode::Bitcast, &[I128], &[_]),
                (Opcode::Bitcast, &[_], &[I128]),
                // TODO
                (
                    Opcode::Sshr | Opcode::Ushr | Opcode::Ishl,
                    &[I8X16 | I16X8 | I32X4 | I64X2, I128]
                ),
                (
                    Opcode::Rotr | Opcode::Rotl,
                    &[I8X16 | I16X8 | I32X4 | I64X2, _]
                ),
                // TODO
                (Opcode::Bitselect, &[_, _, _], &[F32 | F64]),
                (Opcode::VhighBits, &[F32X4 | F64X2]),
            )
        }

        Architecture::S390x => {
            exceptions!(
                op,
                args,
                rets,
                (Opcode::UaddOverflow | Opcode::SaddOverflow),
                (Opcode::UsubOverflow | Opcode::SsubOverflow),
                (Opcode::UmulOverflow | Opcode::SmulOverflow),
                (
                    Opcode::Udiv | Opcode::Sdiv | Opcode::Urem | Opcode::Srem,
                    &[I128, I128]
                ),
                (Opcode::Bnot, &[F32 | F64]),
                (
                    Opcode::Band
                        | Opcode::Bor
                        | Opcode::Bxor
                        | Opcode::BandNot
                        | Opcode::BorNot
                        | Opcode::BxorNot,
                    &([F32, F32] | [F64, F64])
                ),
                (
                    Opcode::FcvtToUint
                        | Opcode::FcvtToUintSat
                        | Opcode::FcvtToSint
                        | Opcode::FcvtToSintSat,
                    &[F32 | F64],
                    &[I128]
                ),
                (
                    Opcode::FcvtFromUint | Opcode::FcvtFromSint,
                    &[I128],
                    &[F32 | F64]
                ),
                (Opcode::SsubSat | Opcode::SaddSat, &[I64X2, I64X2]),
                // https://github.com/bytecodealliance/wasmtime/issues/6104
                (Opcode::Bitcast, &[I128], &[_]),
                (Opcode::Bitcast, &[_], &[I128]),
                // TODO
                (Opcode::Bitselect, &[_, _, _], &[F32 | F64]),
            )
        }

        Architecture::Riscv64(_) => {
            exceptions!(
                op,
                args,
                rets,
                // TODO
                (Opcode::UaddOverflow | Opcode::SaddOverflow),
                (Opcode::UsubOverflow | Opcode::SsubOverflow),
                (Opcode::UmulOverflow | Opcode::SmulOverflow),
                // TODO
                (
                    Opcode::Udiv | Opcode::Sdiv | Opcode::Urem | Opcode::Srem,
                    &[I128, I128]
                ),
                // TODO
                (Opcode::Iabs, &[I128]),
                // TODO
                (Opcode::Bitselect, &[I128, I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5528
                (
                    Opcode::FcvtToUint | Opcode::FcvtToSint,
                    [F32 | F64],
                    &[I128]
                ),
                (
                    Opcode::FcvtToUintSat | Opcode::FcvtToSintSat,
                    &[F32 | F64],
                    &[I128]
                ),
                // https://github.com/bytecodealliance/wasmtime/issues/5528
                (
                    Opcode::FcvtFromUint | Opcode::FcvtFromSint,
                    &[I128],
                    &[F32 | F64]
                ),
                // TODO
                (
                    Opcode::SelectSpectreGuard,
                    &[_, _, _],
                    &[F32 | F64 | I8X16 | I16X8 | I32X4 | I64X2 | F64X2 | F32X4]
                ),
                // TODO
                (Opcode::Bitselect, &[_, _, _], &[F32 | F64]),
                (
                    Opcode::Rotr | Opcode::Rotl,
                    &[I8X16 | I16X8 | I32X4 | I64X2, _]
                ),
            )
        }

        _ => true,
    }
}

type OpcodeSignature = (Opcode, Vec<Type>, Vec<Type>);

static OPCODE_SIGNATURES: Lazy<Vec<OpcodeSignature>> = Lazy::new(|| {
    let types = &[
        I8, I16, I32, I64, I128, // Scalar Integers
        F32, F64, // Scalar Floats
        I8X16, I16X8, I32X4, I64X2, // SIMD Integers
        F32X4, F64X2, // SIMD Floats
    ];

    // When this env variable is passed, we only generate instructions for the opcodes listed in
    // the comma-separated list. This is useful for debugging, as it allows us to focus on a few
    // specific opcodes.
    let allowed_opcodes = std::env::var("FUZZGEN_ALLOWED_OPS").ok().map(|s| {
        s.split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| Opcode::from_str(s).expect("Unrecoginzed opcode"))
            .collect::<Vec<_>>()
    });

    Opcode::all()
        .iter()
        .filter(|op| {
            match op {
                // Control flow opcodes should not be generated through `generate_instructions`.
                Opcode::BrTable
                | Opcode::Brif
                | Opcode::Jump
                | Opcode::Return
                | Opcode::ReturnCall
                | Opcode::ReturnCallIndirect => false,

                // Constants are generated outside of `generate_instructions`
                Opcode::Iconst => false,

                // TODO: extract_vector raises exceptions during return type generation because it
                // uses dynamic vectors.
                Opcode::ExtractVector => false,

                _ => true,
            }
        })
        .flat_map(|op| {
            let constraints = op.constraints();

            let ctrl_types = if let Some(ctrls) = constraints.ctrl_typeset() {
                Vec::from_iter(types.iter().copied().filter(|ty| ctrls.contains(*ty)))
            } else {
                vec![INVALID]
            };

            ctrl_types.into_iter().flat_map(move |ctrl_type| {
                let rets = Vec::from_iter(
                    (0..constraints.num_fixed_results())
                        .map(|i| constraints.result_type(i, ctrl_type)),
                );

                // Cols is a vector whose length will match `num_fixed_value_arguments`, and whose
                // elements will be vectors of types that are valid for that fixed argument
                // position.
                let mut cols = vec![];

                for i in 0..constraints.num_fixed_value_arguments() {
                    match constraints.value_argument_constraint(i, ctrl_type) {
                        ResolvedConstraint::Bound(ty) => cols.push(Vec::from([ty])),
                        ResolvedConstraint::Free(tys) => cols.push(Vec::from_iter(
                            types.iter().copied().filter(|ty| tys.contains(*ty)),
                        )),
                    }
                }

                // Generate the cartesian product of cols to produce a vector of argument lists,
                // argss. The argss vector is seeded with the empty argument list, so there's an
                // initial value to be extended in the loop below.
                let mut argss = vec![vec![]];
                let mut cols = cols.as_slice();
                while let Some((col, rest)) = cols.split_last() {
                    cols = rest;

                    let mut next = vec![];
                    for current in argss.iter() {
                        // Extend the front of each argument candidate with every type in `col`.
                        for ty in col {
                            let mut args = vec![*ty];
                            args.extend_from_slice(&current);
                            next.push(args);
                        }
                    }

                    let _ = std::mem::replace(&mut argss, next);
                }

                argss.into_iter().map(move |args| (*op, args, rets.clone()))
            })
        })
        .filter(|(op, args, rets)| {
            // These op/signature combinations need to be vetted
            exceptions!(
                op,
                args.as_slice(),
                rets.as_slice(),
                (Opcode::Debugtrap),
                (Opcode::Trap),
                (Opcode::Trapz),
                (Opcode::Trapnz),
                (Opcode::CallIndirect, &[I32]),
                (Opcode::FuncAddr),
                (Opcode::X86Pshufb),
                (Opcode::AvgRound),
                (Opcode::Uload8x8),
                (Opcode::Sload8x8),
                (Opcode::Uload16x4),
                (Opcode::Sload16x4),
                (Opcode::Uload32x2),
                (Opcode::Sload32x2),
                (Opcode::StackAddr),
                (Opcode::DynamicStackLoad),
                (Opcode::DynamicStackStore),
                (Opcode::DynamicStackAddr),
                (Opcode::GlobalValue),
                (Opcode::SymbolValue),
                (Opcode::TlsValue),
                (Opcode::GetPinnedReg),
                (Opcode::SetPinnedReg),
                (Opcode::GetFramePointer),
                (Opcode::GetStackPointer),
                (Opcode::GetReturnAddress),
                (Opcode::Null),
                (Opcode::X86Blendv),
                (Opcode::IcmpImm),
                (Opcode::X86Pmulhrsw),
                (Opcode::IaddImm),
                (Opcode::ImulImm),
                (Opcode::UdivImm),
                (Opcode::SdivImm),
                (Opcode::UremImm),
                (Opcode::SremImm),
                (Opcode::IrsubImm),
                (Opcode::IaddCin),
                (Opcode::IaddCarry),
                (Opcode::UaddOverflowTrap),
                (Opcode::IsubBin),
                (Opcode::IsubBorrow),
                (Opcode::BandImm),
                (Opcode::BorImm),
                (Opcode::BxorImm),
                (Opcode::RotlImm),
                (Opcode::RotrImm),
                (Opcode::IshlImm),
                (Opcode::UshrImm),
                (Opcode::SshrImm),
                (Opcode::IsNull),
                (Opcode::IsInvalid),
                (Opcode::ScalarToVector),
                (Opcode::X86Pmaddubsw),
                (Opcode::X86Cvtt2dq),
                (Opcode::Umulhi, &[I128, I128], &[I128]),
                (Opcode::Smulhi, &[I128, I128], &[I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/6073
                (Opcode::Iconcat, &[I32, I32], &[I64]),
                (Opcode::Iconcat, &[I16, I16], &[I32]),
                (Opcode::Iconcat, &[I8, I8], &[I16]),
                // https://github.com/bytecodealliance/wasmtime/issues/6073
                (Opcode::Isplit, &[I64], &[I32, I32]),
                (Opcode::Isplit, &[I32], &[I16, I16]),
                (Opcode::Isplit, &[I16], &[I8, I8]),
                (Opcode::Fmin, &[F32X4, F32X4], &[F32X4]),
                (Opcode::Fmin, &[F64X2, F64X2], &[F64X2]),
                (Opcode::Fmax, &[F32X4, F32X4], &[F32X4]),
                (Opcode::Fmax, &[F64X2, F64X2], &[F64X2]),
                (Opcode::FcvtToUintSat, &[F32X4], &[I8]),
                (Opcode::FcvtToUintSat, &[F64X2], &[I8]),
                (Opcode::FcvtToUintSat, &[F32X4], &[I16]),
                (Opcode::FcvtToUintSat, &[F64X2], &[I16]),
                (Opcode::FcvtToUintSat, &[F32X4], &[I32]),
                (Opcode::FcvtToUintSat, &[F64X2], &[I32]),
                (Opcode::FcvtToUintSat, &[F32X4], &[I64]),
                (Opcode::FcvtToUintSat, &[F64X2], &[I64]),
                (Opcode::FcvtToUintSat, &[F32X4], &[I128]),
                (Opcode::FcvtToUintSat, &[F64X2], &[I128]),
                (Opcode::FcvtToUintSat, &[F32], &[I8X16]),
                (Opcode::FcvtToUintSat, &[F64], &[I8X16]),
                (Opcode::FcvtToUintSat, &[F32X4], &[I8X16]),
                (Opcode::FcvtToUintSat, &[F64X2], &[I8X16]),
                (Opcode::FcvtToUintSat, &[F32], &[I16X8]),
                (Opcode::FcvtToUintSat, &[F64], &[I16X8]),
                (Opcode::FcvtToUintSat, &[F32X4], &[I16X8]),
                (Opcode::FcvtToUintSat, &[F64X2], &[I16X8]),
                (Opcode::FcvtToUintSat, &[F32], &[I32X4]),
                (Opcode::FcvtToUintSat, &[F64], &[I32X4]),
                (Opcode::FcvtToUintSat, &[F64X2], &[I32X4]),
                (Opcode::FcvtToUintSat, &[F32], &[I64X2]),
                (Opcode::FcvtToUintSat, &[F64], &[I64X2]),
                (Opcode::FcvtToUintSat, &[F32X4], &[I64X2]),
                (Opcode::FcvtToSintSat, &[F32X4], &[I8]),
                (Opcode::FcvtToSintSat, &[F64X2], &[I8]),
                (Opcode::FcvtToSintSat, &[F32X4], &[I16]),
                (Opcode::FcvtToSintSat, &[F64X2], &[I16]),
                (Opcode::FcvtToSintSat, &[F32X4], &[I32]),
                (Opcode::FcvtToSintSat, &[F64X2], &[I32]),
                (Opcode::FcvtToSintSat, &[F32X4], &[I64]),
                (Opcode::FcvtToSintSat, &[F64X2], &[I64]),
                (Opcode::FcvtToSintSat, &[F32X4], &[I128]),
                (Opcode::FcvtToSintSat, &[F64X2], &[I128]),
                (Opcode::FcvtToSintSat, &[F32], &[I8X16]),
                (Opcode::FcvtToSintSat, &[F64], &[I8X16]),
                (Opcode::FcvtToSintSat, &[F32X4], &[I8X16]),
                (Opcode::FcvtToSintSat, &[F64X2], &[I8X16]),
                (Opcode::FcvtToSintSat, &[F32], &[I16X8]),
                (Opcode::FcvtToSintSat, &[F64], &[I16X8]),
                (Opcode::FcvtToSintSat, &[F32X4], &[I16X8]),
                (Opcode::FcvtToSintSat, &[F64X2], &[I16X8]),
                (Opcode::FcvtToSintSat, &[F32], &[I32X4]),
                (Opcode::FcvtToSintSat, &[F64], &[I32X4]),
                (Opcode::FcvtToSintSat, &[F64X2], &[I32X4]),
                (Opcode::FcvtToSintSat, &[F32], &[I64X2]),
                (Opcode::FcvtToSintSat, &[F64], &[I64X2]),
                (Opcode::FcvtToSintSat, &[F32X4], &[I64X2]),
                (Opcode::FcvtFromUint, &[I8X16], &[F32]),
                (Opcode::FcvtFromUint, &[I16X8], &[F32]),
                (Opcode::FcvtFromUint, &[I32X4], &[F32]),
                (Opcode::FcvtFromUint, &[I64X2], &[F32]),
                (Opcode::FcvtFromUint, &[I8X16], &[F64]),
                (Opcode::FcvtFromUint, &[I16X8], &[F64]),
                (Opcode::FcvtFromUint, &[I32X4], &[F64]),
                (Opcode::FcvtFromUint, &[I64X2], &[F64]),
                (Opcode::FcvtFromUint, &[I8], &[F32X4]),
                (Opcode::FcvtFromUint, &[I16], &[F32X4]),
                (Opcode::FcvtFromUint, &[I32], &[F32X4]),
                (Opcode::FcvtFromUint, &[I64], &[F32X4]),
                (Opcode::FcvtFromUint, &[I128], &[F32X4]),
                (Opcode::FcvtFromUint, &[I8X16], &[F32X4]),
                (Opcode::FcvtFromUint, &[I16X8], &[F32X4]),
                (Opcode::FcvtFromUint, &[I64X2], &[F32X4]),
                (Opcode::FcvtFromUint, &[I8], &[F64X2]),
                (Opcode::FcvtFromUint, &[I16], &[F64X2]),
                (Opcode::FcvtFromUint, &[I32], &[F64X2]),
                (Opcode::FcvtFromUint, &[I64], &[F64X2]),
                (Opcode::FcvtFromUint, &[I128], &[F64X2]),
                (Opcode::FcvtFromUint, &[I8X16], &[F64X2]),
                (Opcode::FcvtFromUint, &[I16X8], &[F64X2]),
                (Opcode::FcvtFromUint, &[I32X4], &[F64X2]),
                (Opcode::FcvtFromSint, &[I8X16], &[F32]),
                (Opcode::FcvtFromSint, &[I16X8], &[F32]),
                (Opcode::FcvtFromSint, &[I32X4], &[F32]),
                (Opcode::FcvtFromSint, &[I64X2], &[F32]),
                (Opcode::FcvtFromSint, &[I8X16], &[F64]),
                (Opcode::FcvtFromSint, &[I16X8], &[F64]),
                (Opcode::FcvtFromSint, &[I32X4], &[F64]),
                (Opcode::FcvtFromSint, &[I64X2], &[F64]),
                (Opcode::FcvtFromSint, &[I8], &[F32X4]),
                (Opcode::FcvtFromSint, &[I16], &[F32X4]),
                (Opcode::FcvtFromSint, &[I32], &[F32X4]),
                (Opcode::FcvtFromSint, &[I64], &[F32X4]),
                (Opcode::FcvtFromSint, &[I128], &[F32X4]),
                (Opcode::FcvtFromSint, &[I8X16], &[F32X4]),
                (Opcode::FcvtFromSint, &[I16X8], &[F32X4]),
                (Opcode::FcvtFromSint, &[I64X2], &[F32X4]),
                (Opcode::FcvtFromSint, &[I8], &[F64X2]),
                (Opcode::FcvtFromSint, &[I16], &[F64X2]),
                (Opcode::FcvtFromSint, &[I32], &[F64X2]),
                (Opcode::FcvtFromSint, &[I64], &[F64X2]),
                (Opcode::FcvtFromSint, &[I128], &[F64X2]),
                (Opcode::FcvtFromSint, &[I8X16], &[F64X2]),
                (Opcode::FcvtFromSint, &[I16X8], &[F64X2]),
                (Opcode::FcvtFromSint, &[I32X4], &[F64X2]),
            )
        })
        .filter(|(op, ..)| {
            allowed_opcodes
                .as_ref()
                .map_or(true, |opcodes| opcodes.contains(op))
        })
        .collect()
});

fn inserter_for_format(fmt: InstructionFormat) -> OpcodeInserter {
    match fmt {
        InstructionFormat::AtomicCas => insert_atomic_cas,
        InstructionFormat::AtomicRmw => insert_atomic_rmw,
        InstructionFormat::Binary => insert_opcode,
        InstructionFormat::BinaryImm64 => todo!(),
        InstructionFormat::BinaryImm8 => insert_ins_ext_lane,
        InstructionFormat::Call => insert_call,
        InstructionFormat::CallIndirect => insert_call,
        InstructionFormat::CondTrap => todo!(),
        InstructionFormat::DynamicStackLoad => todo!(),
        InstructionFormat::DynamicStackStore => todo!(),
        InstructionFormat::FloatCompare => insert_cmp,
        InstructionFormat::FuncAddr => todo!(),
        InstructionFormat::IntAddTrap => todo!(),
        InstructionFormat::IntCompare => insert_cmp,
        InstructionFormat::IntCompareImm => todo!(),
        InstructionFormat::Load => insert_load_store,
        InstructionFormat::LoadNoOffset => insert_load_store,
        InstructionFormat::NullAry => insert_opcode,
        InstructionFormat::Shuffle => insert_shuffle,
        InstructionFormat::StackLoad => insert_stack_load,
        InstructionFormat::StackStore => insert_stack_store,
        InstructionFormat::Store => insert_load_store,
        InstructionFormat::StoreNoOffset => insert_load_store,
        InstructionFormat::Ternary => insert_opcode,
        InstructionFormat::TernaryImm8 => insert_ins_ext_lane,
        InstructionFormat::Trap => todo!(),
        InstructionFormat::Unary => insert_opcode,
        InstructionFormat::UnaryConst => insert_const,
        InstructionFormat::UnaryGlobalValue => todo!(),
        InstructionFormat::UnaryIeee16 => insert_const,
        InstructionFormat::UnaryIeee32 => insert_const,
        InstructionFormat::UnaryIeee64 => insert_const,
        InstructionFormat::UnaryImm => insert_const,

        InstructionFormat::BranchTable
        | InstructionFormat::Brif
        | InstructionFormat::Jump
        | InstructionFormat::MultiAry => {
            panic!("Control-flow instructions should be handled by 'insert_terminator': {fmt:?}")
        }
    }
}

pub struct FunctionGenerator<'r, 'data>
where
    'data: 'r,
{
    u: &'r mut Unstructured<'data>,
    config: &'r Config,
    resources: Resources,
    isa: OwnedTargetIsa,
    name: UserFuncName,
    signature: Signature,
}

#[derive(Debug, Clone)]
enum BlockTerminator {
    Return,
    Jump(Block),
    Br(Block, Block),
    BrTable(Block, Vec<Block>),
    Switch(Type, Block, HashMap<u128, Block>),
    TailCall(FuncRef),
    TailCallIndirect(FuncRef),
}

#[derive(Debug, Clone)]
enum BlockTerminatorKind {
    Return,
    Jump,
    Br,
    BrTable,
    Switch,
    TailCall,
    TailCallIndirect,
}

/// Alias Analysis Category
///
/// Our alias analysis pass supports 4 categories of accesses to distinguish
/// different regions. The "Other" region is the general case, and is the default
/// Although they have highly suggestive names there is no difference between any
/// of the categories.
///
/// We assign each stack slot a category when we first generate them, and then
/// ensure that all accesses to that stack slot are correctly tagged. We already
/// ensure that memory accesses never cross stack slots, so there is no risk
/// of a memory access being tagged with the wrong category.
#[derive(Debug, PartialEq, Clone, Copy)]
enum AACategory {
    Other,
    Heap,
    Table,
    VmCtx,
}

impl AACategory {
    pub fn all() -> &'static [Self] {
        &[
            AACategory::Other,
            AACategory::Heap,
            AACategory::Table,
            AACategory::VmCtx,
        ]
    }

    pub fn update_memflags(&self, flags: &mut MemFlags) {
        flags.set_alias_region(match self {
            AACategory::Other => None,
            AACategory::Heap => Some(AliasRegion::Heap),
            AACategory::Table => Some(AliasRegion::Table),
            AACategory::VmCtx => Some(AliasRegion::Vmctx),
        })
    }
}

pub type StackAlignment = StackSize;

#[derive(Default)]
struct Resources {
    vars: HashMap<Type, Vec<Variable>>,
    blocks: Vec<(Block, BlockSignature)>,
    blocks_without_params: Vec<Block>,
    block_terminators: Vec<BlockTerminator>,
    func_refs: Vec<(Signature, SigRef, FuncRef)>,
    /// This field is required to be sorted by stack slot size at all times.
    /// We use this invariant when searching for stack slots with a given size.
    /// See [FunctionGenerator::stack_slot_with_size]
    stack_slots: Vec<(StackSlot, StackSize, StackAlignment, AACategory)>,
    usercalls: Vec<(UserExternalName, Signature)>,
    libcalls: Vec<LibCall>,
}

impl Resources {
    /// Partitions blocks at `block`. Only blocks that can be targeted by branches are considered.
    ///
    /// The first slice includes all blocks up to and including `block`.
    /// The second slice includes all remaining blocks.
    fn partition_target_blocks(
        &self,
        block: Block,
    ) -> (&[(Block, BlockSignature)], &[(Block, BlockSignature)]) {
        // Blocks are stored in-order and have no gaps, this means that we can simply index them by
        // their number. We also need to exclude the entry block since it isn't a valid target.
        let target_blocks = &self.blocks[1..];
        target_blocks.split_at(block.as_u32() as usize)
    }

    /// Returns blocks forward of `block`. Only blocks that can be targeted by branches are considered.
    fn forward_blocks(&self, block: Block) -> &[(Block, BlockSignature)] {
        let (_, forward_blocks) = self.partition_target_blocks(block);
        forward_blocks
    }

    /// Generates a slice of `blocks_without_params` ahead of `block`
    fn forward_blocks_without_params(&self, block: Block) -> &[Block] {
        let partition_point = self.blocks_without_params.partition_point(|b| *b <= block);
        &self.blocks_without_params[partition_point..]
    }

    /// Generates an iterator of all valid tail call targets. This includes all functions with both
    ///  the `tail` calling convention and the same return values as the caller.
    fn tail_call_targets<'a>(
        &'a self,
        caller_sig: &'a Signature,
    ) -> impl Iterator<Item = &'a (Signature, SigRef, FuncRef)> {
        self.func_refs.iter().filter(|(sig, _, _)| {
            sig.call_conv == CallConv::Tail && sig.returns == caller_sig.returns
        })
    }
}

impl<'r, 'data> FunctionGenerator<'r, 'data>
where
    'data: 'r,
{
    pub fn new(
        u: &'r mut Unstructured<'data>,
        config: &'r Config,
        isa: OwnedTargetIsa,
        name: UserFuncName,
        signature: Signature,
        usercalls: Vec<(UserExternalName, Signature)>,
        libcalls: Vec<LibCall>,
    ) -> Self {
        Self {
            u,
            config,
            resources: Resources {
                usercalls,
                libcalls,
                ..Resources::default()
            },
            isa,
            name,
            signature,
        }
    }

    /// Generates a random value for config `param`
    fn param(&mut self, param: &RangeInclusive<usize>) -> Result<usize> {
        Ok(self.u.int_in_range(param.clone())?)
    }

    fn system_callconv(&mut self) -> CallConv {
        // TODO: This currently only runs on linux, so this is the only choice
        // We should improve this once we generate flags and targets
        CallConv::SystemV
    }

    /// Finds a stack slot with size of at least n bytes
    fn stack_slot_with_size(
        &mut self,
        n: u32,
    ) -> Result<(StackSlot, StackSize, StackAlignment, AACategory)> {
        let first = self
            .resources
            .stack_slots
            .partition_point(|&(_slot, size, _align, _category)| size < n);
        Ok(*self.u.choose(&self.resources.stack_slots[first..])?)
    }

    /// Generates an address that should allow for a store or a load.
    ///
    /// Addresses aren't generated like other values. They are never stored in variables so that
    /// we don't run the risk of returning them from a function, which would make the fuzzer
    /// complain since they are different from the interpreter to the backend.
    ///
    /// `min_size`: Controls the amount of space that the address should have.
    ///
    /// `aligned`: When passed as true, the resulting address is guaranteed to be aligned
    /// on an 8 byte boundary.
    ///
    /// Returns a valid address and the maximum possible offset that still respects `min_size`.
    fn generate_load_store_address(
        &mut self,
        builder: &mut FunctionBuilder,
        min_size: u32,
        aligned: bool,
    ) -> Result<(Value, u32, AACategory)> {
        // TODO: Currently our only source of addresses is stack_addr, but we
        // should add global_value, symbol_value eventually
        let (addr, available_size, category) = {
            let (ss, slot_size, _align, category) = self.stack_slot_with_size(min_size)?;

            // stack_slot_with_size guarantees that slot_size >= min_size
            let max_offset = slot_size - min_size;
            let offset = if aligned {
                self.u.int_in_range(0..=max_offset / min_size)? * min_size
            } else {
                self.u.int_in_range(0..=max_offset)?
            };

            let base_addr = builder.ins().stack_addr(I64, ss, offset as i32);
            let available_size = slot_size.saturating_sub(offset);
            (base_addr, available_size, category)
        };

        // TODO: Insert a bunch of amode opcodes here to modify the address!

        // Now that we have an address and a size, we just choose a random offset to return to the
        // caller. Preserving min_size bytes.
        let max_offset = available_size.saturating_sub(min_size);
        Ok((addr, max_offset, category))
    }

    // Generates an address and memflags for a load or store.
    fn generate_address_and_memflags(
        &mut self,
        builder: &mut FunctionBuilder,
        min_size: u32,
        is_atomic: bool,
    ) -> Result<(Value, MemFlags, Offset32)> {
        // Should we generate an aligned address
        // Some backends have issues with unaligned atomics.
        // AArch64: https://github.com/bytecodealliance/wasmtime/issues/5483
        // RISCV: https://github.com/bytecodealliance/wasmtime/issues/5882
        let requires_aligned_atomics = matches!(
            self.isa.triple().architecture,
            Architecture::Aarch64(_) | Architecture::Riscv64(_)
        );
        let aligned = if is_atomic && requires_aligned_atomics {
            true
        } else if min_size > 8 {
            // TODO: We currently can't guarantee that a stack_slot will be aligned on a 16 byte
            // boundary. We don't have a way to specify alignment when creating stack slots, and
            // cranelift only guarantees 8 byte alignment between stack slots.
            // See: https://github.com/bytecodealliance/wasmtime/issues/5922#issuecomment-1457926624
            false
        } else {
            bool::arbitrary(self.u)?
        };

        let mut flags = MemFlags::new();
        // Even if we picked an aligned address, we can always generate unaligned memflags
        if aligned && bool::arbitrary(self.u)? {
            flags.set_aligned();
        }
        // If the address is aligned, then we know it won't trap
        if aligned && bool::arbitrary(self.u)? {
            flags.set_notrap();
        }

        let (address, max_offset, category) =
            self.generate_load_store_address(builder, min_size, aligned)?;

        // Set the Alias Analysis bits on the memflags
        category.update_memflags(&mut flags);

        // Pick an offset to pass into the load/store.
        let offset = if aligned {
            0
        } else {
            self.u.int_in_range(0..=max_offset)? as i32
        }
        .into();

        Ok((address, flags, offset))
    }

    /// Get a variable of type `ty` from the current function
    fn get_variable_of_type(&mut self, ty: Type) -> Result<Variable> {
        let opts = self.resources.vars.get(&ty).map_or(&[][..], Vec::as_slice);
        let var = self.u.choose(opts)?;
        Ok(*var)
    }

    /// Generates an instruction(`iconst`/`fconst`/etc...) to introduce a constant value
    fn generate_const(&mut self, builder: &mut FunctionBuilder, ty: Type) -> Result<Value> {
        Ok(match self.u.datavalue(ty)? {
            DataValue::I8(i) => builder.ins().iconst(ty, i as u8 as i64),
            DataValue::I16(i) => builder.ins().iconst(ty, i as u16 as i64),
            DataValue::I32(i) => builder.ins().iconst(ty, i as u32 as i64),
            DataValue::I64(i) => builder.ins().iconst(ty, i),
            DataValue::I128(i) => {
                let hi = builder.ins().iconst(I64, (i >> 64) as i64);
                let lo = builder.ins().iconst(I64, i as i64);
                builder.ins().iconcat(lo, hi)
            }
            DataValue::F16(f) => builder.ins().f16const(f),
            DataValue::F32(f) => builder.ins().f32const(f),
            DataValue::F64(f) => builder.ins().f64const(f),
            DataValue::F128(f) => {
                let handle = builder.func.dfg.constants.insert(f.into());
                builder.ins().f128const(handle)
            }
            DataValue::V128(bytes) => {
                let data = bytes.to_vec().into();
                let handle = builder.func.dfg.constants.insert(data);
                builder.ins().vconst(ty, handle)
            }
            _ => unimplemented!(),
        })
    }

    /// Chooses a random block which can be targeted by a jump / branch.
    /// This means any block that is not the first block.
    fn generate_target_block(&mut self, source_block: Block) -> Result<Block> {
        // We try to mostly generate forward branches to avoid generating an excessive amount of
        // infinite loops. But they are still important, so give them a small chance of existing.
        let (backwards_blocks, forward_blocks) =
            self.resources.partition_target_blocks(source_block);
        let ratio = self.config.backwards_branch_ratio;
        let block_targets = if !backwards_blocks.is_empty() && self.u.ratio(ratio.0, ratio.1)? {
            backwards_blocks
        } else {
            forward_blocks
        };
        assert!(!block_targets.is_empty());

        let (block, _) = self.u.choose(block_targets)?.clone();
        Ok(block)
    }

    fn generate_values_for_block(
        &mut self,
        builder: &mut FunctionBuilder,
        block: Block,
    ) -> Result<Vec<Value>> {
        let (_, sig) = self.resources.blocks[block.as_u32() as usize].clone();
        self.generate_values_for_signature(builder, sig.iter().copied())
    }

    fn generate_values_for_signature<I: Iterator<Item = Type>>(
        &mut self,
        builder: &mut FunctionBuilder,
        signature: I,
    ) -> Result<Vec<Value>> {
        signature
            .map(|ty| {
                let var = self.get_variable_of_type(ty)?;
                let val = builder.use_var(var);
                Ok(val)
            })
            .collect()
    }

    /// The terminator that we need to insert has already been picked ahead of time
    /// we just need to build the instructions for it
    fn insert_terminator(
        &mut self,
        builder: &mut FunctionBuilder,
        source_block: Block,
    ) -> Result<()> {
        let terminator = self.resources.block_terminators[source_block.as_u32() as usize].clone();

        match terminator {
            BlockTerminator::Return => {
                let types: Vec<Type> = {
                    let rets = &builder.func.signature.returns;
                    rets.iter().map(|p| p.value_type).collect()
                };
                let vals = self.generate_values_for_signature(builder, types.into_iter())?;

                builder.ins().return_(&vals[..]);
            }
            BlockTerminator::Jump(target) => {
                let args = self.generate_values_for_block(builder, target)?;
                builder.ins().jump(target, &args[..]);
            }
            BlockTerminator::Br(left, right) => {
                let left_args = self.generate_values_for_block(builder, left)?;
                let right_args = self.generate_values_for_block(builder, right)?;

                let condbr_types = [I8, I16, I32, I64, I128];
                let _type = *self.u.choose(&condbr_types[..])?;
                let val = builder.use_var(self.get_variable_of_type(_type)?);
                builder
                    .ins()
                    .brif(val, left, &left_args[..], right, &right_args[..]);
            }
            BlockTerminator::BrTable(default, targets) => {
                // Create jump tables on demand
                let mut jt = Vec::with_capacity(targets.len());
                for block in targets {
                    let args = self.generate_values_for_block(builder, block)?;
                    jt.push(builder.func.dfg.block_call(block, &args))
                }

                let args = self.generate_values_for_block(builder, default)?;
                let jt_data = JumpTableData::new(builder.func.dfg.block_call(default, &args), &jt);
                let jt = builder.create_jump_table(jt_data);

                // br_table only supports I32
                let val = builder.use_var(self.get_variable_of_type(I32)?);

                builder.ins().br_table(val, jt);
            }
            BlockTerminator::Switch(_type, default, entries) => {
                let mut switch = Switch::new();
                for (&entry, &block) in entries.iter() {
                    switch.set_entry(entry, block);
                }

                let switch_val = builder.use_var(self.get_variable_of_type(_type)?);

                switch.emit(builder, switch_val, default);
            }
            BlockTerminator::TailCall(target) | BlockTerminator::TailCallIndirect(target) => {
                let (sig, sig_ref, func_ref) = self
                    .resources
                    .func_refs
                    .iter()
                    .find(|(_, _, f)| *f == target)
                    .expect("Failed to find previously selected function")
                    .clone();

                let opcode = match terminator {
                    BlockTerminator::TailCall(_) => Opcode::ReturnCall,
                    BlockTerminator::TailCallIndirect(_) => Opcode::ReturnCallIndirect,
                    _ => unreachable!(),
                };

                insert_call_to_function(self, builder, opcode, &sig, sig_ref, func_ref)?;
            }
        }

        Ok(())
    }

    /// Fills the current block with random instructions
    fn generate_instructions(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        for _ in 0..self.param(&self.config.instructions_per_block)? {
            let (op, args, rets) = self.u.choose(&OPCODE_SIGNATURES)?;

            // We filter out instructions that aren't supported by the target at this point instead
            // of building a single vector of valid instructions at the beginning of function
            // generation, to avoid invalidating the corpus when instructions are enabled/disabled.
            if !valid_for_target(&self.isa.triple(), *op, &args, &rets) {
                return Err(arbitrary::Error::IncorrectFormat.into());
            }

            let inserter = inserter_for_format(op.format());
            inserter(self, builder, *op, &args, &rets)?;
        }

        Ok(())
    }

    fn generate_funcrefs(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let usercalls: Vec<(ExternalName, Signature)> = self
            .resources
            .usercalls
            .iter()
            .map(|(name, signature)| {
                let user_func_ref = builder.func.declare_imported_user_function(name.clone());
                let name = ExternalName::User(user_func_ref);
                (name, signature.clone())
            })
            .collect();

        let lib_callconv = self.system_callconv();
        let libcalls: Vec<(ExternalName, Signature)> = self
            .resources
            .libcalls
            .iter()
            .map(|libcall| {
                let pointer_type = Type::int_with_byte_size(
                    self.isa.triple().pointer_width().unwrap().bytes().into(),
                )
                .unwrap();
                let signature = libcall.signature(lib_callconv, pointer_type);
                let name = ExternalName::LibCall(*libcall);
                (name, signature)
            })
            .collect();

        for (name, signature) in usercalls.into_iter().chain(libcalls) {
            let sig_ref = builder.import_signature(signature.clone());
            let func_ref = builder.import_function(ExtFuncData {
                name,
                signature: sig_ref,
                colocated: self.u.arbitrary()?,
            });

            self.resources
                .func_refs
                .push((signature, sig_ref, func_ref));
        }

        Ok(())
    }

    fn generate_stack_slots(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        for _ in 0..self.param(&self.config.static_stack_slots_per_function)? {
            let bytes = self.param(&self.config.static_stack_slot_size)? as u32;
            let alignment = self.param(&self.config.stack_slot_alignment_log2)? as u8;
            let alignment_bytes = 1 << alignment;

            let ss_data = StackSlotData::new(StackSlotKind::ExplicitSlot, bytes, alignment);
            let slot = builder.create_sized_stack_slot(ss_data);

            // Generate one Alias Analysis Category for each slot
            let category = *self.u.choose(AACategory::all())?;

            self.resources
                .stack_slots
                .push((slot, bytes, alignment_bytes, category));
        }

        self.resources
            .stack_slots
            .sort_unstable_by_key(|&(_slot, bytes, _align, _category)| bytes);

        Ok(())
    }

    /// Zero initializes the stack slot by inserting `stack_store`'s.
    fn initialize_stack_slots(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let i8_zero = builder.ins().iconst(I8, 0);
        let i16_zero = builder.ins().iconst(I16, 0);
        let i32_zero = builder.ins().iconst(I32, 0);
        let i64_zero = builder.ins().iconst(I64, 0);
        let i128_zero = builder.ins().uextend(I128, i64_zero);

        for &(slot, init_size, _align, category) in self.resources.stack_slots.iter() {
            let mut size = init_size;

            // Insert the largest available store for the remaining size.
            while size != 0 {
                let offset = (init_size - size) as i32;
                let (val, filled) = match size {
                    sz if sz / 16 > 0 => (i128_zero, 16),
                    sz if sz / 8 > 0 => (i64_zero, 8),
                    sz if sz / 4 > 0 => (i32_zero, 4),
                    sz if sz / 2 > 0 => (i16_zero, 2),
                    _ => (i8_zero, 1),
                };
                let addr = builder.ins().stack_addr(I64, slot, offset);

                // Each stack slot has an associated category, that means we have to set the
                // correct memflags for it. So we can't use `stack_store` directly.
                let mut flags = MemFlags::new();
                flags.set_notrap();
                category.update_memflags(&mut flags);

                builder.ins().store(flags, val, addr, 0);

                size -= filled;
            }
        }
        Ok(())
    }

    /// Creates a random amount of blocks in this function
    fn generate_blocks(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let extra_block_count = self.param(&self.config.blocks_per_function)?;

        // We must always have at least one block, so we generate the "extra" blocks and add 1 for
        // the entry block.
        let block_count = 1 + extra_block_count;

        // Blocks need to be sorted in ascending order
        self.resources.blocks = (0..block_count)
            .map(|i| {
                let is_entry = i == 0;
                let block = builder.create_block();

                // Optionally mark blocks that are not the entry block as cold
                if !is_entry {
                    if bool::arbitrary(self.u)? {
                        builder.set_cold_block(block);
                    }
                }

                // The first block has to have the function signature, but for the rest of them we generate
                // a random signature;
                if is_entry {
                    builder.append_block_params_for_function_params(block);
                    Ok((
                        block,
                        self.signature.params.iter().map(|a| a.value_type).collect(),
                    ))
                } else {
                    let sig = self.generate_block_signature()?;
                    sig.iter().for_each(|ty| {
                        builder.append_block_param(block, *ty);
                    });
                    Ok((block, sig))
                }
            })
            .collect::<Result<Vec<_>>>()?;

        // Valid blocks for jump tables have to have no parameters in the signature, and must also
        // not be the first block.
        self.resources.blocks_without_params = self.resources.blocks[1..]
            .iter()
            .filter(|(_, sig)| sig.len() == 0)
            .map(|(b, _)| *b)
            .collect();

        // Compute the block CFG
        //
        // cranelift-frontend requires us to never generate unreachable blocks
        // To ensure this property we start by constructing a main "spine" of blocks. So block1 can
        // always jump to block2, and block2 can always jump to block3, etc...
        //
        // That is not a very interesting CFG, so we introduce variations on that, but always
        // ensuring that the property of pointing to the next block is maintained whatever the
        // branching mechanism we use.
        let blocks = self.resources.blocks.clone();
        self.resources.block_terminators = blocks
            .iter()
            .map(|&(block, _)| {
                let next_block = Block::with_number(block.as_u32() + 1).unwrap();
                let forward_blocks = self.resources.forward_blocks(block);
                let paramless_targets = self.resources.forward_blocks_without_params(block);
                let has_paramless_targets = !paramless_targets.is_empty();
                let next_block_is_paramless = paramless_targets.contains(&next_block);

                let mut valid_terminators = vec![];

                if forward_blocks.is_empty() {
                    // Return is only valid on the last block.
                    valid_terminators.push(BlockTerminatorKind::Return);
                } else {
                    // If we have more than one block we can allow terminators that target blocks.
                    // TODO: We could add some kind of BrReturn here, to explore edges where we
                    // exit in the middle of the function
                    valid_terminators.extend_from_slice(&[
                        BlockTerminatorKind::Jump,
                        BlockTerminatorKind::Br,
                        BlockTerminatorKind::BrTable,
                    ]);
                }

                // As the Switch interface only allows targeting blocks without params we need
                // to ensure that the next block has no params, since that one is guaranteed to be
                // picked in either case.
                if has_paramless_targets && next_block_is_paramless {
                    valid_terminators.push(BlockTerminatorKind::Switch);
                }

                // Tail Calls are a block terminator, so we should insert them as any other block
                // terminator. We should ensure that we can select at least one target before considering
                // them as candidate instructions.
                let has_tail_callees = self
                    .resources
                    .tail_call_targets(&self.signature)
                    .next()
                    .is_some();
                let is_tail_caller = self.signature.call_conv == CallConv::Tail;

                let supports_tail_calls = match self.isa.triple().architecture {
                    Architecture::Aarch64(_) | Architecture::Riscv64(_) => true,
                    // TODO: x64 currently requires frame pointers for tail calls.
                    Architecture::X86_64 => self.isa.flags().preserve_frame_pointers(),
                    // TODO: Other platforms do not support tail calls yet.
                    _ => false,
                };

                if is_tail_caller && has_tail_callees && supports_tail_calls {
                    valid_terminators.extend([
                        BlockTerminatorKind::TailCall,
                        BlockTerminatorKind::TailCallIndirect,
                    ]);
                }

                let terminator = self.u.choose(&valid_terminators)?;

                // Choose block targets for the terminators that we picked above
                Ok(match terminator {
                    BlockTerminatorKind::Return => BlockTerminator::Return,
                    BlockTerminatorKind::Jump => BlockTerminator::Jump(next_block),
                    BlockTerminatorKind::Br => {
                        BlockTerminator::Br(next_block, self.generate_target_block(block)?)
                    }
                    // TODO: Allow generating backwards branches here
                    BlockTerminatorKind::BrTable => {
                        // Make the default the next block, and then we don't have to worry
                        // that we can reach it via the targets
                        let default = next_block;

                        let target_count = self.param(&self.config.jump_table_entries)?;
                        let targets = Result::from_iter(
                            (0..target_count).map(|_| self.generate_target_block(block)),
                        )?;

                        BlockTerminator::BrTable(default, targets)
                    }
                    BlockTerminatorKind::Switch => {
                        // Make the default the next block, and then we don't have to worry
                        // that we can reach it via the entries below
                        let default_block = next_block;

                        let _type = *self.u.choose(&[I8, I16, I32, I64, I128][..])?;

                        // Build this into a HashMap since we cannot have duplicate entries.
                        let mut entries = HashMap::new();
                        for _ in 0..self.param(&self.config.switch_cases)? {
                            // The Switch API only allows for entries that are addressable by the index type
                            // so we need to limit the range of values that we generate.
                            let (ty_min, ty_max) = _type.bounds(false);
                            let range_start = self.u.int_in_range(ty_min..=ty_max)?;

                            // We can either insert a contiguous range of blocks or a individual block
                            // This is done because the Switch API specializes contiguous ranges.
                            let range_size = if bool::arbitrary(self.u)? {
                                1
                            } else {
                                self.param(&self.config.switch_max_range_size)?
                            } as u128;

                            // Build the switch entries
                            for i in 0..range_size {
                                let index = range_start.wrapping_add(i) % ty_max;
                                let block = *self
                                    .u
                                    .choose(self.resources.forward_blocks_without_params(block))?;

                                entries.insert(index, block);
                            }
                        }

                        BlockTerminator::Switch(_type, default_block, entries)
                    }
                    BlockTerminatorKind::TailCall => {
                        let targets = self
                            .resources
                            .tail_call_targets(&self.signature)
                            .collect::<Vec<_>>();
                        let (_, _, funcref) = *self.u.choose(&targets[..])?;
                        BlockTerminator::TailCall(*funcref)
                    }
                    BlockTerminatorKind::TailCallIndirect => {
                        let targets = self
                            .resources
                            .tail_call_targets(&self.signature)
                            .collect::<Vec<_>>();
                        let (_, _, funcref) = *self.u.choose(&targets[..])?;
                        BlockTerminator::TailCallIndirect(*funcref)
                    }
                })
            })
            .collect::<Result<_>>()?;

        Ok(())
    }

    fn generate_block_signature(&mut self) -> Result<BlockSignature> {
        let param_count = self.param(&self.config.block_signature_params)?;

        let mut params = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            params.push(self.u._type((&*self.isa).supports_simd())?);
        }
        Ok(params)
    }

    fn build_variable_pool(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let block = builder.current_block().unwrap();

        // Define variables for the function signature
        let mut vars: Vec<_> = builder
            .func
            .signature
            .params
            .iter()
            .map(|param| param.value_type)
            .zip(builder.block_params(block).iter().copied())
            .collect();

        // Create a pool of vars that are going to be used in this function
        for _ in 0..self.param(&self.config.vars_per_function)? {
            let ty = self.u._type((&*self.isa).supports_simd())?;
            let value = self.generate_const(builder, ty)?;
            vars.push((ty, value));
        }

        for (id, (ty, value)) in vars.into_iter().enumerate() {
            let var = Variable::new(id);
            builder.declare_var(var, ty);
            builder.def_var(var, value);

            // Randomly declare variables as needing a stack map.
            // We limit these to only types that have fewer than 16 bytes
            // since the stack map mechanism does not support larger types.
            if ty.bytes() <= 16 && self.u.arbitrary()? {
                builder.declare_var_needs_stack_map(var);
            }

            self.resources
                .vars
                .entry(ty)
                .or_insert_with(Vec::new)
                .push(var);
        }

        Ok(())
    }

    /// We generate a function in multiple stages:
    ///
    /// * First we generate a random number of empty blocks
    /// * Then we generate a random pool of variables to be used throughout the function
    /// * We then visit each block and generate random instructions
    ///
    /// Because we generate all blocks and variables up front we already know everything that
    /// we need when generating instructions (i.e. jump targets / variables)
    pub fn generate(mut self) -> Result<Function> {
        let mut fn_builder_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(self.name.clone(), self.signature.clone());

        let mut builder = FunctionBuilder::new(&mut func, &mut fn_builder_ctx);

        // Build the function references before generating the block CFG since we store
        // function references in the CFG.
        self.generate_funcrefs(&mut builder)?;
        self.generate_blocks(&mut builder)?;

        // Function preamble
        self.generate_stack_slots(&mut builder)?;

        // Main instruction generation loop
        for (block, block_sig) in self.resources.blocks.clone().into_iter() {
            let is_block0 = block.as_u32() == 0;
            builder.switch_to_block(block);

            if is_block0 {
                // The first block is special because we must create variables both for the
                // block signature and for the variable pool. Additionally, we must also define
                // initial values for all variables that are not the function signature.
                self.build_variable_pool(&mut builder)?;

                // Stack slots have random bytes at the beginning of the function
                // initialize them to a constant value so that execution stays predictable.
                self.initialize_stack_slots(&mut builder)?;
            } else {
                // Define variables for the block params
                for (i, ty) in block_sig.iter().enumerate() {
                    let var = self.get_variable_of_type(*ty)?;
                    let block_param = builder.block_params(block)[i];
                    builder.def_var(var, block_param);
                }
            }

            // Generate block instructions
            self.generate_instructions(&mut builder)?;

            // Insert a terminator to safely exit the block
            self.insert_terminator(&mut builder, block)?;
        }

        builder.seal_all_blocks();
        builder.finalize();

        Ok(func)
    }
}
