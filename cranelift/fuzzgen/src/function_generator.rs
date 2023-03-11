use crate::config::Config;
use crate::cranelift_arbitrary::CraneliftArbitrary;
use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use cranelift::codegen::data_value::DataValue;
use cranelift::codegen::ir::immediates::Offset32;
use cranelift::codegen::ir::instructions::InstructionFormat;
use cranelift::codegen::ir::stackslot::StackSize;

use cranelift::codegen::ir::{
    types::*, AtomicRmwOp, Block, ConstantData, ExternalName, FuncRef, Function, LibCall, Opcode,
    Signature, StackSlot, Type, UserExternalName, UserFuncName, Value,
};
use cranelift::codegen::isa::CallConv;
use cranelift::frontend::{FunctionBuilder, FunctionBuilderContext, Switch, Variable};
use cranelift::prelude::{
    EntityRef, ExtFuncData, FloatCC, InstBuilder, IntCC, JumpTableData, MemFlags, StackSlotData,
    StackSlotKind,
};
use std::collections::HashMap;
use std::ops::RangeInclusive;
use target_lexicon::{Architecture, Triple};

type BlockSignature = Vec<Type>;

fn insert_opcode(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    args: &'static [Type],
    rets: &'static [Type],
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

fn insert_call(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    _args: &'static [Type],
    _rets: &'static [Type],
) -> Result<()> {
    assert_eq!(opcode, Opcode::Call, "only call handled at the moment");
    let (sig, func_ref) = fgen.u.choose(&fgen.resources.func_refs)?.clone();

    let actuals = fgen.generate_values_for_signature(
        builder,
        sig.params.iter().map(|abi_param| abi_param.value_type),
    )?;

    builder.ins().call(func_ref, &actuals);
    Ok(())
}

fn insert_stack_load(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    _opcode: Opcode,
    _args: &'static [Type],
    rets: &'static [Type],
) -> Result<()> {
    let typevar = rets[0];
    let type_size = typevar.bytes();
    let (slot, slot_size) = fgen.stack_slot_with_size(type_size)?;
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
    args: &'static [Type],
    _rets: &'static [Type],
) -> Result<()> {
    let typevar = args[0];
    let type_size = typevar.bytes();
    let (slot, slot_size) = fgen.stack_slot_with_size(type_size)?;
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
    args: &'static [Type],
    rets: &'static [Type],
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
        let unimplemented_cc = match (fgen.target_triple.architecture, cc) {
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
    _args: &'static [Type],
    rets: &'static [Type],
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
    args: &'static [Type],
    rets: &'static [Type],
) -> Result<()> {
    let from_var = fgen.get_variable_of_type(args[0])?;
    let from_val = builder.use_var(from_var);

    let to_var = fgen.get_variable_of_type(rets[0])?;

    // TODO: We can generate little/big endian flags here.
    let memflags = MemFlags::new();

    let res = builder.ins().bitcast(rets[0], memflags, from_val);
    builder.def_var(to_var, res);
    Ok(())
}

fn insert_load_store(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    args: &'static [Type],
    rets: &'static [Type],
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
    _: &'static [Type],
    rets: &'static [Type],
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
    _: &'static [Type],
    rets: &'static [Type],
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
    _: &'static [Type],
    rets: &'static [Type],
) -> Result<()> {
    let ctrl_type = *rets.first().unwrap();

    let lhs = builder.use_var(fgen.get_variable_of_type(ctrl_type)?);
    let rhs = builder.use_var(fgen.get_variable_of_type(ctrl_type)?);

    let mask = {
        let lanes = fgen.u.arbitrary::<[u8; 16]>()?;
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
    args: &'static [Type],
    rets: &'static [Type],
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
    &'static [Type],
    &'static [Type],
) -> Result<()>;

/// Returns true if we believe this `OpcodeSignature` should compile correctly
/// for the given target triple. We currently have a range of known issues
/// with specific lowerings on specific backends, and we don't want to get
/// fuzz bug reports for those. Over time our goal is to eliminate all of these
/// exceptions.
fn valid_for_target(triple: &Triple, op: Opcode, args: &[Type], rets: &[Type]) -> bool {
    macro_rules! exceptions {
        ( $(($($cases:pat),*)),* $(,)?) => {
            match (op, args, rets) {
                $( ($($cases,)* ..) => false, )*
                _ => true,
            }
        }
    }

    match triple.architecture {
        Architecture::X86_64 => {
            exceptions!(
                (Opcode::IaddCout, &[I8, I8]),
                (Opcode::IaddCout, &[I16, I16]),
                (Opcode::IaddCout, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5468
                (Opcode::Smulhi, &[I8, I8]),
                // https://github.com/bytecodealliance/wasmtime/issues/5468
                (Opcode::Umulhi, &[I8, I8]),
                // https://github.com/bytecodealliance/wasmtime/issues/4756
                (Opcode::Udiv, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4770
                (Opcode::Sdiv, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5474
                (Opcode::Urem, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5474
                (Opcode::Srem, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5466
                (Opcode::Iabs, &[I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/3370
                (Opcode::Smin, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/3370
                (Opcode::Umin, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/3370
                (Opcode::Smax, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/3370
                (Opcode::Umax, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::Band, &[F32, F32]),
                (Opcode::Band, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::Bor, &[F32, F32]),
                (Opcode::Bor, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::Bxor, &[F32, F32]),
                (Opcode::Bxor, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::Bnot, &[F32, F32]),
                (Opcode::Bnot, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/5041
                (Opcode::BandNot, &[I8, I8]),
                (Opcode::BandNot, &[I16, I16]),
                (Opcode::BandNot, &[I32, I32]),
                (Opcode::BandNot, &[I64, I64]),
                (Opcode::BandNot, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::BandNot, &[F32, F32]),
                (Opcode::BandNot, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/5041
                (Opcode::BorNot, &[I8, I8]),
                (Opcode::BorNot, &[I16, I16]),
                (Opcode::BorNot, &[I32, I32]),
                (Opcode::BorNot, &[I64, I64]),
                (Opcode::BorNot, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::BorNot, &[F32, F32]),
                (Opcode::BorNot, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/5041
                (Opcode::BxorNot, &[I8, I8]),
                (Opcode::BxorNot, &[I16, I16]),
                (Opcode::BxorNot, &[I32, I32]),
                (Opcode::BxorNot, &[I64, I64]),
                (Opcode::BxorNot, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::BxorNot, &[F32, F32]),
                (Opcode::BxorNot, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/5107
                (Opcode::Cls, &[I8], &[I8]),
                (Opcode::Cls, &[I16], &[I16]),
                (Opcode::Cls, &[I32], &[I32]),
                (Opcode::Cls, &[I64], &[I64]),
                (Opcode::Cls, &[I128], &[I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5197
                (Opcode::Bitselect, &[I8, I8, I8]),
                (Opcode::Bitselect, &[I16, I16, I16]),
                (Opcode::Bitselect, &[I32, I32, I32]),
                (Opcode::Bitselect, &[I64, I64, I64]),
                (Opcode::Bitselect, &[I128, I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4897
                // https://github.com/bytecodealliance/wasmtime/issues/4899
                (Opcode::FcvtToUint, &[F32], &[I8]),
                (Opcode::FcvtToUint, &[F32], &[I16]),
                (Opcode::FcvtToUint, &[F32], &[I128]),
                (Opcode::FcvtToUint, &[F64], &[I8]),
                (Opcode::FcvtToUint, &[F64], &[I16]),
                (Opcode::FcvtToUint, &[F64], &[I128]),
                (Opcode::FcvtToUint, &[F32X4], &[I32X4]),
                (Opcode::FcvtToUint, &[F64X2], &[I64X2]),
                // https://github.com/bytecodealliance/wasmtime/issues/4897
                // https://github.com/bytecodealliance/wasmtime/issues/4899
                (Opcode::FcvtToUintSat, &[F32], &[I8]),
                (Opcode::FcvtToUintSat, &[F32], &[I16]),
                (Opcode::FcvtToUintSat, &[F32], &[I128]),
                (Opcode::FcvtToUintSat, &[F64], &[I8]),
                (Opcode::FcvtToUintSat, &[F64], &[I16]),
                (Opcode::FcvtToUintSat, &[F64], &[I128]),
                (Opcode::FcvtToUintSat, &[F64X2], &[I64X2]),
                // https://github.com/bytecodealliance/wasmtime/issues/4897
                // https://github.com/bytecodealliance/wasmtime/issues/4899
                (Opcode::FcvtToSint, &[F32], &[I8]),
                (Opcode::FcvtToSint, &[F32], &[I16]),
                (Opcode::FcvtToSint, &[F32], &[I128]),
                (Opcode::FcvtToSint, &[F64], &[I8]),
                (Opcode::FcvtToSint, &[F64], &[I16]),
                (Opcode::FcvtToSint, &[F64], &[I128]),
                (Opcode::FcvtToSint, &[F32X4], &[I32X4]),
                (Opcode::FcvtToSint, &[F64X2], &[I64X2]),
                // https://github.com/bytecodealliance/wasmtime/issues/4897
                // https://github.com/bytecodealliance/wasmtime/issues/4899
                (Opcode::FcvtToSintSat, &[F32], &[I8]),
                (Opcode::FcvtToSintSat, &[F32], &[I16]),
                (Opcode::FcvtToSintSat, &[F32], &[I128]),
                (Opcode::FcvtToSintSat, &[F64], &[I8]),
                (Opcode::FcvtToSintSat, &[F64], &[I16]),
                (Opcode::FcvtToSintSat, &[F64], &[I128]),
                (Opcode::FcvtToSintSat, &[F64X2], &[I64X2]),
                // https://github.com/bytecodealliance/wasmtime/issues/4900
                (Opcode::FcvtFromUint, &[I128], &[F32]),
                (Opcode::FcvtFromUint, &[I128], &[F64]),
                // This has a lowering, but only when preceded by `uwiden_low`.
                (Opcode::FcvtFromUint, &[I64X2], &[F64X2]),
                // https://github.com/bytecodealliance/wasmtime/issues/4900
                (Opcode::FcvtFromSint, &[I128], &[F32]),
                (Opcode::FcvtFromSint, &[I128], &[F64]),
                (Opcode::FcvtFromSint, &[I64X2], &[F64X2]),
                (Opcode::Bmask, &[I8X16]),
                (Opcode::Bmask, &[I16X8]),
                (Opcode::Bmask, &[I32X4]),
                (Opcode::Bmask, &[I64X2]),
                (Opcode::Umulhi, &[I8X16, I8X16]),
                (Opcode::Umulhi, &[I16X8, I16X8]),
                (Opcode::Umulhi, &[I32X4, I32X4]),
                (Opcode::Umulhi, &[I64X2, I64X2]),
                (Opcode::Smulhi, &[I8X16, I8X16]),
                (Opcode::Smulhi, &[I16X8, I16X8]),
                (Opcode::Smulhi, &[I32X4, I32X4]),
                (Opcode::Smulhi, &[I64X2, I64X2]),
                (Opcode::UaddSat, &[I32X4, I32X4]),
                (Opcode::UaddSat, &[I64X2, I64X2]),
                (Opcode::SaddSat, &[I32X4, I32X4]),
                (Opcode::SaddSat, &[I64X2, I64X2]),
                (Opcode::UsubSat, &[I32X4, I32X4]),
                (Opcode::UsubSat, &[I64X2, I64X2]),
                (Opcode::SsubSat, &[I32X4, I32X4]),
                (Opcode::SsubSat, &[I64X2, I64X2]),
                (Opcode::Fcopysign, &[F32X4, F32X4]),
                (Opcode::Fcopysign, &[F64X2, F64X2]),
                (Opcode::Popcnt, &[I8X16]),
                (Opcode::Popcnt, &[I16X8]),
                (Opcode::Popcnt, &[I32X4]),
                (Opcode::Popcnt, &[I64X2]),
                (Opcode::Umax, &[I64X2, I64X2]),
                (Opcode::Smax, &[I64X2, I64X2]),
                (Opcode::Umin, &[I64X2, I64X2]),
                (Opcode::Smin, &[I64X2, I64X2]),
                (Opcode::Bitcast, &[I128], &[_]),
                (Opcode::Bitcast, &[_], &[I128]),
                (Opcode::Uunarrow),
                (Opcode::Snarrow, &[I64X2, I64X2]),
                (Opcode::Unarrow, &[I64X2, I64X2]),
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
                (Opcode::StackStore, &[I8X16], &[]),
                (Opcode::StackStore, &[I16X8], &[]),
                (Opcode::StackStore, &[I32X4], &[]),
                (Opcode::StackStore, &[I64X2], &[]),
                (Opcode::StackStore, &[F32X4], &[]),
                (Opcode::StackStore, &[F64X2], &[]),
                (Opcode::StackLoad, &[], &[I8X16]),
                (Opcode::StackLoad, &[], &[I16X8]),
                (Opcode::StackLoad, &[], &[I32X4]),
                (Opcode::StackLoad, &[], &[I64X2]),
                (Opcode::StackLoad, &[], &[F32X4]),
                (Opcode::StackLoad, &[], &[F64X2]),
            )
        }

        Architecture::Aarch64(_) => {
            exceptions!(
                (Opcode::IaddCout, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4864
                (Opcode::Udiv, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4864
                (Opcode::Sdiv, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5472
                (Opcode::Urem, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5472
                (Opcode::Srem, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5467
                (Opcode::Iabs, &[I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4313
                (Opcode::Smin, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4313
                (Opcode::Umin, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4313
                (Opcode::Smax, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4313
                (Opcode::Umax, &[I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::Band, &[F32, F32]),
                (Opcode::Band, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::Bor, &[F32, F32]),
                (Opcode::Bor, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::Bxor, &[F32, F32]),
                (Opcode::Bxor, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::Bnot, &[F32, F32]),
                (Opcode::Bnot, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::BandNot, &[F32, F32]),
                (Opcode::BandNot, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::BorNot, &[F32, F32]),
                (Opcode::BorNot, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4870
                (Opcode::BxorNot, &[F32, F32]),
                (Opcode::BxorNot, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/5198
                (Opcode::Bitselect, &[I128, I128, I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/4934
                (Opcode::FcvtToUint, &[F32]),
                (Opcode::FcvtToUint, &[F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4934
                (Opcode::FcvtToUintSat, &[F32]),
                (Opcode::FcvtToUintSat, &[F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4934
                (Opcode::FcvtToSint, &[F32]),
                (Opcode::FcvtToSint, &[F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4934
                (Opcode::FcvtToSintSat, &[F32]),
                (Opcode::FcvtToSintSat, &[F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4933
                (Opcode::FcvtFromUint, &[I128], &[F32]),
                (Opcode::FcvtFromUint, &[I128], &[F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/4933
                (Opcode::FcvtFromSint, &[I128], &[F32]),
                (Opcode::FcvtFromSint, &[I128], &[F64]),
                (Opcode::Bmask, &[I8X16]),
                (Opcode::Bmask, &[I16X8]),
                (Opcode::Bmask, &[I32X4]),
                (Opcode::Bmask, &[I64X2]),
                (Opcode::Umulhi, &[I8X16, I8X16]),
                (Opcode::Umulhi, &[I16X8, I16X8]),
                (Opcode::Umulhi, &[I32X4, I32X4]),
                (Opcode::Umulhi, &[I64X2, I64X2]),
                (Opcode::Smulhi, &[I8X16, I8X16]),
                (Opcode::Smulhi, &[I16X8, I16X8]),
                (Opcode::Smulhi, &[I32X4, I32X4]),
                (Opcode::Smulhi, &[I64X2, I64X2]),
                (Opcode::Popcnt, &[I16X8]),
                (Opcode::Popcnt, &[I32X4]),
                (Opcode::Popcnt, &[I64X2]),
                // Nothing wrong with this select. But we have an isle rule that can optimize it
                // into a `min`/`max` instructions, which we don't have implemented yet.
                (Opcode::Select, &[I8, I128, I128]),
            )
        }

        Architecture::S390x => {
            exceptions!(
                (Opcode::IaddCout),
                (Opcode::Udiv, &[I128, I128]),
                (Opcode::Sdiv, &[I128, I128]),
                (Opcode::Urem, &[I128, I128]),
                (Opcode::Srem, &[I128, I128]),
                (Opcode::Band, &[F32, F32]),
                (Opcode::Band, &[F64, F64]),
                (Opcode::Bor, &[F32, F32]),
                (Opcode::Bor, &[F64, F64]),
                (Opcode::Bxor, &[F32, F32]),
                (Opcode::Bxor, &[F64, F64]),
                (Opcode::Bnot, &[F32, F32]),
                (Opcode::Bnot, &[F64, F64]),
                (Opcode::BandNot, &[F32, F32]),
                (Opcode::BandNot, &[F64, F64]),
                (Opcode::BorNot, &[F32, F32]),
                (Opcode::BorNot, &[F64, F64]),
                (Opcode::BxorNot, &[F32, F32]),
                (Opcode::BxorNot, &[F64, F64]),
                (Opcode::FcvtToUint, &[F32], &[I128]),
                (Opcode::FcvtToUint, &[F64], &[I128]),
                (Opcode::FcvtToUintSat, &[F32], &[I128]),
                (Opcode::FcvtToUintSat, &[F64], &[I128]),
                (Opcode::FcvtToSint, &[F32], &[I128]),
                (Opcode::FcvtToSint, &[F64], &[I128]),
                (Opcode::FcvtToSintSat, &[F32], &[I128]),
                (Opcode::FcvtToSintSat, &[F64], &[I128]),
                (Opcode::FcvtFromUint, &[I128], &[F32]),
                (Opcode::FcvtFromUint, &[I128], &[F64]),
                (Opcode::FcvtFromSint, &[I128], &[F32]),
                (Opcode::FcvtFromSint, &[I128], &[F64]),
                (Opcode::Bmask, &[I8X16]),
                (Opcode::Bmask, &[I16X8]),
                (Opcode::Bmask, &[I32X4]),
                (Opcode::Bmask, &[I64X2]),
                (Opcode::SsubSat, &[I64X2, I64X2]),
                (Opcode::SaddSat, &[I64X2, I64X2]),
            )
        }

        Architecture::Riscv64(_) => {
            // RISC-V Does not support SIMD at all
            let is_simd = args.iter().chain(rets).any(|t| t.is_vector());
            if is_simd {
                return false;
            }

            exceptions!(
                // TODO
                (Opcode::IaddCout),
                // TODO
                (Opcode::Udiv, &[I128, I128]),
                // TODO
                (Opcode::Sdiv, &[I128, I128]),
                // TODO
                (Opcode::Urem, &[I128, I128]),
                // TODO
                (Opcode::Srem, &[I128, I128]),
                // TODO
                (Opcode::Iabs, &[I128]),
                // TODO
                (Opcode::Bitselect, &[I128, I128, I128]),
                // TODO
                (Opcode::Bswap),
                // https://github.com/bytecodealliance/wasmtime/issues/5528
                (Opcode::FcvtToUint, &[F32], &[I8]),
                (Opcode::FcvtToUint, &[F32], &[I16]),
                // TODO
                (Opcode::FcvtToUint, &[F32], &[I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5528
                (Opcode::FcvtToUint, &[F64], &[I8]),
                (Opcode::FcvtToUint, &[F64], &[I16]),
                // TODO
                (Opcode::FcvtToUint, &[F64], &[I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5528
                (Opcode::FcvtToUintSat, &[F32], &[I8]),
                (Opcode::FcvtToUintSat, &[F32], &[I16]),
                // TODO
                (Opcode::FcvtToUintSat, &[F32], &[I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5528
                (Opcode::FcvtToUintSat, &[F64], &[I8]),
                (Opcode::FcvtToUintSat, &[F64], &[I16]),
                // TODO
                (Opcode::FcvtToUintSat, &[F64], &[I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5528
                (Opcode::FcvtToSint, &[F32], &[I8]),
                (Opcode::FcvtToSint, &[F32], &[I16]),
                // TODO
                (Opcode::FcvtToSint, &[F32], &[I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5528
                (Opcode::FcvtToSint, &[F64], &[I8]),
                (Opcode::FcvtToSint, &[F64], &[I16]),
                // TODO
                (Opcode::FcvtToSint, &[F64], &[I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5528
                (Opcode::FcvtToSintSat, &[F32], &[I8]),
                (Opcode::FcvtToSintSat, &[F32], &[I16]),
                // TODO
                (Opcode::FcvtToSintSat, &[F32], &[I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5528
                (Opcode::FcvtToSintSat, &[F64], &[I8]),
                (Opcode::FcvtToSintSat, &[F64], &[I16]),
                // TODO
                (Opcode::FcvtToSintSat, &[F64], &[I128]),
                // https://github.com/bytecodealliance/wasmtime/issues/5528
                (Opcode::FcvtFromUint, &[I8], &[F32]),
                (Opcode::FcvtFromUint, &[I8], &[F64]),
                (Opcode::FcvtFromUint, &[I16], &[F32]),
                (Opcode::FcvtFromUint, &[I16], &[F64]),
                // TODO
                (Opcode::FcvtFromUint, &[I128], &[F32]),
                (Opcode::FcvtFromUint, &[I128], &[F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/5528
                (Opcode::FcvtFromSint, &[I8], &[F32]),
                (Opcode::FcvtFromSint, &[I8], &[F64]),
                (Opcode::FcvtFromSint, &[I16], &[F32]),
                (Opcode::FcvtFromSint, &[I16], &[F64]),
                // TODO
                (Opcode::FcvtFromSint, &[I128], &[F32]),
                (Opcode::FcvtFromSint, &[I128], &[F64]),
                // TODO
                (Opcode::BandNot, &[F32, F32]),
                (Opcode::BandNot, &[F64, F64]),
                // TODO
                (Opcode::BorNot, &[F32, F32]),
                (Opcode::BorNot, &[F64, F64]),
                // TODO
                (Opcode::BxorNot, &[F32, F32]),
                (Opcode::BxorNot, &[F64, F64]),
                // https://github.com/bytecodealliance/wasmtime/issues/5884
                (Opcode::AtomicRmw),
            )
        }

        _ => true,
    }
}

type OpcodeSignature = (
    Opcode,
    &'static [Type], // Args
    &'static [Type], // Rets
);

// TODO: Derive this from the `cranelift-meta` generator.
#[rustfmt::skip]
const OPCODE_SIGNATURES: &[OpcodeSignature] = &[
    (Opcode::Nop, &[], &[]),
    // Iadd
    (Opcode::Iadd, &[I8, I8], &[I8]),
    (Opcode::Iadd, &[I16, I16], &[I16]),
    (Opcode::Iadd, &[I32, I32], &[I32]),
    (Opcode::Iadd, &[I64, I64], &[I64]),
    (Opcode::Iadd, &[I128, I128], &[I128]),
    // IaddCout
    (Opcode::IaddCout, &[I8, I8], &[I8, I8]),
    (Opcode::IaddCout, &[I16, I16], &[I16, I8]),
    (Opcode::IaddCout, &[I32, I32], &[I32, I8]),
    (Opcode::IaddCout, &[I64, I64], &[I64, I8]),
    (Opcode::IaddCout, &[I128, I128], &[I128, I8]),
    // UaddSat
    (Opcode::UaddSat, &[I8X16, I8X16], &[I8X16]),
    (Opcode::UaddSat, &[I16X8, I16X8], &[I16X8]),
    (Opcode::UaddSat, &[I32X4, I32X4], &[I32X4]),
    (Opcode::UaddSat, &[I64X2, I64X2], &[I64X2]),
    // SaddSat
    (Opcode::SaddSat, &[I8X16, I8X16], &[I8X16]),
    (Opcode::SaddSat, &[I16X8, I16X8], &[I16X8]),
    (Opcode::SaddSat, &[I32X4, I32X4], &[I32X4]),
    (Opcode::SaddSat, &[I64X2, I64X2], &[I64X2]),
    // Isub
    (Opcode::Isub, &[I8, I8], &[I8]),
    (Opcode::Isub, &[I16, I16], &[I16]),
    (Opcode::Isub, &[I32, I32], &[I32]),
    (Opcode::Isub, &[I64, I64], &[I64]),
    (Opcode::Isub, &[I128, I128], &[I128]),
    // UsubSat
    (Opcode::UsubSat, &[I8X16, I8X16], &[I8X16]),
    (Opcode::UsubSat, &[I16X8, I16X8], &[I16X8]),
    (Opcode::UsubSat, &[I32X4, I32X4], &[I32X4]),
    (Opcode::UsubSat, &[I64X2, I64X2], &[I64X2]),
    // SsubSat
    (Opcode::SsubSat, &[I8X16, I8X16], &[I8X16]),
    (Opcode::SsubSat, &[I16X8, I16X8], &[I16X8]),
    (Opcode::SsubSat, &[I32X4, I32X4], &[I32X4]),
    (Opcode::SsubSat, &[I64X2, I64X2], &[I64X2]),
    // Imul
    (Opcode::Imul, &[I8, I8], &[I8]),
    (Opcode::Imul, &[I16, I16], &[I16]),
    (Opcode::Imul, &[I32, I32], &[I32]),
    (Opcode::Imul, &[I64, I64], &[I64]),
    (Opcode::Imul, &[I128, I128], &[I128]),
    // Smulhi
    (Opcode::Smulhi, &[I8, I8], &[I8]),
    (Opcode::Smulhi, &[I16, I16], &[I16]),
    (Opcode::Smulhi, &[I32, I32], &[I32]),
    (Opcode::Smulhi, &[I64, I64], &[I64]),
    (Opcode::Smulhi, &[I8X16, I8X16], &[I8X16]),
    (Opcode::Smulhi, &[I16X8, I16X8], &[I16X8]),
    (Opcode::Smulhi, &[I32X4, I32X4], &[I32X4]),
    (Opcode::Smulhi, &[I64X2, I64X2], &[I64X2]),
    // Umulhi
    (Opcode::Umulhi, &[I8, I8], &[I8]),
    (Opcode::Umulhi, &[I16, I16], &[I16]),
    (Opcode::Umulhi, &[I32, I32], &[I32]),
    (Opcode::Umulhi, &[I64, I64], &[I64]),
    (Opcode::Umulhi, &[I8X16, I8X16], &[I8X16]),
    (Opcode::Umulhi, &[I16X8, I16X8], &[I16X8]),
    (Opcode::Umulhi, &[I32X4, I32X4], &[I32X4]),
    (Opcode::Umulhi, &[I64X2, I64X2], &[I64X2]),
    // Udiv
    (Opcode::Udiv, &[I8, I8], &[I8]),
    (Opcode::Udiv, &[I16, I16], &[I16]),
    (Opcode::Udiv, &[I32, I32], &[I32]),
    (Opcode::Udiv, &[I64, I64], &[I64]),
    (Opcode::Udiv, &[I128, I128], &[I128]),
    // Sdiv
    (Opcode::Sdiv, &[I8, I8], &[I8]),
    (Opcode::Sdiv, &[I16, I16], &[I16]),
    (Opcode::Sdiv, &[I32, I32], &[I32]),
    (Opcode::Sdiv, &[I64, I64], &[I64]),
    (Opcode::Sdiv, &[I128, I128], &[I128]),
    // Urem
    (Opcode::Urem, &[I8, I8], &[I8]),
    (Opcode::Urem, &[I16, I16], &[I16]),
    (Opcode::Urem, &[I32, I32], &[I32]),
    (Opcode::Urem, &[I64, I64], &[I64]),
    (Opcode::Urem, &[I128, I128], &[I128]),
    // Srem
    (Opcode::Srem, &[I8, I8], &[I8]),
    (Opcode::Srem, &[I16, I16], &[I16]),
    (Opcode::Srem, &[I32, I32], &[I32]),
    (Opcode::Srem, &[I64, I64], &[I64]),
    (Opcode::Srem, &[I128, I128], &[I128]),
    // Ineg
    (Opcode::Ineg, &[I8, I8], &[I8]),
    (Opcode::Ineg, &[I16, I16], &[I16]),
    (Opcode::Ineg, &[I32, I32], &[I32]),
    (Opcode::Ineg, &[I64, I64], &[I64]),
    (Opcode::Ineg, &[I128, I128], &[I128]),
    // Iabs
    (Opcode::Iabs, &[I8], &[I8]),
    (Opcode::Iabs, &[I16], &[I16]),
    (Opcode::Iabs, &[I32], &[I32]),
    (Opcode::Iabs, &[I64], &[I64]),
    (Opcode::Iabs, &[I128], &[I128]),
    (Opcode::Iabs, &[I8X16, I8X16], &[I8X16]),
    (Opcode::Iabs, &[I16X8, I16X8], &[I16X8]),
    (Opcode::Iabs, &[I32X4, I32X4], &[I32X4]),
    (Opcode::Iabs, &[I64X2, I64X2], &[I64X2]),
    // Smin
    (Opcode::Smin, &[I8, I8], &[I8]),
    (Opcode::Smin, &[I16, I16], &[I16]),
    (Opcode::Smin, &[I32, I32], &[I32]),
    (Opcode::Smin, &[I64, I64], &[I64]),
    (Opcode::Smin, &[I128, I128], &[I128]),
    (Opcode::Smin, &[I8X16, I8X16], &[I8X16]),
    (Opcode::Smin, &[I16X8, I16X8], &[I16X8]),
    (Opcode::Smin, &[I32X4, I32X4], &[I32X4]),
    (Opcode::Smin, &[I64X2, I64X2], &[I64X2]),
    // Umin
    (Opcode::Umin, &[I8, I8], &[I8]),
    (Opcode::Umin, &[I16, I16], &[I16]),
    (Opcode::Umin, &[I32, I32], &[I32]),
    (Opcode::Umin, &[I64, I64], &[I64]),
    (Opcode::Umin, &[I128, I128], &[I128]),
    (Opcode::Umin, &[I8X16, I8X16], &[I8X16]),
    (Opcode::Umin, &[I16X8, I16X8], &[I16X8]),
    (Opcode::Umin, &[I32X4, I32X4], &[I32X4]),
    (Opcode::Umin, &[I64X2, I64X2], &[I64X2]),
    // Smax
    (Opcode::Smax, &[I8, I8], &[I8]),
    (Opcode::Smax, &[I16, I16], &[I16]),
    (Opcode::Smax, &[I32, I32], &[I32]),
    (Opcode::Smax, &[I64, I64], &[I64]),
    (Opcode::Smax, &[I128, I128], &[I128]),
    (Opcode::Smax, &[I8X16, I8X16], &[I8X16]),
    (Opcode::Smax, &[I16X8, I16X8], &[I16X8]),
    (Opcode::Smax, &[I32X4, I32X4], &[I32X4]),
    (Opcode::Smax, &[I64X2, I64X2], &[I64X2]),
    // Umax
    (Opcode::Umax, &[I8, I8], &[I8]),
    (Opcode::Umax, &[I16, I16], &[I16]),
    (Opcode::Umax, &[I32, I32], &[I32]),
    (Opcode::Umax, &[I64, I64], &[I64]),
    (Opcode::Umax, &[I128, I128], &[I128]),
    (Opcode::Umax, &[I8X16, I8X16], &[I8X16]),
    (Opcode::Umax, &[I16X8, I16X8], &[I16X8]),
    (Opcode::Umax, &[I32X4, I32X4], &[I32X4]),
    (Opcode::Umax, &[I64X2, I64X2], &[I64X2]),
    // Rotr
    (Opcode::Rotr, &[I8, I8], &[I8]),
    (Opcode::Rotr, &[I8, I16], &[I8]),
    (Opcode::Rotr, &[I8, I32], &[I8]),
    (Opcode::Rotr, &[I8, I64], &[I8]),
    (Opcode::Rotr, &[I8, I128], &[I8]),
    (Opcode::Rotr, &[I16, I8], &[I16]),
    (Opcode::Rotr, &[I16, I16], &[I16]),
    (Opcode::Rotr, &[I16, I32], &[I16]),
    (Opcode::Rotr, &[I16, I64], &[I16]),
    (Opcode::Rotr, &[I16, I128], &[I16]),
    (Opcode::Rotr, &[I32, I8], &[I32]),
    (Opcode::Rotr, &[I32, I16], &[I32]),
    (Opcode::Rotr, &[I32, I32], &[I32]),
    (Opcode::Rotr, &[I32, I64], &[I32]),
    (Opcode::Rotr, &[I32, I128], &[I32]),
    (Opcode::Rotr, &[I64, I8], &[I64]),
    (Opcode::Rotr, &[I64, I16], &[I64]),
    (Opcode::Rotr, &[I64, I32], &[I64]),
    (Opcode::Rotr, &[I64, I64], &[I64]),
    (Opcode::Rotr, &[I64, I128], &[I64]),
    (Opcode::Rotr, &[I128, I8], &[I128]),
    (Opcode::Rotr, &[I128, I16], &[I128]),
    (Opcode::Rotr, &[I128, I32], &[I128]),
    (Opcode::Rotr, &[I128, I64], &[I128]),
    (Opcode::Rotr, &[I128, I128], &[I128]),
    // Rotl
    (Opcode::Rotl, &[I8, I8], &[I8]),
    (Opcode::Rotl, &[I8, I16], &[I8]),
    (Opcode::Rotl, &[I8, I32], &[I8]),
    (Opcode::Rotl, &[I8, I64], &[I8]),
    (Opcode::Rotl, &[I8, I128], &[I8]),
    (Opcode::Rotl, &[I16, I8], &[I16]),
    (Opcode::Rotl, &[I16, I16], &[I16]),
    (Opcode::Rotl, &[I16, I32], &[I16]),
    (Opcode::Rotl, &[I16, I64], &[I16]),
    (Opcode::Rotl, &[I16, I128], &[I16]),
    (Opcode::Rotl, &[I32, I8], &[I32]),
    (Opcode::Rotl, &[I32, I16], &[I32]),
    (Opcode::Rotl, &[I32, I32], &[I32]),
    (Opcode::Rotl, &[I32, I64], &[I32]),
    (Opcode::Rotl, &[I32, I128], &[I32]),
    (Opcode::Rotl, &[I64, I8], &[I64]),
    (Opcode::Rotl, &[I64, I16], &[I64]),
    (Opcode::Rotl, &[I64, I32], &[I64]),
    (Opcode::Rotl, &[I64, I64], &[I64]),
    (Opcode::Rotl, &[I64, I128], &[I64]),
    (Opcode::Rotl, &[I128, I8], &[I128]),
    (Opcode::Rotl, &[I128, I16], &[I128]),
    (Opcode::Rotl, &[I128, I32], &[I128]),
    (Opcode::Rotl, &[I128, I64], &[I128]),
    (Opcode::Rotl, &[I128, I128], &[I128]),
    // Ishl
    (Opcode::Ishl, &[I8, I8], &[I8]),
    (Opcode::Ishl, &[I8, I16], &[I8]),
    (Opcode::Ishl, &[I8, I32], &[I8]),
    (Opcode::Ishl, &[I8, I64], &[I8]),
    (Opcode::Ishl, &[I8, I128], &[I8]),
    (Opcode::Ishl, &[I16, I8], &[I16]),
    (Opcode::Ishl, &[I16, I16], &[I16]),
    (Opcode::Ishl, &[I16, I32], &[I16]),
    (Opcode::Ishl, &[I16, I64], &[I16]),
    (Opcode::Ishl, &[I16, I128], &[I16]),
    (Opcode::Ishl, &[I32, I8], &[I32]),
    (Opcode::Ishl, &[I32, I16], &[I32]),
    (Opcode::Ishl, &[I32, I32], &[I32]),
    (Opcode::Ishl, &[I32, I64], &[I32]),
    (Opcode::Ishl, &[I32, I128], &[I32]),
    (Opcode::Ishl, &[I64, I8], &[I64]),
    (Opcode::Ishl, &[I64, I16], &[I64]),
    (Opcode::Ishl, &[I64, I32], &[I64]),
    (Opcode::Ishl, &[I64, I64], &[I64]),
    (Opcode::Ishl, &[I64, I128], &[I64]),
    (Opcode::Ishl, &[I128, I8], &[I128]),
    (Opcode::Ishl, &[I128, I16], &[I128]),
    (Opcode::Ishl, &[I128, I32], &[I128]),
    (Opcode::Ishl, &[I128, I64], &[I128]),
    (Opcode::Ishl, &[I128, I128], &[I128]),
    // Sshr
    (Opcode::Sshr, &[I8, I8], &[I8]),
    (Opcode::Sshr, &[I8, I16], &[I8]),
    (Opcode::Sshr, &[I8, I32], &[I8]),
    (Opcode::Sshr, &[I8, I64], &[I8]),
    (Opcode::Sshr, &[I8, I128], &[I8]),
    (Opcode::Sshr, &[I16, I8], &[I16]),
    (Opcode::Sshr, &[I16, I16], &[I16]),
    (Opcode::Sshr, &[I16, I32], &[I16]),
    (Opcode::Sshr, &[I16, I64], &[I16]),
    (Opcode::Sshr, &[I16, I128], &[I16]),
    (Opcode::Sshr, &[I32, I8], &[I32]),
    (Opcode::Sshr, &[I32, I16], &[I32]),
    (Opcode::Sshr, &[I32, I32], &[I32]),
    (Opcode::Sshr, &[I32, I64], &[I32]),
    (Opcode::Sshr, &[I32, I128], &[I32]),
    (Opcode::Sshr, &[I64, I8], &[I64]),
    (Opcode::Sshr, &[I64, I16], &[I64]),
    (Opcode::Sshr, &[I64, I32], &[I64]),
    (Opcode::Sshr, &[I64, I64], &[I64]),
    (Opcode::Sshr, &[I64, I128], &[I64]),
    (Opcode::Sshr, &[I128, I8], &[I128]),
    (Opcode::Sshr, &[I128, I16], &[I128]),
    (Opcode::Sshr, &[I128, I32], &[I128]),
    (Opcode::Sshr, &[I128, I64], &[I128]),
    (Opcode::Sshr, &[I128, I128], &[I128]),
    // Ushr
    (Opcode::Ushr, &[I8, I8], &[I8]),
    (Opcode::Ushr, &[I8, I16], &[I8]),
    (Opcode::Ushr, &[I8, I32], &[I8]),
    (Opcode::Ushr, &[I8, I64], &[I8]),
    (Opcode::Ushr, &[I8, I128], &[I8]),
    (Opcode::Ushr, &[I16, I8], &[I16]),
    (Opcode::Ushr, &[I16, I16], &[I16]),
    (Opcode::Ushr, &[I16, I32], &[I16]),
    (Opcode::Ushr, &[I16, I64], &[I16]),
    (Opcode::Ushr, &[I16, I128], &[I16]),
    (Opcode::Ushr, &[I32, I8], &[I32]),
    (Opcode::Ushr, &[I32, I16], &[I32]),
    (Opcode::Ushr, &[I32, I32], &[I32]),
    (Opcode::Ushr, &[I32, I64], &[I32]),
    (Opcode::Ushr, &[I32, I128], &[I32]),
    (Opcode::Ushr, &[I64, I8], &[I64]),
    (Opcode::Ushr, &[I64, I16], &[I64]),
    (Opcode::Ushr, &[I64, I32], &[I64]),
    (Opcode::Ushr, &[I64, I64], &[I64]),
    (Opcode::Ushr, &[I64, I128], &[I64]),
    (Opcode::Ushr, &[I128, I8], &[I128]),
    (Opcode::Ushr, &[I128, I16], &[I128]),
    (Opcode::Ushr, &[I128, I32], &[I128]),
    (Opcode::Ushr, &[I128, I64], &[I128]),
    (Opcode::Ushr, &[I128, I128], &[I128]),
    // Uextend
    (Opcode::Uextend, &[I8], &[I16]),
    (Opcode::Uextend, &[I8], &[I32]),
    (Opcode::Uextend, &[I8], &[I64]),
    (Opcode::Uextend, &[I8], &[I128]),
    (Opcode::Uextend, &[I16], &[I32]),
    (Opcode::Uextend, &[I16], &[I64]),
    (Opcode::Uextend, &[I16], &[I128]),
    (Opcode::Uextend, &[I32], &[I64]),
    (Opcode::Uextend, &[I32], &[I128]),
    (Opcode::Uextend, &[I64], &[I128]),
    // Sextend
    (Opcode::Sextend, &[I8], &[I16]),
    (Opcode::Sextend, &[I8], &[I32]),
    (Opcode::Sextend, &[I8], &[I64]),
    (Opcode::Sextend, &[I8], &[I128]),
    (Opcode::Sextend, &[I16], &[I32]),
    (Opcode::Sextend, &[I16], &[I64]),
    (Opcode::Sextend, &[I16], &[I128]),
    (Opcode::Sextend, &[I32], &[I64]),
    (Opcode::Sextend, &[I32], &[I128]),
    (Opcode::Sextend, &[I64], &[I128]),
    // Ireduce
    (Opcode::Ireduce, &[I16], &[I8]),
    (Opcode::Ireduce, &[I32], &[I8]),
    (Opcode::Ireduce, &[I32], &[I16]),
    (Opcode::Ireduce, &[I64], &[I8]),
    (Opcode::Ireduce, &[I64], &[I16]),
    (Opcode::Ireduce, &[I64], &[I32]),
    (Opcode::Ireduce, &[I128], &[I8]),
    (Opcode::Ireduce, &[I128], &[I16]),
    (Opcode::Ireduce, &[I128], &[I32]),
    (Opcode::Ireduce, &[I128], &[I64]),
    // Isplit
    (Opcode::Isplit, &[I128], &[I64, I64]),
    // Iconcat
    (Opcode::Iconcat, &[I64, I64], &[I128]),
    // Band
    (Opcode::Band, &[I8, I8], &[I8]),
    (Opcode::Band, &[I16, I16], &[I16]),
    (Opcode::Band, &[I32, I32], &[I32]),
    (Opcode::Band, &[I64, I64], &[I64]),
    (Opcode::Band, &[I128, I128], &[I128]),
    (Opcode::Band, &[F32, F32], &[F32]),
    (Opcode::Band, &[F64, F64], &[F64]),
    // Bor
    (Opcode::Bor, &[I8, I8], &[I8]),
    (Opcode::Bor, &[I16, I16], &[I16]),
    (Opcode::Bor, &[I32, I32], &[I32]),
    (Opcode::Bor, &[I64, I64], &[I64]),
    (Opcode::Bor, &[I128, I128], &[I128]),
    (Opcode::Bor, &[F32, F32], &[F32]),
    (Opcode::Bor, &[F64, F64], &[F64]),
    // Bxor
    (Opcode::Bxor, &[I8, I8], &[I8]),
    (Opcode::Bxor, &[I16, I16], &[I16]),
    (Opcode::Bxor, &[I32, I32], &[I32]),
    (Opcode::Bxor, &[I64, I64], &[I64]),
    (Opcode::Bxor, &[I128, I128], &[I128]),
    (Opcode::Bxor, &[F32, F32], &[F32]),
    (Opcode::Bxor, &[F64, F64], &[F64]),
    // Bnot
    (Opcode::Bnot, &[I8, I8], &[I8]),
    (Opcode::Bnot, &[I16, I16], &[I16]),
    (Opcode::Bnot, &[I32, I32], &[I32]),
    (Opcode::Bnot, &[I64, I64], &[I64]),
    (Opcode::Bnot, &[I128, I128], &[I128]),
    (Opcode::Bnot, &[F32, F32], &[F32]),
    (Opcode::Bnot, &[F64, F64], &[F64]),
    // BandNot
    (Opcode::BandNot, &[I8, I8], &[I8]),
    (Opcode::BandNot, &[I16, I16], &[I16]),
    (Opcode::BandNot, &[I32, I32], &[I32]),
    (Opcode::BandNot, &[I64, I64], &[I64]),
    (Opcode::BandNot, &[I128, I128], &[I128]),
    (Opcode::BandNot, &[F32, F32], &[F32]),
    (Opcode::BandNot, &[F64, F64], &[F64]),
    // BorNot
    (Opcode::BorNot, &[I8, I8], &[I8]),
    (Opcode::BorNot, &[I16, I16], &[I16]),
    (Opcode::BorNot, &[I32, I32], &[I32]),
    (Opcode::BorNot, &[I64, I64], &[I64]),
    (Opcode::BorNot, &[I128, I128], &[I128]),
    (Opcode::BorNot, &[F32, F32], &[F32]),
    (Opcode::BorNot, &[F64, F64], &[F64]),
    // BxorNot
    (Opcode::BxorNot, &[I8, I8], &[I8]),
    (Opcode::BxorNot, &[I16, I16], &[I16]),
    (Opcode::BxorNot, &[I32, I32], &[I32]),
    (Opcode::BxorNot, &[I64, I64], &[I64]),
    (Opcode::BxorNot, &[I128, I128], &[I128]),
    (Opcode::BxorNot, &[F32, F32], &[F32]),
    (Opcode::BxorNot, &[F64, F64], &[F64]),
    // Bitrev
    (Opcode::Bitrev, &[I8], &[I8]),
    (Opcode::Bitrev, &[I16], &[I16]),
    (Opcode::Bitrev, &[I32], &[I32]),
    (Opcode::Bitrev, &[I64], &[I64]),
    (Opcode::Bitrev, &[I128], &[I128]),
    // Clz
    (Opcode::Clz, &[I8], &[I8]),
    (Opcode::Clz, &[I16], &[I16]),
    (Opcode::Clz, &[I32], &[I32]),
    (Opcode::Clz, &[I64], &[I64]),
    (Opcode::Clz, &[I128], &[I128]),
    // Cls
    (Opcode::Cls, &[I8], &[I8]),
    (Opcode::Cls, &[I16], &[I16]),
    (Opcode::Cls, &[I32], &[I32]),
    (Opcode::Cls, &[I64], &[I64]),
    (Opcode::Cls, &[I128], &[I128]),
    // Ctz
    (Opcode::Ctz, &[I8], &[I8]),
    (Opcode::Ctz, &[I16], &[I16]),
    (Opcode::Ctz, &[I32], &[I32]),
    (Opcode::Ctz, &[I64], &[I64]),
    (Opcode::Ctz, &[I128], &[I128]),
    // Popcnt
    (Opcode::Popcnt, &[I8], &[I8]),
    (Opcode::Popcnt, &[I16], &[I16]),
    (Opcode::Popcnt, &[I32], &[I32]),
    (Opcode::Popcnt, &[I64], &[I64]),
    (Opcode::Popcnt, &[I128], &[I128]),
    (Opcode::Popcnt, &[I8X16], &[I8X16]),
    (Opcode::Popcnt, &[I16X8], &[I16X8]),
    (Opcode::Popcnt, &[I32X4], &[I32X4]),
    (Opcode::Popcnt, &[I64X2], &[I64X2]),
    // Bmask
    (Opcode::Bmask, &[I8], &[I8]),
    (Opcode::Bmask, &[I16], &[I8]),
    (Opcode::Bmask, &[I32], &[I8]),
    (Opcode::Bmask, &[I64], &[I8]),
    (Opcode::Bmask, &[I128], &[I8]),
    (Opcode::Bmask, &[I8], &[I16]),
    (Opcode::Bmask, &[I16], &[I16]),
    (Opcode::Bmask, &[I32], &[I16]),
    (Opcode::Bmask, &[I64], &[I16]),
    (Opcode::Bmask, &[I128], &[I16]),
    (Opcode::Bmask, &[I8], &[I32]),
    (Opcode::Bmask, &[I16], &[I32]),
    (Opcode::Bmask, &[I32], &[I32]),
    (Opcode::Bmask, &[I64], &[I32]),
    (Opcode::Bmask, &[I128], &[I32]),
    (Opcode::Bmask, &[I8], &[I64]),
    (Opcode::Bmask, &[I16], &[I64]),
    (Opcode::Bmask, &[I32], &[I64]),
    (Opcode::Bmask, &[I64], &[I64]),
    (Opcode::Bmask, &[I128], &[I64]),
    (Opcode::Bmask, &[I8], &[I128]),
    (Opcode::Bmask, &[I16], &[I128]),
    (Opcode::Bmask, &[I32], &[I128]),
    (Opcode::Bmask, &[I64], &[I128]),
    (Opcode::Bmask, &[I128], &[I128]),
    (Opcode::Bmask, &[I8X16], &[I8X16]),
    (Opcode::Bmask, &[I16X8], &[I16X8]),
    (Opcode::Bmask, &[I32X4], &[I32X4]),
    (Opcode::Bmask, &[I64X2], &[I64X2]),
    // Bswap
    (Opcode::Bswap, &[I16], &[I16]),
    (Opcode::Bswap, &[I32], &[I32]),
    (Opcode::Bswap, &[I64], &[I64]),
    (Opcode::Bswap, &[I128], &[I128]),
    // Bitselect
    (Opcode::Bitselect, &[I8, I8, I8], &[I8]),
    (Opcode::Bitselect, &[I16, I16, I16], &[I16]),
    (Opcode::Bitselect, &[I32, I32, I32], &[I32]),
    (Opcode::Bitselect, &[I64, I64, I64], &[I64]),
    (Opcode::Bitselect, &[I128, I128, I128], &[I128]),
    // Select
    (Opcode::Select, &[I8, I8, I8], &[I8]),
    (Opcode::Select, &[I8, I16, I16], &[I16]),
    (Opcode::Select, &[I8, I32, I32], &[I32]),
    (Opcode::Select, &[I8, I64, I64], &[I64]),
    (Opcode::Select, &[I8, I128, I128], &[I128]),
    (Opcode::Select, &[I16, I8, I8], &[I8]),
    (Opcode::Select, &[I16, I16, I16], &[I16]),
    (Opcode::Select, &[I16, I32, I32], &[I32]),
    (Opcode::Select, &[I16, I64, I64], &[I64]),
    (Opcode::Select, &[I16, I128, I128], &[I128]),
    (Opcode::Select, &[I32, I8, I8], &[I8]),
    (Opcode::Select, &[I32, I16, I16], &[I16]),
    (Opcode::Select, &[I32, I32, I32], &[I32]),
    (Opcode::Select, &[I32, I64, I64], &[I64]),
    (Opcode::Select, &[I32, I128, I128], &[I128]),
    (Opcode::Select, &[I64, I8, I8], &[I8]),
    (Opcode::Select, &[I64, I16, I16], &[I16]),
    (Opcode::Select, &[I64, I32, I32], &[I32]),
    (Opcode::Select, &[I64, I64, I64], &[I64]),
    (Opcode::Select, &[I64, I128, I128], &[I128]),
    (Opcode::Select, &[I128, I8, I8], &[I8]),
    (Opcode::Select, &[I128, I16, I16], &[I16]),
    (Opcode::Select, &[I128, I32, I32], &[I32]),
    (Opcode::Select, &[I128, I64, I64], &[I64]),
    (Opcode::Select, &[I128, I128, I128], &[I128]),
    // SelectSpectreGuard
    (Opcode::SelectSpectreGuard, &[I8, I8, I8], &[I8]),
    (Opcode::SelectSpectreGuard, &[I8, I16, I16], &[I16]),
    (Opcode::SelectSpectreGuard, &[I8, I32, I32], &[I32]),
    (Opcode::SelectSpectreGuard, &[I8, I64, I64], &[I64]),
    (Opcode::SelectSpectreGuard, &[I8, I128, I128], &[I128]),
    (Opcode::SelectSpectreGuard, &[I16, I8, I8], &[I8]),
    (Opcode::SelectSpectreGuard, &[I16, I16, I16], &[I16]),
    (Opcode::SelectSpectreGuard, &[I16, I32, I32], &[I32]),
    (Opcode::SelectSpectreGuard, &[I16, I64, I64], &[I64]),
    (Opcode::SelectSpectreGuard, &[I16, I128, I128], &[I128]),
    (Opcode::SelectSpectreGuard, &[I32, I8, I8], &[I8]),
    (Opcode::SelectSpectreGuard, &[I32, I16, I16], &[I16]),
    (Opcode::SelectSpectreGuard, &[I32, I32, I32], &[I32]),
    (Opcode::SelectSpectreGuard, &[I32, I64, I64], &[I64]),
    (Opcode::SelectSpectreGuard, &[I32, I128, I128], &[I128]),
    (Opcode::SelectSpectreGuard, &[I64, I8, I8], &[I8]),
    (Opcode::SelectSpectreGuard, &[I64, I16, I16], &[I16]),
    (Opcode::SelectSpectreGuard, &[I64, I32, I32], &[I32]),
    (Opcode::SelectSpectreGuard, &[I64, I64, I64], &[I64]),
    (Opcode::SelectSpectreGuard, &[I64, I128, I128], &[I128]),
    (Opcode::SelectSpectreGuard, &[I128, I8, I8], &[I8]),
    (Opcode::SelectSpectreGuard, &[I128, I16, I16], &[I16]),
    (Opcode::SelectSpectreGuard, &[I128, I32, I32], &[I32]),
    (Opcode::SelectSpectreGuard, &[I128, I64, I64], &[I64]),
    (Opcode::SelectSpectreGuard, &[I128, I128, I128], &[I128]),
    // Fadd
    (Opcode::Fadd, &[F32, F32], &[F32]),
    (Opcode::Fadd, &[F64, F64], &[F64]),
    // Fmul
    (Opcode::Fmul, &[F32, F32], &[F32]),
    (Opcode::Fmul, &[F64, F64], &[F64]),
    // Fsub
    (Opcode::Fsub, &[F32, F32], &[F32]),
    (Opcode::Fsub, &[F64, F64], &[F64]),
    // Fdiv
    (Opcode::Fdiv, &[F32, F32], &[F32]),
    (Opcode::Fdiv, &[F64, F64], &[F64]),
    // Fmin
    (Opcode::Fmin, &[F32, F32], &[F32]),
    (Opcode::Fmin, &[F64, F64], &[F64]),
    // Fmax
    (Opcode::Fmax, &[F32, F32], &[F32]),
    (Opcode::Fmax, &[F64, F64], &[F64]),
    // FminPseudo
    (Opcode::FminPseudo, &[F32, F32], &[F32]),
    (Opcode::FminPseudo, &[F64, F64], &[F64]),
    // FmaxPseudo
    (Opcode::FmaxPseudo, &[F32, F32], &[F32]),
    (Opcode::FmaxPseudo, &[F64, F64], &[F64]),
    // Fcopysign
    (Opcode::Fcopysign, &[F32, F32], &[F32]),
    (Opcode::Fcopysign, &[F64, F64], &[F64]),
    (Opcode::Fcopysign, &[F32X4, F32X4], &[F32X4]),
    (Opcode::Fcopysign, &[F64X2, F64X2], &[F64X2]),
    // Fma
    (Opcode::Fma, &[F32, F32, F32], &[F32]),
    (Opcode::Fma, &[F64, F64, F64], &[F64]),
    // Fabs
    (Opcode::Fabs, &[F32], &[F32]),
    (Opcode::Fabs, &[F64], &[F64]),
    // Fneg
    (Opcode::Fneg, &[F32], &[F32]),
    (Opcode::Fneg, &[F64], &[F64]),
    // Sqrt
    (Opcode::Sqrt, &[F32], &[F32]),
    (Opcode::Sqrt, &[F64], &[F64]),
    // Ceil
    (Opcode::Ceil, &[F32], &[F32]),
    (Opcode::Ceil, &[F64], &[F64]),
    // Floor
    (Opcode::Floor, &[F32], &[F32]),
    (Opcode::Floor, &[F64], &[F64]),
    // Trunc
    (Opcode::Trunc, &[F32], &[F32]),
    (Opcode::Trunc, &[F64], &[F64]),
    // Nearest
    (Opcode::Nearest, &[F32], &[F32]),
    (Opcode::Nearest, &[F64], &[F64]),
    // Fpromote
    (Opcode::Fpromote, &[F32], &[F64]),
    // Fdemote
    (Opcode::Fdemote, &[F64], &[F32]),
    // FcvtToUint
    (Opcode::FcvtToUint, &[F32], &[I8]),
    (Opcode::FcvtToUint, &[F32], &[I16]),
    (Opcode::FcvtToUint, &[F32], &[I32]),
    (Opcode::FcvtToUint, &[F32], &[I64]),
    (Opcode::FcvtToUint, &[F32], &[I128]),
    (Opcode::FcvtToUint, &[F64], &[I8]),
    (Opcode::FcvtToUint, &[F64], &[I16]),
    (Opcode::FcvtToUint, &[F64], &[I32]),
    (Opcode::FcvtToUint, &[F64], &[I64]),
    (Opcode::FcvtToUint, &[F64], &[I128]),
    // FcvtToUintSat
    (Opcode::FcvtToUintSat, &[F32], &[I8]),
    (Opcode::FcvtToUintSat, &[F32], &[I16]),
    (Opcode::FcvtToUintSat, &[F32], &[I32]),
    (Opcode::FcvtToUintSat, &[F32], &[I64]),
    (Opcode::FcvtToUintSat, &[F32], &[I128]),
    (Opcode::FcvtToUintSat, &[F64], &[I8]),
    (Opcode::FcvtToUintSat, &[F64], &[I16]),
    (Opcode::FcvtToUintSat, &[F64], &[I32]),
    (Opcode::FcvtToUintSat, &[F64], &[I64]),
    (Opcode::FcvtToUintSat, &[F64], &[I128]),
    (Opcode::FcvtToUintSat, &[F32X4], &[I32X4]),
    (Opcode::FcvtToUintSat, &[F64X2], &[I64X2]),
    // FcvtToSint
    (Opcode::FcvtToSint, &[F32], &[I8]),
    (Opcode::FcvtToSint, &[F32], &[I16]),
    (Opcode::FcvtToSint, &[F32], &[I32]),
    (Opcode::FcvtToSint, &[F32], &[I64]),
    (Opcode::FcvtToSint, &[F32], &[I128]),
    (Opcode::FcvtToSint, &[F64], &[I8]),
    (Opcode::FcvtToSint, &[F64], &[I16]),
    (Opcode::FcvtToSint, &[F64], &[I32]),
    (Opcode::FcvtToSint, &[F64], &[I64]),
    (Opcode::FcvtToSint, &[F64], &[I128]),
    // FcvtToSintSat
    (Opcode::FcvtToSintSat, &[F32], &[I8]),
    (Opcode::FcvtToSintSat, &[F32], &[I16]),
    (Opcode::FcvtToSintSat, &[F32], &[I32]),
    (Opcode::FcvtToSintSat, &[F32], &[I64]),
    (Opcode::FcvtToSintSat, &[F32], &[I128]),
    (Opcode::FcvtToSintSat, &[F64], &[I8]),
    (Opcode::FcvtToSintSat, &[F64], &[I16]),
    (Opcode::FcvtToSintSat, &[F64], &[I32]),
    (Opcode::FcvtToSintSat, &[F64], &[I64]),
    (Opcode::FcvtToSintSat, &[F64], &[I128]),
    (Opcode::FcvtToSintSat, &[F32X4], &[I32X4]),
    (Opcode::FcvtToSintSat, &[F64X2], &[I64X2]),
    // FcvtFromUint
    (Opcode::FcvtFromUint, &[I8], &[F32]),
    (Opcode::FcvtFromUint, &[I16], &[F32]),
    (Opcode::FcvtFromUint, &[I32], &[F32]),
    (Opcode::FcvtFromUint, &[I64], &[F32]),
    (Opcode::FcvtFromUint, &[I128], &[F32]),
    (Opcode::FcvtFromUint, &[I8], &[F64]),
    (Opcode::FcvtFromUint, &[I16], &[F64]),
    (Opcode::FcvtFromUint, &[I32], &[F64]),
    (Opcode::FcvtFromUint, &[I64], &[F64]),
    (Opcode::FcvtFromUint, &[I128], &[F64]),
    (Opcode::FcvtFromUint, &[I32X4], &[F32X4]),
    (Opcode::FcvtFromUint, &[I64X2], &[F64X2]),
    // FcvtFromSint
    (Opcode::FcvtFromSint, &[I8], &[F32]),
    (Opcode::FcvtFromSint, &[I16], &[F32]),
    (Opcode::FcvtFromSint, &[I32], &[F32]),
    (Opcode::FcvtFromSint, &[I64], &[F32]),
    (Opcode::FcvtFromSint, &[I128], &[F32]),
    (Opcode::FcvtFromSint, &[I8], &[F64]),
    (Opcode::FcvtFromSint, &[I16], &[F64]),
    (Opcode::FcvtFromSint, &[I32], &[F64]),
    (Opcode::FcvtFromSint, &[I64], &[F64]),
    (Opcode::FcvtFromSint, &[I128], &[F64]),
    (Opcode::FcvtFromSint, &[I32X4], &[F32X4]),
    (Opcode::FcvtFromSint, &[I64X2], &[F64X2]),
    // FcvtLowFromSint
    (Opcode::FcvtLowFromSint, &[I32X4], &[F64X2]),
    // FvpromoteLow
    (Opcode::FvpromoteLow, &[F32X4], &[F64X2]),
    // Fvdemote
    (Opcode::Fvdemote, &[F64X2], &[F32X4]),
    // Fcmp
    (Opcode::Fcmp, &[F32, F32], &[I8]),
    (Opcode::Fcmp, &[F64, F64], &[I8]),
    (Opcode::Fcmp, &[F32X4, F32X4], &[I32X4]),
    (Opcode::Fcmp, &[F64X2, F64X2], &[I64X2]),
    // Icmp
    (Opcode::Icmp, &[I8, I8], &[I8]),
    (Opcode::Icmp, &[I16, I16], &[I8]),
    (Opcode::Icmp, &[I32, I32], &[I8]),
    (Opcode::Icmp, &[I64, I64], &[I8]),
    (Opcode::Icmp, &[I128, I128], &[I8]),
    (Opcode::Icmp, &[I8X16, I8X16], &[I8X16]),
    (Opcode::Icmp, &[I16X8, I16X8], &[I16X8]),
    (Opcode::Icmp, &[I32X4, I32X4], &[I32X4]),
    (Opcode::Icmp, &[I64X2, I64X2], &[I64X2]),
    // Fence
    (Opcode::Fence, &[], &[]),
    // Stack Access
    (Opcode::StackStore, &[I8], &[]),
    (Opcode::StackStore, &[I16], &[]),
    (Opcode::StackStore, &[I32], &[]),
    (Opcode::StackStore, &[I64], &[]),
    (Opcode::StackStore, &[I128], &[]),
    (Opcode::StackStore, &[F32], &[]),
    (Opcode::StackStore, &[F64], &[]),
    (Opcode::StackStore, &[I8X16], &[]),
    (Opcode::StackStore, &[I16X8], &[]),
    (Opcode::StackStore, &[I32X4], &[]),
    (Opcode::StackStore, &[I64X2], &[]),
    (Opcode::StackStore, &[F32X4], &[]),
    (Opcode::StackStore, &[F64X2], &[]),
    (Opcode::StackLoad, &[], &[I8]),
    (Opcode::StackLoad, &[], &[I16]),
    (Opcode::StackLoad, &[], &[I32]),
    (Opcode::StackLoad, &[], &[I64]),
    (Opcode::StackLoad, &[], &[I128]),
    (Opcode::StackLoad, &[], &[F32]),
    (Opcode::StackLoad, &[], &[F64]),
    (Opcode::StackLoad, &[], &[I8X16]),
    (Opcode::StackLoad, &[], &[I16X8]),
    (Opcode::StackLoad, &[], &[I32X4]),
    (Opcode::StackLoad, &[], &[I64X2]),
    (Opcode::StackLoad, &[], &[F32X4]),
    (Opcode::StackLoad, &[], &[F64X2]),
    // Loads
    (Opcode::Load, &[], &[I8]),
    (Opcode::Load, &[], &[I16]),
    (Opcode::Load, &[], &[I32]),
    (Opcode::Load, &[], &[I64]),
    (Opcode::Load, &[], &[I128]),
    (Opcode::Load, &[], &[F32]),
    (Opcode::Load, &[], &[F64]),
    (Opcode::Load, &[], &[I8X16]),
    (Opcode::Load, &[], &[I16X8]),
    (Opcode::Load, &[], &[I32X4]),
    (Opcode::Load, &[], &[I64X2]),
    (Opcode::Load, &[], &[F32X4]),
    (Opcode::Load, &[], &[F64X2]),
    // Special Loads
    (Opcode::Uload8, &[], &[I16]),
    (Opcode::Uload8, &[], &[I32]),
    (Opcode::Uload8, &[], &[I64]),
    (Opcode::Uload16, &[], &[I32]),
    (Opcode::Uload16, &[], &[I64]),
    (Opcode::Uload32, &[], &[I64]),
    (Opcode::Sload8, &[], &[I16]),
    (Opcode::Sload8, &[], &[I32]),
    (Opcode::Sload8, &[], &[I64]),
    (Opcode::Sload16, &[], &[I32]),
    (Opcode::Sload16, &[], &[I64]),
    (Opcode::Sload32, &[], &[I64]),
    // TODO: Unimplemented in the interpreter
    // Opcode::Uload8x8
    // Opcode::Sload8x8
    // Opcode::Uload16x4
    // Opcode::Sload16x4
    // Opcode::Uload32x2
    // Opcode::Sload32x2
    // AtomicLoad
    (Opcode::AtomicLoad, &[], &[I8]),
    (Opcode::AtomicLoad, &[], &[I16]),
    (Opcode::AtomicLoad, &[], &[I32]),
    (Opcode::AtomicLoad, &[], &[I64]),
    // Stores
    (Opcode::Store, &[I8], &[]),
    (Opcode::Store, &[I16], &[]),
    (Opcode::Store, &[I32], &[]),
    (Opcode::Store, &[I64], &[]),
    (Opcode::Store, &[I128], &[]),
    (Opcode::Store, &[F32], &[]),
    (Opcode::Store, &[F64], &[]),
    (Opcode::Store, &[I8X16], &[]),
    (Opcode::Store, &[I16X8], &[]),
    (Opcode::Store, &[I32X4], &[]),
    (Opcode::Store, &[I64X2], &[]),
    (Opcode::Store, &[F32X4], &[]),
    (Opcode::Store, &[F64X2], &[]),
    // Special Stores
    (Opcode::Istore8, &[I16], &[]),
    (Opcode::Istore8, &[I32], &[]),
    (Opcode::Istore8, &[I64], &[]),
    (Opcode::Istore16, &[I32], &[]),
    (Opcode::Istore16, &[I64], &[]),
    (Opcode::Istore32, &[I64], &[]),
    // AtomicStore
    (Opcode::AtomicStore, &[I8], &[]),
    (Opcode::AtomicStore, &[I16], &[]),
    (Opcode::AtomicStore, &[I32], &[]),
    (Opcode::AtomicStore, &[I64], &[]),
    // AtomicRMW
    (Opcode::AtomicRmw, &[I8, I8], &[I8]),
    (Opcode::AtomicRmw, &[I16, I16], &[I16]),
    (Opcode::AtomicRmw, &[I32, I32], &[I32]),
    (Opcode::AtomicRmw, &[I64, I64], &[I64]),
    // AtomicCas
    (Opcode::AtomicCas, &[I8, I8], &[I8]),
    (Opcode::AtomicCas, &[I16, I16], &[I16]),
    (Opcode::AtomicCas, &[I32, I32], &[I32]),
    (Opcode::AtomicCas, &[I64, I64], &[I64]),
    // Bitcast
    (Opcode::Bitcast, &[F32], &[I32]),
    (Opcode::Bitcast, &[I32], &[F32]),
    (Opcode::Bitcast, &[F64], &[I64]),
    (Opcode::Bitcast, &[I64], &[F64]),
    // Shuffle
    (Opcode::Shuffle, &[I8X16, I8X16, I8X16], &[I8X16]),
    // Swizzle
    (Opcode::Swizzle, &[I8X16, I8X16], &[I8X16]),
    // Splat
    (Opcode::Splat, &[I8], &[I8X16]),
    (Opcode::Splat, &[I16], &[I16X8]),
    (Opcode::Splat, &[I32], &[I32X4]),
    (Opcode::Splat, &[I64], &[I64X2]),
    (Opcode::Splat, &[F32], &[F32X4]),
    (Opcode::Splat, &[F64], &[F64X2]),
    // Insert Lane
    (Opcode::Insertlane, &[I8X16, I8], &[I8X16]),
    (Opcode::Insertlane, &[I16X8, I16], &[I16X8]),
    (Opcode::Insertlane, &[I32X4, I32], &[I32X4]),
    (Opcode::Insertlane, &[I64X2, I64], &[I64X2]),
    (Opcode::Insertlane, &[F32X4, F32], &[F32X4]),
    (Opcode::Insertlane, &[F64X2, F64], &[F64X2]),
    // Extract Lane
    (Opcode::Extractlane, &[I8X16], &[I8]),
    (Opcode::Extractlane, &[I16X8], &[I16]),
    (Opcode::Extractlane, &[I32X4], &[I32]),
    (Opcode::Extractlane, &[I64X2], &[I64]),
    (Opcode::Extractlane, &[F32X4], &[F32]),
    (Opcode::Extractlane, &[F64X2], &[F64]),
    // Snarrow
    (Opcode::Snarrow, &[I64X2, I64X2], &[I32X4]),
    (Opcode::Snarrow, &[I32X4, I32X4], &[I16X8]),
    (Opcode::Snarrow, &[I16X8, I16X8], &[I8X16]),
    // Unarrow
    (Opcode::Unarrow, &[I64X2, I64X2], &[I32X4]),
    (Opcode::Unarrow, &[I32X4, I32X4], &[I16X8]),
    (Opcode::Unarrow, &[I16X8, I16X8], &[I8X16]),
    // Uunarrow
    (Opcode::Uunarrow, &[I64X2, I64X2], &[I32X4]),
    (Opcode::Uunarrow, &[I32X4, I32X4], &[I16X8]),
    (Opcode::Uunarrow, &[I16X8, I16X8], &[I8X16]),
    // VhighBits
    (Opcode::VhighBits, &[I8X16], &[I8]),
    (Opcode::VhighBits, &[I16X8], &[I8]),
    (Opcode::VhighBits, &[I32X4], &[I8]),
    (Opcode::VhighBits, &[I64X2], &[I8]),
    // VanyTrue
    (Opcode::VanyTrue, &[I8X16, I8X16, I8X16], &[I8]),
    (Opcode::VanyTrue, &[I16X8, I16X8, I16X8], &[I8]),
    (Opcode::VanyTrue, &[I32X4, I32X4, I32X4], &[I8]),
    (Opcode::VanyTrue, &[I64X2, I64X2, I64X2], &[I8]),
    // SwidenLow
    (Opcode::SwidenLow, &[I8X16], &[I16X8]),
    (Opcode::SwidenLow, &[I16X8], &[I32X4]),
    (Opcode::SwidenLow, &[I32X4], &[I64X2]),
    // SwidenHigh
    (Opcode::SwidenHigh, &[I8X16], &[I16X8]),
    (Opcode::SwidenHigh, &[I16X8], &[I32X4]),
    (Opcode::SwidenHigh, &[I32X4], &[I64X2]),
    // UwidenLow
    (Opcode::UwidenLow, &[I8X16], &[I16X8]),
    (Opcode::UwidenLow, &[I16X8], &[I32X4]),
    (Opcode::UwidenLow, &[I32X4], &[I64X2]),
    // UwidenHigh
    (Opcode::UwidenHigh, &[I8X16], &[I16X8]),
    (Opcode::UwidenHigh, &[I16X8], &[I32X4]),
    (Opcode::UwidenHigh, &[I32X4], &[I64X2]),
    // SqmulRoundSat
    (Opcode::SqmulRoundSat, &[I16X8, I16X8], &[I16X8]),
    (Opcode::SqmulRoundSat, &[I32X4, I32X4], &[I32X4]),
    // IaddPairwise
    (Opcode::IaddPairwise, &[I8X16, I8X16], &[I8X16]),
    (Opcode::IaddPairwise, &[I16X8, I16X8], &[I16X8]),
    (Opcode::IaddPairwise, &[I32X4, I32X4], &[I32X4]),
    // Integer Consts
    (Opcode::Iconst, &[], &[I8]),
    (Opcode::Iconst, &[], &[I16]),
    (Opcode::Iconst, &[], &[I32]),
    (Opcode::Iconst, &[], &[I64]),
    // Float Consts
    (Opcode::F32const, &[], &[F32]),
    (Opcode::F64const, &[], &[F64]),
    // Vector Consts
    (Opcode::Vconst, &[], &[I8X16]),
    (Opcode::Vconst, &[], &[I16X8]),
    (Opcode::Vconst, &[], &[I32X4]),
    (Opcode::Vconst, &[], &[I64X2]),
    (Opcode::Vconst, &[], &[F32X4]),
    (Opcode::Vconst, &[], &[F64X2]),
    // Call
    (Opcode::Call, &[], &[]),
];

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
        InstructionFormat::TableAddr => todo!(),
        InstructionFormat::Ternary => insert_opcode,
        InstructionFormat::TernaryImm8 => insert_ins_ext_lane,
        InstructionFormat::Trap => todo!(),
        InstructionFormat::Unary => insert_opcode,
        InstructionFormat::UnaryConst => insert_const,
        InstructionFormat::UnaryGlobalValue => todo!(),
        InstructionFormat::UnaryIeee32 => insert_const,
        InstructionFormat::UnaryIeee64 => insert_const,
        InstructionFormat::UnaryImm => insert_const,

        InstructionFormat::BranchTable
        | InstructionFormat::Brif
        | InstructionFormat::Jump
        | InstructionFormat::MultiAry => {
            panic!("Control-flow instructions should be handled by 'insert_terminator'")
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
    target_triple: Triple,
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
}

#[derive(Debug, Clone)]
enum BlockTerminatorKind {
    Return,
    Jump,
    Br,
    BrTable,
    Switch,
}

#[derive(Default)]
struct Resources {
    vars: HashMap<Type, Vec<Variable>>,
    blocks: Vec<(Block, BlockSignature)>,
    blocks_without_params: Vec<Block>,
    block_terminators: Vec<BlockTerminator>,
    func_refs: Vec<(Signature, FuncRef)>,
    stack_slots: Vec<(StackSlot, StackSize)>,
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
}

impl<'r, 'data> FunctionGenerator<'r, 'data>
where
    'data: 'r,
{
    pub fn new(
        u: &'r mut Unstructured<'data>,
        config: &'r Config,
        target_triple: Triple,
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
            target_triple,
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
    fn stack_slot_with_size(&mut self, n: u32) -> Result<(StackSlot, StackSize)> {
        let first = self
            .resources
            .stack_slots
            .partition_point(|&(_slot, size)| size < n);
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
    ) -> Result<(Value, u32)> {
        // TODO: Currently our only source of addresses is stack_addr, but we
        // should add global_value, symbol_value eventually
        let (addr, available_size) = {
            let (ss, slot_size) = self.stack_slot_with_size(min_size)?;

            // stack_slot_with_size guarantees that slot_size >= min_size
            let max_offset = slot_size - min_size;
            let offset = if aligned {
                self.u.int_in_range(0..=max_offset / min_size)? * min_size
            } else {
                self.u.int_in_range(0..=max_offset)?
            };

            let base_addr = builder.ins().stack_addr(I64, ss, offset as i32);
            let available_size = slot_size.saturating_sub(offset);
            (base_addr, available_size)
        };

        // TODO: Insert a bunch of amode opcodes here to modify the address!

        // Now that we have an address and a size, we just choose a random offset to return to the
        // caller. Preserving min_size bytes.
        let max_offset = available_size.saturating_sub(min_size);
        Ok((addr, max_offset))
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
            self.target_triple.architecture,
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

        let (address, max_offset) = self.generate_load_store_address(builder, min_size, aligned)?;

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
            DataValue::I8(i) => builder.ins().iconst(ty, i as i64),
            DataValue::I16(i) => builder.ins().iconst(ty, i as i64),
            DataValue::I32(i) => builder.ins().iconst(ty, i as i64),
            DataValue::I64(i) => builder.ins().iconst(ty, i as i64),
            DataValue::I128(i) => {
                let hi = builder.ins().iconst(I64, (i >> 64) as i64);
                let lo = builder.ins().iconst(I64, i as i64);
                builder.ins().iconcat(lo, hi)
            }
            DataValue::F32(f) => builder.ins().f32const(f),
            DataValue::F64(f) => builder.ins().f64const(f),
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
        }

        Ok(())
    }

    /// Fills the current block with random instructions
    fn generate_instructions(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        for _ in 0..self.param(&self.config.instructions_per_block)? {
            let (op, args, rets) = *self.u.choose(OPCODE_SIGNATURES)?;

            // We filter out instructions that aren't supported by the target at this point instead
            // of building a single vector of valid instructions at the beginning of function
            // generation, to avoid invalidating the corpus when instructions are enabled/disabled.
            if !valid_for_target(&self.target_triple, op, args, rets) {
                return Err(arbitrary::Error::IncorrectFormat.into());
            }

            let inserter = inserter_for_format(op.format());
            inserter(self, builder, op, args, rets)?;
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
                let signature = libcall.signature(lib_callconv);
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

            self.resources.func_refs.push((signature, func_ref));
        }

        Ok(())
    }

    fn generate_stack_slots(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        for _ in 0..self.param(&self.config.static_stack_slots_per_function)? {
            let bytes = self.param(&self.config.static_stack_slot_size)? as u32;
            let ss_data = StackSlotData::new(StackSlotKind::ExplicitSlot, bytes);
            let slot = builder.create_sized_stack_slot(ss_data);
            self.resources.stack_slots.push((slot, bytes));
        }

        self.resources
            .stack_slots
            .sort_unstable_by_key(|&(_slot, bytes)| bytes);

        Ok(())
    }

    /// Zero initializes the stack slot by inserting `stack_store`'s.
    fn initialize_stack_slots(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let i8_zero = builder.ins().iconst(I8, 0);
        let i16_zero = builder.ins().iconst(I16, 0);
        let i32_zero = builder.ins().iconst(I32, 0);
        let i64_zero = builder.ins().iconst(I64, 0);
        let i128_zero = builder.ins().uextend(I128, i64_zero);

        for &(slot, init_size) in self.resources.stack_slots.iter() {
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
                builder.ins().stack_store(val, slot, offset);
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
                })
            })
            .collect::<Result<_>>()?;

        Ok(())
    }

    fn generate_block_signature(&mut self) -> Result<BlockSignature> {
        let param_count = self.param(&self.config.block_signature_params)?;

        let mut params = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            params.push(self.u._type(self.target_triple.architecture)?);
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
            let ty = self.u._type(self.target_triple.architecture)?;
            let value = self.generate_const(builder, ty)?;
            vars.push((ty, value));
        }

        for (id, (ty, value)) in vars.into_iter().enumerate() {
            let var = Variable::new(id);
            builder.declare_var(var, ty);
            builder.def_var(var, value);
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

        self.generate_blocks(&mut builder)?;

        // Function preamble
        self.generate_funcrefs(&mut builder)?;
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
