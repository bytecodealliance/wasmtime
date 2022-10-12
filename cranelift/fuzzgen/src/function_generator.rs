use crate::codegen::ir::{ArgumentExtension, ArgumentPurpose};
use crate::config::Config;
use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use cranelift::codegen::ir::immediates::Offset32;
use cranelift::codegen::ir::instructions::InstructionFormat;
use cranelift::codegen::ir::stackslot::StackSize;
use cranelift::codegen::ir::{types::*, FuncRef, LibCall, UserExternalName, UserFuncName};
use cranelift::codegen::ir::{
    AbiParam, Block, ExternalName, Function, Opcode, Signature, StackSlot, Type, Value,
};
use cranelift::codegen::isa::CallConv;
use cranelift::frontend::{FunctionBuilder, FunctionBuilderContext, Switch, Variable};
use cranelift::prelude::{
    EntityRef, ExtFuncData, FloatCC, InstBuilder, IntCC, JumpTableData, MemFlags, StackSlotData,
    StackSlotKind,
};
use std::collections::HashMap;
use std::ops::RangeInclusive;

/// Generates a Vec with `len` elements comprised of `options`
fn arbitrary_vec<T: Clone>(
    u: &mut Unstructured,
    len: usize,
    options: &[T],
) -> arbitrary::Result<Vec<T>> {
    (0..len).map(|_| u.choose(options).cloned()).collect()
}

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

    // For pretty much every instruction the control type is the return type
    // except for Iconcat and Isplit which are *special* and the control type
    // is the input type.
    let ctrl_type = if opcode == Opcode::Iconcat || opcode == Opcode::Isplit {
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
        // Some FloatCC's are not implemented on AArch64, see:
        // https://github.com/bytecodealliance/wasmtime/issues/4850
        let float_cc = if cfg!(target_arch = "aarch64") {
            &[
                FloatCC::Ordered,
                FloatCC::Unordered,
                FloatCC::Equal,
                FloatCC::NotEqual,
                FloatCC::LessThan,
                FloatCC::LessThanOrEqual,
                FloatCC::GreaterThan,
                FloatCC::GreaterThanOrEqual,
            ]
        } else {
            FloatCC::all()
        };

        let cc = *fgen.u.choose(float_cc)?;
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

fn insert_load_store(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    args: &'static [Type],
    rets: &'static [Type],
) -> Result<()> {
    let ctrl_type = *rets.first().or(args.first()).unwrap();
    let type_size = ctrl_type.bytes();
    let (address, offset) = fgen.generate_load_store_address(builder, type_size)?;

    // TODO: More advanced MemFlags
    let flags = MemFlags::new();

    // The variable being loaded or stored into
    let var = fgen.get_variable_of_type(ctrl_type)?;

    if opcode.can_store() {
        let val = builder.use_var(var);

        builder
            .ins()
            .Store(opcode, ctrl_type, flags, offset, val, address);
    } else {
        let (inst, dfg) = builder
            .ins()
            .Load(opcode, ctrl_type, flags, offset, address);

        let new_val = dfg.first_result(inst);
        builder.def_var(var, new_val);
    }

    Ok(())
}

type OpcodeInserter = fn(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    Opcode,
    &'static [Type],
    &'static [Type],
) -> Result<()>;

// TODO: Derive this from the `cranelift-meta` generator.
const OPCODE_SIGNATURES: &'static [(
    Opcode,
    &'static [Type], // Args
    &'static [Type], // Rets
    OpcodeInserter,
)] = &[
    (Opcode::Nop, &[], &[], insert_opcode),
    // Iadd
    (Opcode::Iadd, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Iadd, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Iadd, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Iadd, &[I64, I64], &[I64], insert_opcode),
    (Opcode::Iadd, &[I128, I128], &[I128], insert_opcode),
    // Isub
    (Opcode::Isub, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Isub, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Isub, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Isub, &[I64, I64], &[I64], insert_opcode),
    (Opcode::Isub, &[I128, I128], &[I128], insert_opcode),
    // Imul
    (Opcode::Imul, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Imul, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Imul, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Imul, &[I64, I64], &[I64], insert_opcode),
    (Opcode::Imul, &[I128, I128], &[I128], insert_opcode),
    // Udiv
    (Opcode::Udiv, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Udiv, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Udiv, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Udiv, &[I64, I64], &[I64], insert_opcode),
    // udiv.i128 not implemented in some backends:
    //   x64: https://github.com/bytecodealliance/wasmtime/issues/4756
    //   aarch64: https://github.com/bytecodealliance/wasmtime/issues/4864
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::Udiv, &[I128, I128], &[I128], insert_opcode),
    // Sdiv
    (Opcode::Sdiv, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Sdiv, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Sdiv, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Sdiv, &[I64, I64], &[I64], insert_opcode),
    // sdiv.i128 not implemented in some backends:
    //   x64: https://github.com/bytecodealliance/wasmtime/issues/4770
    //   aarch64: https://github.com/bytecodealliance/wasmtime/issues/4864
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::Sdiv, &[I128, I128], &[I128], insert_opcode),
    // Rotr
    (Opcode::Rotr, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Rotr, &[I8, I16], &[I8], insert_opcode),
    (Opcode::Rotr, &[I8, I32], &[I8], insert_opcode),
    (Opcode::Rotr, &[I8, I64], &[I8], insert_opcode),
    (Opcode::Rotr, &[I8, I128], &[I8], insert_opcode),
    (Opcode::Rotr, &[I16, I8], &[I16], insert_opcode),
    (Opcode::Rotr, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Rotr, &[I16, I32], &[I16], insert_opcode),
    (Opcode::Rotr, &[I16, I64], &[I16], insert_opcode),
    (Opcode::Rotr, &[I16, I128], &[I16], insert_opcode),
    (Opcode::Rotr, &[I32, I8], &[I32], insert_opcode),
    (Opcode::Rotr, &[I32, I16], &[I32], insert_opcode),
    (Opcode::Rotr, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Rotr, &[I32, I64], &[I32], insert_opcode),
    (Opcode::Rotr, &[I32, I128], &[I32], insert_opcode),
    (Opcode::Rotr, &[I64, I8], &[I64], insert_opcode),
    (Opcode::Rotr, &[I64, I16], &[I64], insert_opcode),
    (Opcode::Rotr, &[I64, I32], &[I64], insert_opcode),
    (Opcode::Rotr, &[I64, I64], &[I64], insert_opcode),
    (Opcode::Rotr, &[I64, I128], &[I64], insert_opcode),
    (Opcode::Rotr, &[I128, I8], &[I128], insert_opcode),
    (Opcode::Rotr, &[I128, I16], &[I128], insert_opcode),
    (Opcode::Rotr, &[I128, I32], &[I128], insert_opcode),
    (Opcode::Rotr, &[I128, I64], &[I128], insert_opcode),
    (Opcode::Rotr, &[I128, I128], &[I128], insert_opcode),
    // Rotl
    (Opcode::Rotl, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Rotl, &[I8, I16], &[I8], insert_opcode),
    (Opcode::Rotl, &[I8, I32], &[I8], insert_opcode),
    (Opcode::Rotl, &[I8, I64], &[I8], insert_opcode),
    (Opcode::Rotl, &[I8, I128], &[I8], insert_opcode),
    (Opcode::Rotl, &[I16, I8], &[I16], insert_opcode),
    (Opcode::Rotl, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Rotl, &[I16, I32], &[I16], insert_opcode),
    (Opcode::Rotl, &[I16, I64], &[I16], insert_opcode),
    (Opcode::Rotl, &[I16, I128], &[I16], insert_opcode),
    (Opcode::Rotl, &[I32, I8], &[I32], insert_opcode),
    (Opcode::Rotl, &[I32, I16], &[I32], insert_opcode),
    (Opcode::Rotl, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Rotl, &[I32, I64], &[I32], insert_opcode),
    (Opcode::Rotl, &[I32, I128], &[I32], insert_opcode),
    (Opcode::Rotl, &[I64, I8], &[I64], insert_opcode),
    (Opcode::Rotl, &[I64, I16], &[I64], insert_opcode),
    (Opcode::Rotl, &[I64, I32], &[I64], insert_opcode),
    (Opcode::Rotl, &[I64, I64], &[I64], insert_opcode),
    (Opcode::Rotl, &[I64, I128], &[I64], insert_opcode),
    (Opcode::Rotl, &[I128, I8], &[I128], insert_opcode),
    (Opcode::Rotl, &[I128, I16], &[I128], insert_opcode),
    (Opcode::Rotl, &[I128, I32], &[I128], insert_opcode),
    (Opcode::Rotl, &[I128, I64], &[I128], insert_opcode),
    (Opcode::Rotl, &[I128, I128], &[I128], insert_opcode),
    // Ishl
    (Opcode::Ishl, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Ishl, &[I8, I16], &[I8], insert_opcode),
    (Opcode::Ishl, &[I8, I32], &[I8], insert_opcode),
    (Opcode::Ishl, &[I8, I64], &[I8], insert_opcode),
    (Opcode::Ishl, &[I8, I128], &[I8], insert_opcode),
    (Opcode::Ishl, &[I16, I8], &[I16], insert_opcode),
    (Opcode::Ishl, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Ishl, &[I16, I32], &[I16], insert_opcode),
    (Opcode::Ishl, &[I16, I64], &[I16], insert_opcode),
    (Opcode::Ishl, &[I16, I128], &[I16], insert_opcode),
    (Opcode::Ishl, &[I32, I8], &[I32], insert_opcode),
    (Opcode::Ishl, &[I32, I16], &[I32], insert_opcode),
    (Opcode::Ishl, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Ishl, &[I32, I64], &[I32], insert_opcode),
    (Opcode::Ishl, &[I32, I128], &[I32], insert_opcode),
    (Opcode::Ishl, &[I64, I8], &[I64], insert_opcode),
    (Opcode::Ishl, &[I64, I16], &[I64], insert_opcode),
    (Opcode::Ishl, &[I64, I32], &[I64], insert_opcode),
    (Opcode::Ishl, &[I64, I64], &[I64], insert_opcode),
    (Opcode::Ishl, &[I64, I128], &[I64], insert_opcode),
    (Opcode::Ishl, &[I128, I8], &[I128], insert_opcode),
    (Opcode::Ishl, &[I128, I16], &[I128], insert_opcode),
    (Opcode::Ishl, &[I128, I32], &[I128], insert_opcode),
    (Opcode::Ishl, &[I128, I64], &[I128], insert_opcode),
    (Opcode::Ishl, &[I128, I128], &[I128], insert_opcode),
    // Sshr
    (Opcode::Sshr, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Sshr, &[I8, I16], &[I8], insert_opcode),
    (Opcode::Sshr, &[I8, I32], &[I8], insert_opcode),
    (Opcode::Sshr, &[I8, I64], &[I8], insert_opcode),
    (Opcode::Sshr, &[I8, I128], &[I8], insert_opcode),
    (Opcode::Sshr, &[I16, I8], &[I16], insert_opcode),
    (Opcode::Sshr, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Sshr, &[I16, I32], &[I16], insert_opcode),
    (Opcode::Sshr, &[I16, I64], &[I16], insert_opcode),
    (Opcode::Sshr, &[I16, I128], &[I16], insert_opcode),
    (Opcode::Sshr, &[I32, I8], &[I32], insert_opcode),
    (Opcode::Sshr, &[I32, I16], &[I32], insert_opcode),
    (Opcode::Sshr, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Sshr, &[I32, I64], &[I32], insert_opcode),
    (Opcode::Sshr, &[I32, I128], &[I32], insert_opcode),
    (Opcode::Sshr, &[I64, I8], &[I64], insert_opcode),
    (Opcode::Sshr, &[I64, I16], &[I64], insert_opcode),
    (Opcode::Sshr, &[I64, I32], &[I64], insert_opcode),
    (Opcode::Sshr, &[I64, I64], &[I64], insert_opcode),
    (Opcode::Sshr, &[I64, I128], &[I64], insert_opcode),
    (Opcode::Sshr, &[I128, I8], &[I128], insert_opcode),
    (Opcode::Sshr, &[I128, I16], &[I128], insert_opcode),
    (Opcode::Sshr, &[I128, I32], &[I128], insert_opcode),
    (Opcode::Sshr, &[I128, I64], &[I128], insert_opcode),
    (Opcode::Sshr, &[I128, I128], &[I128], insert_opcode),
    // Ushr
    (Opcode::Ushr, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Ushr, &[I8, I16], &[I8], insert_opcode),
    (Opcode::Ushr, &[I8, I32], &[I8], insert_opcode),
    (Opcode::Ushr, &[I8, I64], &[I8], insert_opcode),
    (Opcode::Ushr, &[I8, I128], &[I8], insert_opcode),
    (Opcode::Ushr, &[I16, I8], &[I16], insert_opcode),
    (Opcode::Ushr, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Ushr, &[I16, I32], &[I16], insert_opcode),
    (Opcode::Ushr, &[I16, I64], &[I16], insert_opcode),
    (Opcode::Ushr, &[I16, I128], &[I16], insert_opcode),
    (Opcode::Ushr, &[I32, I8], &[I32], insert_opcode),
    (Opcode::Ushr, &[I32, I16], &[I32], insert_opcode),
    (Opcode::Ushr, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Ushr, &[I32, I64], &[I32], insert_opcode),
    (Opcode::Ushr, &[I32, I128], &[I32], insert_opcode),
    (Opcode::Ushr, &[I64, I8], &[I64], insert_opcode),
    (Opcode::Ushr, &[I64, I16], &[I64], insert_opcode),
    (Opcode::Ushr, &[I64, I32], &[I64], insert_opcode),
    (Opcode::Ushr, &[I64, I64], &[I64], insert_opcode),
    (Opcode::Ushr, &[I64, I128], &[I64], insert_opcode),
    (Opcode::Ushr, &[I128, I8], &[I128], insert_opcode),
    (Opcode::Ushr, &[I128, I16], &[I128], insert_opcode),
    (Opcode::Ushr, &[I128, I32], &[I128], insert_opcode),
    (Opcode::Ushr, &[I128, I64], &[I128], insert_opcode),
    (Opcode::Ushr, &[I128, I128], &[I128], insert_opcode),
    // Uextend
    (Opcode::Uextend, &[I8], &[I16], insert_opcode),
    (Opcode::Uextend, &[I8], &[I32], insert_opcode),
    (Opcode::Uextend, &[I8], &[I64], insert_opcode),
    (Opcode::Uextend, &[I8], &[I128], insert_opcode),
    (Opcode::Uextend, &[I16], &[I32], insert_opcode),
    (Opcode::Uextend, &[I16], &[I64], insert_opcode),
    (Opcode::Uextend, &[I16], &[I128], insert_opcode),
    (Opcode::Uextend, &[I32], &[I64], insert_opcode),
    (Opcode::Uextend, &[I32], &[I128], insert_opcode),
    (Opcode::Uextend, &[I64], &[I128], insert_opcode),
    // Sextend
    (Opcode::Sextend, &[I8], &[I16], insert_opcode),
    (Opcode::Sextend, &[I8], &[I32], insert_opcode),
    (Opcode::Sextend, &[I8], &[I64], insert_opcode),
    (Opcode::Sextend, &[I8], &[I128], insert_opcode),
    (Opcode::Sextend, &[I16], &[I32], insert_opcode),
    (Opcode::Sextend, &[I16], &[I64], insert_opcode),
    (Opcode::Sextend, &[I16], &[I128], insert_opcode),
    (Opcode::Sextend, &[I32], &[I64], insert_opcode),
    (Opcode::Sextend, &[I32], &[I128], insert_opcode),
    (Opcode::Sextend, &[I64], &[I128], insert_opcode),
    // Ireduce
    (Opcode::Ireduce, &[I16], &[I8], insert_opcode),
    (Opcode::Ireduce, &[I32], &[I8], insert_opcode),
    (Opcode::Ireduce, &[I32], &[I16], insert_opcode),
    (Opcode::Ireduce, &[I64], &[I8], insert_opcode),
    (Opcode::Ireduce, &[I64], &[I16], insert_opcode),
    (Opcode::Ireduce, &[I64], &[I32], insert_opcode),
    (Opcode::Ireduce, &[I128], &[I8], insert_opcode),
    (Opcode::Ireduce, &[I128], &[I16], insert_opcode),
    (Opcode::Ireduce, &[I128], &[I32], insert_opcode),
    (Opcode::Ireduce, &[I128], &[I64], insert_opcode),
    // Isplit
    (Opcode::Isplit, &[I128], &[I64, I64], insert_opcode),
    // Iconcat
    (Opcode::Iconcat, &[I64, I64], &[I128], insert_opcode),
    // Band
    (Opcode::Band, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Band, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Band, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Band, &[I64, I64], &[I64], insert_opcode),
    (Opcode::Band, &[I128, I128], &[I128], insert_opcode),
    // Float bitops are currently not supported:
    // See: https://github.com/bytecodealliance/wasmtime/issues/4870
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::Band, &[F32, F32], &[F32], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::Band, &[F64, F64], &[F64], insert_opcode),
    // Bor
    (Opcode::Bor, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Bor, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Bor, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Bor, &[I64, I64], &[I64], insert_opcode),
    (Opcode::Bor, &[I128, I128], &[I128], insert_opcode),
    // Float bitops are currently not supported:
    // See: https://github.com/bytecodealliance/wasmtime/issues/4870
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::Bor, &[F32, F32], &[F32], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::Bor, &[F64, F64], &[F64], insert_opcode),
    // Bxor
    (Opcode::Bxor, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Bxor, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Bxor, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Bxor, &[I64, I64], &[I64], insert_opcode),
    (Opcode::Bxor, &[I128, I128], &[I128], insert_opcode),
    // Float bitops are currently not supported:
    // See: https://github.com/bytecodealliance/wasmtime/issues/4870
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::Bxor, &[F32, F32], &[F32], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::Bxor, &[F64, F64], &[F64], insert_opcode),
    // Bnot
    (Opcode::Bnot, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Bnot, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Bnot, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Bnot, &[I64, I64], &[I64], insert_opcode),
    (Opcode::Bnot, &[I128, I128], &[I128], insert_opcode),
    // Float bitops are currently not supported:
    // See: https://github.com/bytecodealliance/wasmtime/issues/4870
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::Bnot, &[F32, F32], &[F32], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::Bnot, &[F64, F64], &[F64], insert_opcode),
    // BandNot
    // Some Integer ops not supported on x86: https://github.com/bytecodealliance/wasmtime/issues/5041
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BandNot, &[I8, I8], &[I8], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BandNot, &[I16, I16], &[I16], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BandNot, &[I32, I32], &[I32], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BandNot, &[I64, I64], &[I64], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BandNot, &[I128, I128], &[I128], insert_opcode),
    // Float bitops are currently not supported:
    // See: https://github.com/bytecodealliance/wasmtime/issues/4870
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::BandNot, &[F32, F32], &[F32], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::BandNot, &[F64, F64], &[F64], insert_opcode),
    // BorNot
    // Some Integer ops not supported on x86: https://github.com/bytecodealliance/wasmtime/issues/5041
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BorNot, &[I8, I8], &[I8], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BorNot, &[I16, I16], &[I16], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BorNot, &[I32, I32], &[I32], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BorNot, &[I64, I64], &[I64], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BorNot, &[I128, I128], &[I128], insert_opcode),
    // Float bitops are currently not supported:
    // See: https://github.com/bytecodealliance/wasmtime/issues/4870
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::BorNot, &[F32, F32], &[F32], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::BorNot, &[F64, F64], &[F64], insert_opcode),
    // BxorNot
    // Some Integer ops not supported on x86: https://github.com/bytecodealliance/wasmtime/issues/5041
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BxorNot, &[I8, I8], &[I8], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BxorNot, &[I16, I16], &[I16], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BxorNot, &[I32, I32], &[I32], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BxorNot, &[I64, I64], &[I64], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::BxorNot, &[I128, I128], &[I128], insert_opcode),
    // Float bitops are currently not supported:
    // See: https://github.com/bytecodealliance/wasmtime/issues/4870
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::BxorNot, &[F32, F32], &[F32], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::BxorNot, &[F64, F64], &[F64], insert_opcode),
    // Fadd
    (Opcode::Fadd, &[F32, F32], &[F32], insert_opcode),
    (Opcode::Fadd, &[F64, F64], &[F64], insert_opcode),
    // Fmul
    (Opcode::Fmul, &[F32, F32], &[F32], insert_opcode),
    (Opcode::Fmul, &[F64, F64], &[F64], insert_opcode),
    // Fsub
    (Opcode::Fsub, &[F32, F32], &[F32], insert_opcode),
    (Opcode::Fsub, &[F64, F64], &[F64], insert_opcode),
    // Fdiv
    (Opcode::Fdiv, &[F32, F32], &[F32], insert_opcode),
    (Opcode::Fdiv, &[F64, F64], &[F64], insert_opcode),
    // Fmin
    (Opcode::Fmin, &[F32, F32], &[F32], insert_opcode),
    (Opcode::Fmin, &[F64, F64], &[F64], insert_opcode),
    // Fmax
    (Opcode::Fmax, &[F32, F32], &[F32], insert_opcode),
    (Opcode::Fmax, &[F64, F64], &[F64], insert_opcode),
    // FminPseudo
    (Opcode::FminPseudo, &[F32, F32], &[F32], insert_opcode),
    (Opcode::FminPseudo, &[F64, F64], &[F64], insert_opcode),
    // FmaxPseudo
    (Opcode::FmaxPseudo, &[F32, F32], &[F32], insert_opcode),
    (Opcode::FmaxPseudo, &[F64, F64], &[F64], insert_opcode),
    // Fcopysign
    (Opcode::Fcopysign, &[F32, F32], &[F32], insert_opcode),
    (Opcode::Fcopysign, &[F64, F64], &[F64], insert_opcode),
    // Fma
    (Opcode::Fma, &[F32, F32, F32], &[F32], insert_opcode),
    (Opcode::Fma, &[F64, F64, F64], &[F64], insert_opcode),
    // Fabs
    (Opcode::Fabs, &[F32], &[F32], insert_opcode),
    (Opcode::Fabs, &[F64], &[F64], insert_opcode),
    // Fneg
    (Opcode::Fneg, &[F32], &[F32], insert_opcode),
    (Opcode::Fneg, &[F64], &[F64], insert_opcode),
    // Sqrt
    (Opcode::Sqrt, &[F32], &[F32], insert_opcode),
    (Opcode::Sqrt, &[F64], &[F64], insert_opcode),
    // Ceil
    (Opcode::Ceil, &[F32], &[F32], insert_opcode),
    (Opcode::Ceil, &[F64], &[F64], insert_opcode),
    // Floor
    (Opcode::Floor, &[F32], &[F32], insert_opcode),
    (Opcode::Floor, &[F64], &[F64], insert_opcode),
    // Trunc
    (Opcode::Trunc, &[F32], &[F32], insert_opcode),
    (Opcode::Trunc, &[F64], &[F64], insert_opcode),
    // Nearest
    (Opcode::Nearest, &[F32], &[F32], insert_opcode),
    (Opcode::Nearest, &[F64], &[F64], insert_opcode),
    // FcvtToUint
    // TODO: Some ops disabled:
    //   x64: https://github.com/bytecodealliance/wasmtime/issues/4897
    //   x64: https://github.com/bytecodealliance/wasmtime/issues/4899
    //   aarch64: https://github.com/bytecodealliance/wasmtime/issues/4934
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToUint, &[F32], &[I8], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToUint, &[F32], &[I16], insert_opcode),
    (Opcode::FcvtToUint, &[F32], &[I32], insert_opcode),
    (Opcode::FcvtToUint, &[F32], &[I64], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::FcvtToUint, &[F32], &[I128], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToUint, &[F64], &[I8], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToUint, &[F64], &[I16], insert_opcode),
    (Opcode::FcvtToUint, &[F64], &[I32], insert_opcode),
    (Opcode::FcvtToUint, &[F64], &[I64], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::FcvtToUint, &[F64], &[I128], insert_opcode),
    // FcvtToUintSat
    // TODO: Some ops disabled:
    //   x64: https://github.com/bytecodealliance/wasmtime/issues/4897
    //   x64: https://github.com/bytecodealliance/wasmtime/issues/4899
    //   aarch64: https://github.com/bytecodealliance/wasmtime/issues/4934
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToUintSat, &[F32], &[I8], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToUintSat, &[F32], &[I16], insert_opcode),
    (Opcode::FcvtToUintSat, &[F32], &[I32], insert_opcode),
    (Opcode::FcvtToUintSat, &[F32], &[I64], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::FcvtToUintSat, &[F32], &[I128], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToUintSat, &[F64], &[I8], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToUintSat, &[F64], &[I16], insert_opcode),
    (Opcode::FcvtToUintSat, &[F64], &[I32], insert_opcode),
    (Opcode::FcvtToUintSat, &[F64], &[I64], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::FcvtToUintSat, &[F64], &[I128], insert_opcode),
    // FcvtToSint
    // TODO: Some ops disabled:
    //   x64: https://github.com/bytecodealliance/wasmtime/issues/4897
    //   x64: https://github.com/bytecodealliance/wasmtime/issues/4899
    //   aarch64: https://github.com/bytecodealliance/wasmtime/issues/4934
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToSint, &[F32], &[I8], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToSint, &[F32], &[I16], insert_opcode),
    (Opcode::FcvtToSint, &[F32], &[I32], insert_opcode),
    (Opcode::FcvtToSint, &[F32], &[I64], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::FcvtToSint, &[F32], &[I128], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToSint, &[F64], &[I8], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToSint, &[F64], &[I16], insert_opcode),
    (Opcode::FcvtToSint, &[F64], &[I32], insert_opcode),
    (Opcode::FcvtToSint, &[F64], &[I64], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::FcvtToSint, &[F64], &[I128], insert_opcode),
    // FcvtToSintSat
    // TODO: Some ops disabled:
    //   x64: https://github.com/bytecodealliance/wasmtime/issues/4897
    //   x64: https://github.com/bytecodealliance/wasmtime/issues/4899
    //   aarch64: https://github.com/bytecodealliance/wasmtime/issues/4934
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToSintSat, &[F32], &[I8], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToSintSat, &[F32], &[I16], insert_opcode),
    (Opcode::FcvtToSintSat, &[F32], &[I32], insert_opcode),
    (Opcode::FcvtToSintSat, &[F32], &[I64], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::FcvtToSintSat, &[F32], &[I128], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToSintSat, &[F64], &[I8], insert_opcode),
    #[cfg(not(target_arch = "x86_64"))]
    (Opcode::FcvtToSintSat, &[F64], &[I16], insert_opcode),
    (Opcode::FcvtToSintSat, &[F64], &[I32], insert_opcode),
    (Opcode::FcvtToSintSat, &[F64], &[I64], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::FcvtToSintSat, &[F64], &[I128], insert_opcode),
    // FcvtFromUint
    // TODO: Some ops disabled:
    //   x64: https://github.com/bytecodealliance/wasmtime/issues/4900
    //   aarch64: https://github.com/bytecodealliance/wasmtime/issues/4933
    (Opcode::FcvtFromUint, &[I8], &[F32], insert_opcode),
    (Opcode::FcvtFromUint, &[I16], &[F32], insert_opcode),
    (Opcode::FcvtFromUint, &[I32], &[F32], insert_opcode),
    (Opcode::FcvtFromUint, &[I64], &[F32], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::FcvtFromUint, &[I128], &[F32], insert_opcode),
    (Opcode::FcvtFromUint, &[I8], &[F64], insert_opcode),
    (Opcode::FcvtFromUint, &[I16], &[F64], insert_opcode),
    (Opcode::FcvtFromUint, &[I32], &[F64], insert_opcode),
    (Opcode::FcvtFromUint, &[I64], &[F64], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::FcvtFromUint, &[I128], &[F64], insert_opcode),
    // FcvtFromSint
    // TODO: Some ops disabled:
    //   x64: https://github.com/bytecodealliance/wasmtime/issues/4900
    //   aarch64: https://github.com/bytecodealliance/wasmtime/issues/4933
    (Opcode::FcvtFromSint, &[I8], &[F32], insert_opcode),
    (Opcode::FcvtFromSint, &[I16], &[F32], insert_opcode),
    (Opcode::FcvtFromSint, &[I32], &[F32], insert_opcode),
    (Opcode::FcvtFromSint, &[I64], &[F32], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::FcvtFromSint, &[I128], &[F32], insert_opcode),
    (Opcode::FcvtFromSint, &[I8], &[F64], insert_opcode),
    (Opcode::FcvtFromSint, &[I16], &[F64], insert_opcode),
    (Opcode::FcvtFromSint, &[I32], &[F64], insert_opcode),
    (Opcode::FcvtFromSint, &[I64], &[F64], insert_opcode),
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    (Opcode::FcvtFromSint, &[I128], &[F64], insert_opcode),
    // Fcmp
    (Opcode::Fcmp, &[F32, F32], &[I8], insert_cmp),
    (Opcode::Fcmp, &[F64, F64], &[I8], insert_cmp),
    // Icmp
    (Opcode::Icmp, &[I8, I8], &[I8], insert_cmp),
    (Opcode::Icmp, &[I16, I16], &[I8], insert_cmp),
    (Opcode::Icmp, &[I32, I32], &[I8], insert_cmp),
    (Opcode::Icmp, &[I64, I64], &[I8], insert_cmp),
    (Opcode::Icmp, &[I128, I128], &[I8], insert_cmp),
    // Stack Access
    (Opcode::StackStore, &[I8], &[], insert_stack_store),
    (Opcode::StackStore, &[I16], &[], insert_stack_store),
    (Opcode::StackStore, &[I32], &[], insert_stack_store),
    (Opcode::StackStore, &[I64], &[], insert_stack_store),
    (Opcode::StackStore, &[I128], &[], insert_stack_store),
    (Opcode::StackLoad, &[], &[I8], insert_stack_load),
    (Opcode::StackLoad, &[], &[I16], insert_stack_load),
    (Opcode::StackLoad, &[], &[I32], insert_stack_load),
    (Opcode::StackLoad, &[], &[I64], insert_stack_load),
    (Opcode::StackLoad, &[], &[I128], insert_stack_load),
    // Loads
    (Opcode::Load, &[], &[I8], insert_load_store),
    (Opcode::Load, &[], &[I16], insert_load_store),
    (Opcode::Load, &[], &[I32], insert_load_store),
    (Opcode::Load, &[], &[I64], insert_load_store),
    (Opcode::Load, &[], &[I128], insert_load_store),
    (Opcode::Load, &[], &[F32], insert_load_store),
    (Opcode::Load, &[], &[F64], insert_load_store),
    // Special Loads
    (Opcode::Uload8, &[], &[I16], insert_load_store),
    (Opcode::Uload8, &[], &[I32], insert_load_store),
    (Opcode::Uload8, &[], &[I64], insert_load_store),
    (Opcode::Uload16, &[], &[I32], insert_load_store),
    (Opcode::Uload16, &[], &[I64], insert_load_store),
    (Opcode::Uload32, &[], &[I64], insert_load_store),
    (Opcode::Sload8, &[], &[I16], insert_load_store),
    (Opcode::Sload8, &[], &[I32], insert_load_store),
    (Opcode::Sload8, &[], &[I64], insert_load_store),
    (Opcode::Sload16, &[], &[I32], insert_load_store),
    (Opcode::Sload16, &[], &[I64], insert_load_store),
    (Opcode::Sload32, &[], &[I64], insert_load_store),
    // TODO: Unimplemented in the interpreter
    // Opcode::Uload8x8
    // Opcode::Sload8x8
    // Opcode::Uload16x4
    // Opcode::Sload16x4
    // Opcode::Uload32x2
    // Opcode::Sload32x2
    // Stores
    (Opcode::Store, &[I8], &[], insert_load_store),
    (Opcode::Store, &[I16], &[], insert_load_store),
    (Opcode::Store, &[I32], &[], insert_load_store),
    (Opcode::Store, &[I64], &[], insert_load_store),
    (Opcode::Store, &[I128], &[], insert_load_store),
    (Opcode::Store, &[F32], &[], insert_load_store),
    (Opcode::Store, &[F64], &[], insert_load_store),
    // Special Stores
    (Opcode::Istore8, &[I16], &[], insert_load_store),
    (Opcode::Istore8, &[I32], &[], insert_load_store),
    (Opcode::Istore8, &[I64], &[], insert_load_store),
    (Opcode::Istore16, &[I32], &[], insert_load_store),
    (Opcode::Istore16, &[I64], &[], insert_load_store),
    (Opcode::Istore32, &[I64], &[], insert_load_store),
    // Integer Consts
    (Opcode::Iconst, &[], &[I8], insert_const),
    (Opcode::Iconst, &[], &[I16], insert_const),
    (Opcode::Iconst, &[], &[I32], insert_const),
    (Opcode::Iconst, &[], &[I64], insert_const),
    (Opcode::Iconst, &[], &[I128], insert_const),
    // Float Consts
    (Opcode::F32const, &[], &[F32], insert_const),
    (Opcode::F64const, &[], &[F64], insert_const),
    // Call
    (Opcode::Call, &[], &[], insert_call),
];

