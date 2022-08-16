use crate::codegen::ir::{ArgumentExtension, ArgumentPurpose, ValueList};
use crate::config::Config;
use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use cranelift::codegen::ir::{types::*, FuncRef, LibCall, UserExternalName, UserFuncName};
use cranelift::codegen::ir::{
    AbiParam, Block, ExternalName, Function, JumpTable, Opcode, Signature, StackSlot, Type, Value,
};
use cranelift::codegen::isa::CallConv;
use cranelift::frontend::{FunctionBuilder, FunctionBuilderContext, Switch, Variable};
use cranelift::prelude::{
    EntityRef, ExtFuncData, InstBuilder, IntCC, JumpTableData, StackSlotData, StackSlotKind,
};
use std::collections::HashMap;
use std::ops::RangeInclusive;

type BlockSignature = Vec<Type>;

fn insert_opcode(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    args: &'static [Type],
    rets: &'static [Type],
) -> Result<()> {
    let mut arg_vals = ValueList::new();
    for &arg in args.into_iter() {
        let var = fgen.get_variable_of_type(arg)?;
        let val = builder.use_var(var);
        arg_vals.push(val, &mut builder.func.dfg.value_lists);
    }

    let typevar = rets.first().copied().unwrap_or(INVALID);
    let (inst, dfg) = builder.ins().MultiAry(opcode, typevar, arg_vals);
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
    let (sig, func_ref) = fgen.u.choose(&fgen.func_refs)?.clone();

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
    let slot = fgen.stack_slot_with_size(builder, typevar.bytes())?;
    let slot_size = builder.func.sized_stack_slots[slot].size;
    let type_size = typevar.bytes();
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
    let slot = fgen.stack_slot_with_size(builder, typevar.bytes())?;
    let slot_size = builder.func.sized_stack_slots[slot].size;
    let type_size = typevar.bytes();
    let offset = fgen.u.int_in_range(0..=(slot_size - type_size))? as i32;

    let arg0 = fgen.get_variable_of_type(typevar)?;
    let arg0 = builder.use_var(arg0);

    builder.ins().stack_store(arg0, slot, offset);
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
    (Opcode::Udiv, &[I128, I128], &[I128], insert_opcode),
    // Sdiv
    (Opcode::Sdiv, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Sdiv, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Sdiv, &[I32, I32], &[I32], insert_opcode),
    (Opcode::Sdiv, &[I64, I64], &[I64], insert_opcode),
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
    // Some test cases disabled due to: https://github.com/bytecodealliance/wasmtime/issues/4699
    (Opcode::Ishl, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Ishl, &[I8, I16], &[I8], insert_opcode),
    (Opcode::Ishl, &[I8, I32], &[I8], insert_opcode),
    (Opcode::Ishl, &[I8, I64], &[I8], insert_opcode),
    // (Opcode::Ishl, &[I8, I128], &[I8], insert_opcode),
    (Opcode::Ishl, &[I16, I8], &[I16], insert_opcode),
    (Opcode::Ishl, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Ishl, &[I16, I32], &[I16], insert_opcode),
    (Opcode::Ishl, &[I16, I64], &[I16], insert_opcode),
    // (Opcode::Ishl, &[I16, I128], &[I16], insert_opcode),
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
    // Some test cases disabled due to: https://github.com/bytecodealliance/wasmtime/issues/4699
    (Opcode::Sshr, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Sshr, &[I8, I16], &[I8], insert_opcode),
    (Opcode::Sshr, &[I8, I32], &[I8], insert_opcode),
    (Opcode::Sshr, &[I8, I64], &[I8], insert_opcode),
    // (Opcode::Sshr, &[I8, I128], &[I8], insert_opcode),
    (Opcode::Sshr, &[I16, I8], &[I16], insert_opcode),
    (Opcode::Sshr, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Sshr, &[I16, I32], &[I16], insert_opcode),
    (Opcode::Sshr, &[I16, I64], &[I16], insert_opcode),
    // (Opcode::Sshr, &[I16, I128], &[I16], insert_opcode),
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
    // Some test cases disabled due to: https://github.com/bytecodealliance/wasmtime/issues/4699
    (Opcode::Ushr, &[I8, I8], &[I8], insert_opcode),
    (Opcode::Ushr, &[I8, I16], &[I8], insert_opcode),
    (Opcode::Ushr, &[I8, I32], &[I8], insert_opcode),
    (Opcode::Ushr, &[I8, I64], &[I8], insert_opcode),
    // (Opcode::Ushr, &[I8, I128], &[I8], insert_opcode),
    (Opcode::Ushr, &[I16, I8], &[I16], insert_opcode),
    (Opcode::Ushr, &[I16, I16], &[I16], insert_opcode),
    (Opcode::Ushr, &[I16, I32], &[I16], insert_opcode),
    (Opcode::Ushr, &[I16, I64], &[I16], insert_opcode),
    // (Opcode::Ushr, &[I16, I128], &[I16], insert_opcode),
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
    // Integer Consts
    (Opcode::Iconst, &[], &[I8], insert_const),
    (Opcode::Iconst, &[], &[I16], insert_const),
    (Opcode::Iconst, &[], &[I32], insert_const),
    (Opcode::Iconst, &[], &[I64], insert_const),
    (Opcode::Iconst, &[], &[I128], insert_const),
    // Float Consts
    (Opcode::F32const, &[], &[F32], insert_const),
    (Opcode::F64const, &[], &[F64], insert_const),
    // Bool Consts
    (Opcode::Bconst, &[], &[B1], insert_const),
    // Call
    (Opcode::Call, &[], &[], insert_call),
];