/// These libcalls need a interpreter implementation in `cranelift-fuzzgen.rs`
const ALLOWED_LIBCALLS: &'static [LibCall] = &[
    LibCall::CeilF32,
    LibCall::CeilF64,
    LibCall::FloorF32,
    LibCall::FloorF64,
    LibCall::TruncF32,
    LibCall::TruncF64,
];

pub struct FunctionGenerator<'r, 'data>
where
    'data: 'r,
{
    u: &'r mut Unstructured<'data>,
    config: &'r Config,
    resources: Resources,
}

#[derive(Debug, Clone)]
enum BlockTerminator {
    Return,
    Jump(Block),
    Br(Block, Block),
    BrIcmp(Block, Block),
    BrTable(Block, Vec<Block>),
    Switch(Type, Block, HashMap<u128, Block>),
}

#[derive(Debug, Clone)]
enum BlockTerminatorKind {
    Return,
    Jump,
    Br,
    BrIcmp,
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
    pub fn new(u: &'r mut Unstructured<'data>, config: &'r Config) -> Self {
        Self {
            u,
            config,
            resources: Resources::default(),
        }
    }

    /// Generates a random value for config `param`
    fn param(&mut self, param: &RangeInclusive<usize>) -> Result<usize> {
        Ok(self.u.int_in_range(param.clone())?)
    }