pub struct FunctionGenerator<'r, 'data>
where
    'data: 'r,
{
    u: &'r mut Unstructured<'data>,
    config: &'r Config,
    vars: Vec<(Type, Variable)>,
    blocks: Vec<(Block, BlockSignature)>,
    jump_tables: Vec<JumpTable>,
    func_refs: Vec<(Signature, FuncRef)>,
    next_func_index: u32,
    static_stack_slots: Vec<StackSlot>,
}

impl<'r, 'data> FunctionGenerator<'r, 'data>
where
    'data: 'r,
{
    pub fn new(u: &'r mut Unstructured<'data>, config: &'r Config) -> Self {
        Self {
            u,
            config,
            vars: vec![],
            blocks: vec![],
            jump_tables: vec![],
            func_refs: vec![],
            static_stack_slots: vec![],
            next_func_index: 0,
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

    fn generate_intcc(&mut self) -> Result<IntCC> {
        Ok(*self.u.choose(
            &[
                IntCC::Equal,
                IntCC::NotEqual,
                IntCC::SignedLessThan,
                IntCC::SignedGreaterThanOrEqual,
                IntCC::SignedGreaterThan,
                IntCC::SignedLessThanOrEqual,
                IntCC::UnsignedLessThan,
                IntCC::UnsignedGreaterThanOrEqual,
                IntCC::UnsignedGreaterThan,
                IntCC::UnsignedLessThanOrEqual,
                IntCC::Overflow,
                IntCC::NotOverflow,
            ][..],
        )?)
    }

    fn generate_type(&mut self) -> Result<Type> {
        // TODO: It would be nice if we could get these directly from cranelift
        let scalars = [
            // IFLAGS, FFLAGS,
            B1, // B8, B16, B32, B64, B128,
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
    fn stack_slot_with_size(&mut self, builder: &mut FunctionBuilder, n: u32) -> Result<StackSlot> {
        let opts: Vec<_> = self
            .static_stack_slots
            .iter()
            .filter(|ss| builder.func.sized_stack_slots[**ss].size >= n)
            .map(|ss| *ss)
            .collect();

        Ok(*self.u.choose(&opts[..])?)
    }

    /// Creates a new var
    fn create_var(&mut self, builder: &mut FunctionBuilder, ty: Type) -> Result<Variable> {
        let id = self.vars.len();
        let var = Variable::new(id);
        builder.declare_var(var, ty);
        self.vars.push((ty, var));
        Ok(var)
    }

    fn vars_of_type(&self, ty: Type) -> Vec<Variable> {
        self.vars
            .iter()
            .filter(|(var_ty, _)| *var_ty == ty)
            .map(|(_, v)| *v)
            .collect()
    }

    /// Get a variable of type `ty` from the current function
    fn get_variable_of_type(&mut self, ty: Type) -> Result<Variable> {
        let opts = self.vars_of_type(ty);
        let var = self.u.choose(&opts[..])?;
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
            ty if ty.is_bool() => builder.ins().bconst(ty, bool::arbitrary(self.u)?),
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
    ///
    /// For convenience we also generate values that match the block's signature
    fn generate_target_block(
        &mut self,
        builder: &mut FunctionBuilder,
    ) -> Result<(Block, Vec<Value>)> {
        let block_targets = &self.blocks[1..];
        let (block, signature) = self.u.choose(block_targets)?.clone();
        let args = self.generate_values_for_signature(builder, signature.into_iter())?;
        Ok((block, args))
    }

    /// Valid blocks for jump tables have to have no parameters in the signature, and must also
    /// not be the first block.
    fn generate_valid_jumptable_target_blocks(&mut self) -> Vec<Block> {
        self.blocks[1..]
            .iter()
            .filter(|(_, sig)| sig.len() == 0)
            .map(|(b, _)| *b)
            .collect()
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

    fn generate_return(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let types: Vec<Type> = {
            let rets = &builder.func.signature.returns;
            rets.iter().map(|p| p.value_type).collect()
        };
        let vals = self.generate_values_for_signature(builder, types.into_iter())?;

        builder.ins().return_(&vals[..]);
        Ok(())
    }

    fn generate_jump(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let (block, args) = self.generate_target_block(builder)?;
        builder.ins().jump(block, &args[..]);
        Ok(())
    }

    /// Generates a br_table into a random block
    fn generate_br_table(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let var = self.get_variable_of_type(I32)?; // br_table only supports I32
        let val = builder.use_var(var);

        let valid_blocks = self.generate_valid_jumptable_target_blocks();
        let default_block = *self.u.choose(&valid_blocks[..])?;

        let jt = *self.u.choose(&self.jump_tables[..])?;
        builder.ins().br_table(val, default_block, jt);
        Ok(())
    }

    /// Generates a brz/brnz into a random block
    fn generate_br(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let (block, args) = self.generate_target_block(builder)?;

        let condbr_types = [I8, I16, I32, I64, I128, B1];
        let _type = *self.u.choose(&condbr_types[..])?;
        let var = self.get_variable_of_type(_type)?;
        let val = builder.use_var(var);

        if bool::arbitrary(self.u)? {
            builder.ins().brz(val, block, &args[..]);
        } else {
            builder.ins().brnz(val, block, &args[..]);
        }

        // After brz/brnz we must generate a jump
        self.generate_jump(builder)?;
        Ok(())
    }

    fn generate_bricmp(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let (block, args) = self.generate_target_block(builder)?;
        let cond = self.generate_intcc()?;

        let bricmp_types = [
            I8, I16, I32,
            I64,
            // I128 - TODO: https://github.com/bytecodealliance/wasmtime/issues/4406
        ];
        let _type = *self.u.choose(&bricmp_types[..])?;

        let lhs_var = self.get_variable_of_type(_type)?;
        let lhs_val = builder.use_var(lhs_var);

        let rhs_var = self.get_variable_of_type(_type)?;
        let rhs_val = builder.use_var(rhs_var);

        builder
            .ins()
            .br_icmp(cond, lhs_val, rhs_val, block, &args[..]);

        // After bricmp's we must generate a jump
        self.generate_jump(builder)?;
        Ok(())
    }

    fn generate_switch(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let _type = *self.u.choose(&[I8, I16, I32, I64, I128][..])?;
        let switch_var = self.get_variable_of_type(_type)?;
        let switch_val = builder.use_var(switch_var);

        let valid_blocks = self.generate_valid_jumptable_target_blocks();
        let default_block = *self.u.choose(&valid_blocks[..])?;

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
                let block = *self.u.choose(&valid_blocks[..])?;
                entries.insert(index, block);
            }
        }

        let mut switch = Switch::new();
        for (entry, block) in entries.into_iter() {
            switch.set_entry(entry, block);
        }
        switch.emit(builder, switch_val, default_block);

        Ok(())
    }

    /// We always need to exit safely out of a block.
    /// This either means a jump into another block or a return.
    fn finalize_block(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let gen = self.u.choose(
            &[
                Self::generate_bricmp,
                Self::generate_br,
                Self::generate_br_table,
                Self::generate_jump,
                Self::generate_return,
                Self::generate_switch,
            ][..],
        )?;

        gen(self, builder)
    }

    /// Fills the current block with random instructions
    fn generate_instructions(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        for _ in 0..self.param(&self.config.instructions_per_block)? {
            let (op, args, rets, inserter) = *self.u.choose(OPCODE_SIGNATURES)?;
            inserter(self, builder, op, args, rets)?;
        }

        Ok(())
    }

    fn generate_jumptables(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let valid_blocks = self.generate_valid_jumptable_target_blocks();

        for _ in 0..self.param(&self.config.jump_tables_per_function)? {
            let mut jt_data = JumpTableData::new();

            for _ in 0..self.param(&self.config.jump_table_entries)? {
                let block = *self.u.choose(&valid_blocks[..])?;
                jt_data.push_entry(block);
            }

            self.jump_tables.push(builder.create_jump_table(jt_data));
        }
        Ok(())
    }

    fn generate_funcrefs(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        for _ in 0..self.param(&self.config.funcrefs_per_function)? {
            let (ext_name, sig) = if self.u.arbitrary::<bool>()? {
                let func_index = self.next_func_index;
                self.next_func_index = self.next_func_index.wrapping_add(1);
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
                // Use udivi64 as an example of a libcall function.
                let mut signature = Signature::new(CallConv::Fast);
                signature.params.push(AbiParam::new(I64));
                signature.params.push(AbiParam::new(I64));
                signature.returns.push(AbiParam::new(I64));
                (ExternalName::LibCall(LibCall::UdivI64), signature)
            };

            let sig_ref = builder.import_signature(sig.clone());
            let func_ref = builder.import_function(ExtFuncData {
                name: ext_name,
                signature: sig_ref,
                colocated: self.u.arbitrary()?,
            });

            self.func_refs.push((sig, func_ref));
        }

        Ok(())
    }

    fn generate_stack_slots(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        for _ in 0..self.param(&self.config.static_stack_slots_per_function)? {
            let bytes = self.param(&self.config.static_stack_slot_size)? as u32;
            let ss_data = StackSlotData::new(StackSlotKind::ExplicitSlot, bytes);
            let slot = builder.create_sized_stack_slot(ss_data);

            self.static_stack_slots.push(slot);
        }
        Ok(())
    }

    /// Zero initializes the stack slot by inserting `stack_store`'s.
    fn initialize_stack_slots(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let i128_zero = builder.ins().iconst(I128, 0);
        let i64_zero = builder.ins().iconst(I64, 0);
        let i32_zero = builder.ins().iconst(I32, 0);
        let i16_zero = builder.ins().iconst(I16, 0);
        let i8_zero = builder.ins().iconst(I8, 0);

        for &slot in self.static_stack_slots.iter() {
            let init_size = builder.func.sized_stack_slots[slot].size;
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
    fn generate_blocks(
        &mut self,
        builder: &mut FunctionBuilder,
        sig: &Signature,
    ) -> Result<Vec<(Block, BlockSignature)>> {
        let extra_block_count = self.param(&self.config.blocks_per_function)?;

        // We must always have at least one block, so we generate the "extra" blocks and add 1 for
        // the entry block.
        let block_count = 1 + extra_block_count;

        let blocks = (0..block_count)
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

        Ok(blocks)
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
        let func_params = builder.func.signature.params.clone();

        // Define variables for the function signature
        for (i, param) in func_params.iter().enumerate() {
            let var = self.create_var(builder, param.value_type)?;
            let block_param = builder.block_params(block)[i];
            builder.def_var(var, block_param);
        }

        // Create a pool of vars that are going to be used in this function
        for _ in 0..self.param(&self.config.vars_per_function)? {
            let ty = self.generate_type()?;
            let var = self.create_var(builder, ty)?;
            let value = self.generate_const(builder, ty)?;
            builder.def_var(var, value);
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
        let mut func = Function::with_name_signature(UserFuncName::user(0, 1), sig.clone());

        let mut builder = FunctionBuilder::new(&mut func, &mut fn_builder_ctx);

        self.blocks = self.generate_blocks(&mut builder, &sig)?;

        // Function preamble
        self.generate_jumptables(&mut builder)?;
        self.generate_funcrefs(&mut builder)?;
        self.generate_stack_slots(&mut builder)?;

        // Main instruction generation loop
        for (i, (block, block_sig)) in self.blocks.clone().iter().enumerate() {
            let is_block0 = i == 0;
            builder.switch_to_block(*block);

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
                    let block_param = builder.block_params(*block)[i];
                    builder.def_var(var, block_param);
                }
            }

            // Generate block instructions
            self.generate_instructions(&mut builder)?;

            self.finalize_block(&mut builder)?;
        }

        builder.seal_all_blocks();
        builder.finalize();

        Ok(func)
    }
}