    fn generate_callconv(&mut self) -> Result<CallConv> {
        // TODO: Generate random CallConvs per target
        Ok(CallConv::SystemV)
    }

    fn system_callconv(&mut self) -> CallConv {
        // TODO: This currently only runs on linux, so this is the only choice
        // We should improve this once we generate flags and targets
        CallConv::SystemV
    }

    fn generate_type(&mut self) -> Result<Type> {
        // TODO: It would be nice if we could get these directly from cranelift
        let scalars = [
            // IFLAGS, FFLAGS,
            I8, I16, I32, I64, I128, F32, F64,
            // R32, R64,
        ];
        // TODO: vector types

        let ty = self.u.choose(&scalars[..])?;
        Ok(*ty)
    }

    fn generate_abi_param(&mut self) -> Result<AbiParam> {
        let value_type = self.generate_type()?;
        // TODO: There are more argument purposes to be explored...
        let purpose = ArgumentPurpose::Normal;
        let extension = match self.u.int_in_range(0..=2)? {
            2 => ArgumentExtension::Sext,
            1 => ArgumentExtension::Uext,
            _ => ArgumentExtension::None,
        };

        Ok(AbiParam {
            value_type,
            purpose,
            extension,
        })
    }

    fn generate_signature(&mut self) -> Result<Signature> {
        let callconv = self.generate_callconv()?;
        let mut sig = Signature::new(callconv);

        for _ in 0..self.param(&self.config.signature_params)? {
            sig.params.push(self.generate_abi_param()?);
        }

        for _ in 0..self.param(&self.config.signature_rets)? {
            sig.returns.push(self.generate_abi_param()?);
        }

        Ok(sig)
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
    /// The address is not guaranteed to be valid, but there's a chance that it is.
    ///
    /// `min_size`: Controls the amount of space that the address should have.This is not
    /// guaranteed to be respected
    fn generate_load_store_address(
        &mut self,
        builder: &mut FunctionBuilder,
        min_size: u32,
    ) -> Result<(Value, Offset32)> {
        // TODO: Currently our only source of addresses is stack_addr, but we should
        // add heap_addr, global_value, symbol_value eventually
        let (addr, available_size) = {
            let (ss, slot_size) = self.stack_slot_with_size(min_size)?;
            let max_offset = slot_size.saturating_sub(min_size);
            let offset = self.u.int_in_range(0..=max_offset)? as i32;
            let base_addr = builder.ins().stack_addr(I64, ss, offset);
            let available_size = (slot_size as i32).saturating_sub(offset);
            (base_addr, available_size)
        };

        // TODO: Insert a bunch of amode opcodes here to modify the address!

        // Now that we have an address and a size, we just choose a random offset to return to the
        // caller. Try to preserve min_size bytes.
        let max_offset = available_size.saturating_sub(min_size as i32);
        let offset = self.u.int_in_range(0..=max_offset)? as i32;

        Ok((addr, offset.into()))
    }

    /// Get a variable of type `ty` from the current function
    fn get_variable_of_type(&mut self, ty: Type) -> Result<Variable> {
        let opts = self.resources.vars.get(&ty).map_or(&[][..], Vec::as_slice);
        let var = self.u.choose(opts)?;
        Ok(*var)
    }

    /// Generates an instruction(`iconst`/`fconst`/etc...) to introduce a constant value
    fn generate_const(&mut self, builder: &mut FunctionBuilder, ty: Type) -> Result<Value> {
        Ok(match ty {
            I128 => {
                // See: https://github.com/bytecodealliance/wasmtime/issues/2906
                let hi = builder.ins().iconst(I64, self.u.arbitrary::<i64>()?);
                let lo = builder.ins().iconst(I64, self.u.arbitrary::<i64>()?);
                builder.ins().iconcat(lo, hi)
            }
            ty if ty.is_int() => {
                let imm64 = match ty {
                    I8 => self.u.arbitrary::<i8>()? as i64,
                    I16 => self.u.arbitrary::<i16>()? as i64,
                    I32 => self.u.arbitrary::<i32>()? as i64,
                    I64 => self.u.arbitrary::<i64>()?,
                    _ => unreachable!(),
                };
                builder.ins().iconst(ty, imm64)
            }
            // f{32,64}::arbitrary does not generate a bunch of important values
            // such as Signaling NaN's / NaN's with payload, so generate floats from integers.
            F32 => builder
                .ins()
                .f32const(f32::from_bits(u32::arbitrary(self.u)?)),
            F64 => builder
                .ins()
                .f64const(f64::from_bits(u64::arbitrary(self.u)?)),
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

                if bool::arbitrary(self.u)? {
                    builder.ins().brz(val, left, &left_args[..]);
                } else {
                    builder.ins().brnz(val, left, &left_args[..]);
                }
                builder.ins().jump(right, &right_args[..]);
            }
            BlockTerminator::BrIcmp(left, right) => {
                let cc = *self.u.choose(IntCC::all())?;
                let _type = *self.u.choose(&[I8, I16, I32, I64, I128])?;

                let lhs = builder.use_var(self.get_variable_of_type(_type)?);
                let rhs = builder.use_var(self.get_variable_of_type(_type)?);

                let left_args = self.generate_values_for_block(builder, left)?;
                let right_args = self.generate_values_for_block(builder, right)?;

                builder.ins().br_icmp(cc, lhs, rhs, left, &left_args[..]);
                builder.ins().jump(right, &right_args[..]);
            }
            BlockTerminator::BrTable(default, targets) => {
                // Create jump tables on demand
                let jt = builder.create_jump_table(JumpTableData::with_blocks(targets));

                // br_table only supports I32
                let val = builder.use_var(self.get_variable_of_type(I32)?);

                builder.ins().br_table(val, default, jt);
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
            let (op, args, rets, inserter) = *self.u.choose(OPCODE_SIGNATURES)?;
            inserter(self, builder, op, args, rets)?;
        }

        Ok(())
    }

    fn generate_funcrefs(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let count = self.param(&self.config.funcrefs_per_function)?;
        for func_index in 0..count.try_into().unwrap() {
            let (ext_name, sig) = if self.u.arbitrary::<bool>()? {
                let user_func_ref = builder
                    .func
                    .declare_imported_user_function(UserExternalName {
                        namespace: 0,
                        index: func_index,
                    });
                let name = ExternalName::User(user_func_ref);
                let signature = self.generate_signature()?;
                (name, signature)
            } else {
                let libcall = *self.u.choose(ALLOWED_LIBCALLS)?;
                // TODO: Use [CallConv::for_libcall] once we generate flags.
                let callconv = self.system_callconv();
                let signature = libcall.signature(callconv);
                (ExternalName::LibCall(libcall), signature)
            };

            let sig_ref = builder.import_signature(sig.clone());
            let func_ref = builder.import_function(ExtFuncData {
                name: ext_name,
                signature: sig_ref,
                colocated: self.u.arbitrary()?,
            });

            self.resources.func_refs.push((sig, func_ref));
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
        let i128_zero = builder.ins().iconst(I128, 0);
        let i64_zero = builder.ins().iconst(I64, 0);
        let i32_zero = builder.ins().iconst(I32, 0);
        let i16_zero = builder.ins().iconst(I16, 0);
        let i8_zero = builder.ins().iconst(I8, 0);

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
    fn generate_blocks(&mut self, builder: &mut FunctionBuilder, sig: &Signature) -> Result<()> {
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
                    Ok((block, sig.params.iter().map(|a| a.value_type).collect()))
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
                    // TODO: We could add some kind of BrReturn/BrIcmpReturn here, to explore edges where we exit
                    // in the middle of the function
                    valid_terminators.extend_from_slice(&[
                        BlockTerminatorKind::Jump,
                        BlockTerminatorKind::Br,
                        BlockTerminatorKind::BrIcmp,
                    ]);
                }

                // BrTable and the Switch interface only allow targeting blocks without params
                // we also need to ensure that the next block has no params, since that one is
                // guaranteed to be picked in either case.
                if has_paramless_targets && next_block_is_paramless {
                    valid_terminators.extend_from_slice(&[
                        BlockTerminatorKind::BrTable,
                        BlockTerminatorKind::Switch,
                    ]);
                }

                let terminator = self.u.choose(&valid_terminators[..])?;

                // Choose block targets for the terminators that we picked above
                Ok(match terminator {
                    BlockTerminatorKind::Return => BlockTerminator::Return,
                    BlockTerminatorKind::Jump => BlockTerminator::Jump(next_block),
                    BlockTerminatorKind::Br => {
                        BlockTerminator::Br(next_block, self.generate_target_block(block)?)
                    }
                    BlockTerminatorKind::BrIcmp => {
                        BlockTerminator::BrIcmp(next_block, self.generate_target_block(block)?)
                    }
                    // TODO: Allow generating backwards branches here
                    BlockTerminatorKind::BrTable => {
                        // Make the default the next block, and then we don't have to worry
                        // that we can reach it via the targets
                        let default = next_block;

                        let target_count = self.param(&self.config.jump_table_entries)?;
                        let targets = arbitrary_vec(
                            self.u,
                            target_count,
                            self.resources.forward_blocks_without_params(block),
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
            params.push(self.generate_type()?);
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
            let ty = self.generate_type()?;
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
        let sig = self.generate_signature()?;

        let mut fn_builder_ctx = FunctionBuilderContext::new();
        // function name must be in a different namespace than TESTFILE_NAMESPACE (0)
        let mut func = Function::with_name_signature(UserFuncName::user(1, 0), sig.clone());

        let mut builder = FunctionBuilder::new(&mut func, &mut fn_builder_ctx);

        self.generate_blocks(&mut builder, &sig)?;

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
