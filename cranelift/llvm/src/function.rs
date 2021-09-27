use std::collections::HashMap;

use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::immediates::{Imm64, Offset32};
use cranelift_codegen::ir::{
    types, AtomicRmwOp, Block, InstructionData, Opcode, Signature, StackSlot,
};
use cranelift_module::{DataId, Module as _, ModuleResult};
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::types::{BasicType, BasicTypeEnum, FloatType, FunctionType, IntType, VectorType};
use inkwell::values::{
    AnyValue, BasicValue, BasicValueEnum, CallableValue, IntValue, PhiValue, PointerValue,
};
use inkwell::{AddressSpace, AtomicOrdering, AtomicRMWBinOp, FloatPredicate, IntPredicate};

fn translate_int_ty<'ctx>(
    context: &'ctx Context,
    ty: cranelift_codegen::ir::Type,
) -> IntType<'ctx> {
    match ty {
        types::I8 => context.i8_type(),
        types::I16 => context.i16_type(),
        types::I32 => context.i32_type(),
        types::I64 => context.i64_type(),
        types::I128 => context.i128_type(),
        _ => unreachable!(),
    }
}

fn translate_float_ty<'ctx>(
    context: &'ctx Context,
    ty: cranelift_codegen::ir::Type,
) -> FloatType<'ctx> {
    match ty {
        types::F32 => context.f32_type(),
        types::F64 => context.f64_type(),
        _ => unreachable!(),
    }
}

fn translate_vector_ty<'ctx>(
    context: &'ctx Context,
    ty: cranelift_codegen::ir::Type,
) -> VectorType<'ctx> {
    assert!(ty.is_vector());
    match ty.lane_type() {
        types::I8 => context.i8_type().vec_type(ty.lane_count().into()).into(),
        types::I16 => context.i16_type().vec_type(ty.lane_count().into()).into(),
        types::I32 => context.i32_type().vec_type(ty.lane_count().into()).into(),
        types::I64 => context.i64_type().vec_type(ty.lane_count().into()).into(),
        types::I128 => context.i128_type().vec_type(ty.lane_count().into()).into(),
        types::F32 => context.f32_type().vec_type(ty.lane_count().into()).into(),
        types::F64 => context.f64_type().vec_type(ty.lane_count().into()).into(),
        _ => unreachable!(),
    }
}

fn translate_ty<'ctx>(
    context: &'ctx Context,
    ty: cranelift_codegen::ir::Type,
) -> BasicTypeEnum<'ctx> {
    if !ty.is_vector() {
        match ty.lane_type() {
            types::I8 => context.i8_type().into(),
            types::I16 => context.i16_type().into(),
            types::I32 => context.i32_type().into(),
            types::I64 => context.i64_type().into(),
            types::I128 => context.i128_type().into(),
            types::F32 => context.f32_type().into(),
            types::F64 => context.f64_type().into(),
            _ => unreachable!(),
        }
    } else {
        translate_vector_ty(context, ty).into()
    }
}

fn translate_imm64<'ctx>(
    context: &'ctx Context,
    ty: cranelift_codegen::ir::Type,
    imm: Imm64,
) -> IntValue<'ctx> {
    let ty = translate_int_ty(context, ty);
    let imm: i64 = imm.into();
    ty.const_int(imm as u64, false /* FIXME right value? */)
}

fn translate_ptr_no_offset<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    pointee_ty: cranelift_codegen::ir::Type,
    ptr: IntValue<'ctx>,
) -> PointerValue<'ctx> {
    let pointee_ty = translate_ty(context, pointee_ty);
    builder.build_int_to_ptr(ptr, pointee_ty.ptr_type(AddressSpace::Generic), "ptr")
}

fn translate_ptr_offset32<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    pointee_ty: cranelift_codegen::ir::Type,
    ptr: IntValue<'ctx>,
    offset: Offset32,
) -> PointerValue<'ctx> {
    let ptr_ty = ptr.get_type();
    let pointee_ty = translate_ty(context, pointee_ty);
    let offset: i64 = offset.into();
    let offset = ptr_ty.const_int(offset as u64, false /* FIXME right value? */);
    let ptr = builder.build_int_add(ptr, offset, "ptr_val");
    builder.build_int_to_ptr(ptr, pointee_ty.ptr_type(AddressSpace::Generic), "ptr")
}

pub fn translate_sig<'ctx>(
    context: &'ctx Context,
    signature: &Signature,
    is_var_args: bool, // FIXME add native vararg support to Cranelift
) -> FunctionType<'ctx> {
    let params = signature
        .params
        .iter()
        .map(|param| translate_ty(context, param.value_type))
        .collect::<Vec<_>>();
    match &*signature.returns {
        [] => context.void_type().fn_type(&params, false),
        [ret_abi_param] => {
            translate_ty(context, ret_abi_param.value_type).fn_type(&params, is_var_args)
        }
        [ret_abi_param_a, ret_abi_param_b] => {
            let ret_ty_a = translate_ty(context, ret_abi_param_a.value_type);
            let ret_ty_b = translate_ty(context, ret_abi_param_b.value_type);
            context.struct_type(&[ret_ty_a, ret_ty_b], false).fn_type(&params, is_var_args)
        }
        _ => todo!(),
    }
}

pub fn define_function<'ctx>(
    module: &mut crate::LlvmModule<'ctx>,
    func_id: cranelift_module::FuncId,
    ctx: &mut cranelift_codegen::Context,
) -> ModuleResult<()> {
    struct PrintOnPanic<F: Fn() -> String>(F);
    impl<F: Fn() -> String> Drop for PrintOnPanic<F> {
        fn drop(&mut self) {
            if ::std::thread::panicking() {
                println!("{}", (self.0)());
            }
        }
    }

    let isa = module.isa();
    if isa.flags().enable_nan_canonicalization() {
        ctx.canonicalize_nans(isa)?;
    }

    ctx.compute_cfg();
    ctx.legalize(isa)?;

    ctx.compute_domtree();
    // Mandatory for LLVM as dead phi nodes are not allowed
    // v
    ctx.eliminate_unreachable_code(isa)?;

    let func = &ctx.func;

    let _func_bomb = PrintOnPanic(|| format!("{}", func));

    let func_val = module.function_refs[&func_id];

    let _llvm_func_bomb = PrintOnPanic(|| {
        func_val.print_to_stderr();
        "".to_string()
    });

    let mut block_map: HashMap<Block, BasicBlock> = HashMap::new();
    let mut phi_map: HashMap<Block, Vec<PhiValue>> = HashMap::new();
    let mut val_map: HashMap<cranelift_codegen::ir::Value, BasicValueEnum<'ctx>> = HashMap::new();
    let mut val_placeholder_map: HashMap<cranelift_codegen::ir::Value, PhiValue<'ctx>> =
        HashMap::new();
    for block in func.layout.blocks() {
        block_map.insert(block, module.context.append_basic_block(func_val, &block.to_string()));
    }

    let mut stack_slot_map: HashMap<StackSlot, IntValue> = HashMap::new();
    module.builder.position_at_end(block_map[&func.layout.entry_block().unwrap()]);
    for (stack_slot, stack_slot_data) in func.stack_slots.iter() {
        let ptr_ty = module.context.i64_type(); // FIXME;
        let ptr = module.builder.build_alloca(
            module.context.i8_type().array_type(stack_slot_data.size),
            &format!("{}_ptr", stack_slot),
        );
        ptr.as_instruction().unwrap().set_alignment(16).unwrap(); // FIXME add alignment specification to clif ir
        stack_slot_map.insert(
            stack_slot,
            module.builder.build_ptr_to_int(ptr, ptr_ty, &format!("{}", stack_slot)),
        );
    }

    macro_rules! use_val {
        ($val:expr) => {{
            let val = func.dfg.resolve_aliases($val);
            *val_map.entry(val).or_insert_with(|| {
                let phi = module.builder.build_phi(
                    translate_ty(module.context, func.dfg.value_type(val)),
                    &val.to_string(),
                );
                val_placeholder_map.insert(val, phi);
                phi.as_basic_value()
            })
        }};
    }

    macro_rules! def_val {
        ($val:expr, $ret:expr $(,)?) => {{
            let val = $val;
            let ret: BasicValueEnum = $ret;
            match val_map.entry(val) {
                std::collections::hash_map::Entry::Occupied(mut occ) => {
                    val_placeholder_map[&val].replace_all_uses_with(&ret);
                    val_placeholder_map[&val].as_instruction().erase_from_basic_block();
                    occ.insert(ret);
                }
                std::collections::hash_map::Entry::Vacant(vac) => {
                    vac.insert(ret);
                }
            }
        }};
    }

    #[allow(unused_variables)]
    let val_map = (); // Shadow val_map to ensure all accesses go though use_val!() and def_val!()

    for block in func.layout.blocks() {
        if block == func.layout.entry_block().unwrap() {
            for (i, &val) in func.dfg.block_params(block).iter().enumerate() {
                def_val!(val, func_val.get_nth_param(i as u32).unwrap().into());
            }
        } else {
            module.builder.position_at_end(block_map[&block]);
            let mut phis = vec![];
            for &val in func.dfg.block_params(block) {
                let phi = module.builder.build_phi(
                    translate_ty(module.context, func.dfg.value_type(val)),
                    &val.to_string(),
                );
                phis.push(phi);
                def_val!(val, phi.as_basic_value());
            }
            phi_map.insert(block, phis);
        }
    }

    for block in func.layout.blocks() {
        module.builder.position_at_end(block_map[&block]);
        for inst in func.layout.block_insts(block) {
            let res_vals = func.dfg.inst_results(inst);
            match &func.dfg[inst] {
                InstructionData::NullAry { opcode: Opcode::Nop } => {}
                InstructionData::NullAry { opcode: Opcode::Fence } => {
                    module.builder.build_fence(AtomicOrdering::SequentiallyConsistent, 0, "");
                }

                InstructionData::Unary { opcode: Opcode::Bitcast | Opcode::RawBitcast, arg } => {
                    let arg = use_val!(*arg);
                    let res = module.builder.build_bitcast(
                        arg,
                        translate_ty(module.context, func.dfg.ctrl_typevar(inst)),
                        &res_vals[0].to_string(),
                    );
                    def_val!(res_vals[0], res);
                }
                InstructionData::Unary { opcode: Opcode::Isplit, arg } => {
                    let arg = use_val!(*arg).into_int_value();
                    assert!(func.dfg.ctrl_typevar(inst) == types::I128);
                    let lsb = module
                        .builder
                        .build_int_truncate(
                            arg,
                            module.context.i64_type(),
                            &res_vals[0].to_string(),
                        )
                        .as_basic_value_enum();
                    let msb_untruncated = module.builder.build_right_shift(
                        arg,
                        module.context.i128_type().const_int(64, false),
                        false,
                        &format!("{}_untrunc", res_vals[1]),
                    );
                    let msb = module
                        .builder
                        .build_int_truncate(
                            msb_untruncated,
                            module.context.i64_type(),
                            &res_vals[1].to_string(),
                        )
                        .as_basic_value_enum();
                    def_val!(res_vals[0], lsb);
                    def_val!(res_vals[1], msb);
                }
                InstructionData::Unary {
                    opcode:
                        opcode @ Opcode::Bnot
                        | opcode @ Opcode::Bint
                        | opcode @ Opcode::Ineg
                        | opcode @ Opcode::Uextend
                        | opcode @ Opcode::Sextend
                        | opcode @ Opcode::Ireduce
                        | opcode @ Opcode::FcvtFromUint
                        | opcode @ Opcode::FcvtFromSint
                        | opcode @ Opcode::Clz
                        | opcode @ Opcode::Ctz
                        | opcode @ Opcode::Bitrev
                        | opcode @ Opcode::Popcnt,
                    arg,
                } => {
                    let arg = use_val!(*arg).into_int_value();
                    let res = match opcode {
                        Opcode::Bnot => module
                            .builder
                            .build_not(arg, &res_vals[0].to_string())
                            .as_basic_value_enum(),
                        Opcode::Bint => module
                            .builder
                            .build_int_z_extend(
                                arg,
                                translate_int_ty(module.context, func.dfg.ctrl_typevar(inst)),
                                &res_vals[0].to_string(),
                            )
                            .as_basic_value_enum(),
                        Opcode::Ineg => module
                            .builder
                            .build_int_neg(arg, &res_vals[0].to_string())
                            .as_basic_value_enum(),
                        Opcode::Uextend => module
                            .builder
                            .build_int_z_extend(
                                arg,
                                translate_int_ty(module.context, func.dfg.ctrl_typevar(inst)),
                                &res_vals[0].to_string(),
                            )
                            .as_basic_value_enum(),
                        Opcode::Sextend => module
                            .builder
                            .build_int_s_extend(
                                arg,
                                translate_int_ty(module.context, func.dfg.ctrl_typevar(inst)),
                                &res_vals[0].to_string(),
                            )
                            .as_basic_value_enum(),
                        Opcode::Ireduce => module
                            .builder
                            .build_int_truncate(
                                arg,
                                translate_int_ty(module.context, func.dfg.ctrl_typevar(inst)),
                                &res_vals[0].to_string(),
                            )
                            .as_basic_value_enum(),
                        Opcode::FcvtFromUint => module
                            .builder
                            .build_unsigned_int_to_float(
                                arg,
                                translate_float_ty(module.context, func.dfg.ctrl_typevar(inst)),
                                &res_vals[0].to_string(),
                            )
                            .as_basic_value_enum(),
                        Opcode::FcvtFromSint => module
                            .builder
                            .build_signed_int_to_float(
                                arg,
                                translate_float_ty(module.context, func.dfg.ctrl_typevar(inst)),
                                &res_vals[0].to_string(),
                            )
                            .as_basic_value_enum(),
                        Opcode::Clz => {
                            let func = module.get_intrinsic(
                                format!("llvm.ctlz.i{}", func.dfg.ctrl_typevar(inst).bits()),
                                arg.get_type().fn_type(
                                    &[
                                        arg.get_type().as_basic_type_enum(),
                                        module.context.bool_type().as_basic_type_enum(),
                                    ],
                                    false,
                                ),
                            );
                            let res = module.builder.build_call(
                                func,
                                &[
                                    arg.into(),
                                    module.context.bool_type().const_zero().into(), /* zero UB? */
                                ],
                                &res_vals[0].to_string(),
                            );
                            res.try_as_basic_value().unwrap_left()
                        }
                        Opcode::Ctz => {
                            let func = module.get_intrinsic(
                                format!("llvm.cttz.i{}", func.dfg.ctrl_typevar(inst).bits()),
                                arg.get_type().fn_type(
                                    &[
                                        arg.get_type().as_basic_type_enum(),
                                        module.context.bool_type().as_basic_type_enum(),
                                    ],
                                    false,
                                ),
                            );
                            let res = module.builder.build_call(
                                func,
                                &[
                                    arg.into(),
                                    module.context.bool_type().const_zero().into(), /* zero UB? */
                                ],
                                &res_vals[0].to_string(),
                            );
                            res.try_as_basic_value().unwrap_left()
                        }
                        Opcode::Bitrev => {
                            let func = module.get_intrinsic(
                                format!("llvm.bitreverse.i{}", func.dfg.ctrl_typevar(inst).bits()),
                                arg.get_type()
                                    .fn_type(&[arg.get_type().as_basic_type_enum()], false),
                            );
                            let res = module.builder.build_call(
                                func,
                                &[arg.into()],
                                &res_vals[0].to_string(),
                            );
                            res.try_as_basic_value().unwrap_left()
                        }
                        Opcode::Popcnt => {
                            let func = module.get_intrinsic(
                                format!("llvm.ctpop.i{}", func.dfg.ctrl_typevar(inst).bits()),
                                arg.get_type()
                                    .fn_type(&[arg.get_type().as_basic_type_enum()], false),
                            );
                            let res = module.builder.build_call(
                                func,
                                &[arg.into()],
                                &res_vals[0].to_string(),
                            );
                            res.try_as_basic_value().unwrap_left()
                        }
                        _ => unreachable!(),
                    };
                    def_val!(res_vals[0], res);
                }
                InstructionData::Unary {
                    opcode:
                        opcode @ Opcode::Fneg
                        | opcode @ Opcode::Fpromote
                        | opcode @ Opcode::Fdemote
                        | opcode @ Opcode::FcvtToUintSat
                        | opcode @ Opcode::FcvtToSintSat,
                    arg,
                } => {
                    //let arg_ty = func.dfg.value_type(*arg);
                    let ret_ty = func.dfg.ctrl_typevar(inst);
                    let arg = use_val!(*arg).into_float_value();
                    let res = match opcode {
                        Opcode::Fneg => module
                            .builder
                            .build_float_neg(arg, &res_vals[0].to_string())
                            .as_basic_value_enum(),
                        Opcode::Fpromote => module
                            .builder
                            .build_float_ext(
                                arg,
                                translate_float_ty(&module.context, ret_ty),
                                &res_vals[0].to_string(),
                            )
                            .as_basic_value_enum(),
                        Opcode::Fdemote => module
                            .builder
                            .build_float_trunc(
                                arg,
                                translate_float_ty(&module.context, ret_ty),
                                &res_vals[0].to_string(),
                            )
                            .as_basic_value_enum(),
                        Opcode::FcvtToUintSat => {
                            // FIXME too old LLVM installed locally
                            /*
                            let func = module.get_intrinsic(
                                format!("llvm.fptoui.sat.i{}.f{}", ret_ty.bits(), arg_ty.bits(),),
                                translate_int_ty(&module.context, ret_ty)
                                    .fn_type(&[arg.get_type().as_basic_type_enum()], false),
                            );
                            let res = module.builder.build_call(
                                func,
                                &[arg.into()],
                                &res_vals[0].to_string(),
                            );
                            res.try_as_basic_value().unwrap_left()
                            */
                            module
                                .builder
                                .build_float_to_unsigned_int(
                                    arg,
                                    translate_int_ty(&module.context, ret_ty),
                                    &res_vals[0].to_string(),
                                )
                                .as_basic_value_enum()
                        }
                        Opcode::FcvtToSintSat => {
                            // FIXME too old LLVM installed locally
                            /*
                            let func = module.get_intrinsic(
                                format!("llvm.fptosi.sat.i{}.f{}", ret_ty.bits(), arg_ty.bits(),),
                                translate_int_ty(&module.context, ret_ty)
                                    .fn_type(&[arg.get_type().as_basic_type_enum()], false),
                            );
                            let res = module.builder.build_call(
                                func,
                                &[arg.into()],
                                &res_vals[0].to_string(),
                            );
                            res.try_as_basic_value().unwrap_left()
                            */
                            module
                                .builder
                                .build_float_to_signed_int(
                                    arg,
                                    translate_int_ty(&module.context, ret_ty),
                                    &res_vals[0].to_string(),
                                )
                                .as_basic_value_enum()
                        }
                        _ => unreachable!(),
                    };
                    def_val!(res_vals[0], res);
                }
                InstructionData::UnaryImm { opcode: Opcode::Iconst, imm } => {
                    let imm = translate_imm64(module.context, func.dfg.ctrl_typevar(inst), *imm);
                    def_val!(res_vals[0], imm.as_basic_value_enum());
                }
                InstructionData::UnaryIeee32 { opcode: Opcode::F32const, imm } => {
                    let imm =
                        module.context.f32_type().const_float(f32::from_bits(imm.bits()).into());
                    def_val!(res_vals[0], imm.as_basic_value_enum());
                }
                InstructionData::UnaryIeee64 { opcode: Opcode::F64const, imm } => {
                    let imm = module.context.f64_type().const_float(f64::from_bits(imm.bits()));
                    def_val!(res_vals[0], imm.as_basic_value_enum());
                }
                InstructionData::Binary {
                    opcode:
                        opcode @ Opcode::Iadd
                        | opcode @ Opcode::Isub
                        | opcode @ Opcode::Imul
                        | opcode @ Opcode::Umulhi
                        | opcode @ Opcode::Smulhi
                        | opcode @ Opcode::Udiv
                        | opcode @ Opcode::Sdiv
                        | opcode @ Opcode::Urem
                        | opcode @ Opcode::Srem
                        | opcode @ Opcode::Ishl
                        | opcode @ Opcode::Ushr
                        | opcode @ Opcode::Sshr
                        | opcode @ Opcode::Rotl
                        | opcode @ Opcode::Rotr
                        | opcode @ Opcode::Band
                        | opcode @ Opcode::Bor
                        | opcode @ Opcode::Bxor
                        | opcode @ Opcode::Iconcat,
                    args: [lhs, rhs],
                } => {
                    let lhs = use_val!(*lhs).into_int_value();
                    let rhs = use_val!(*rhs).into_int_value();
                    let res = match opcode {
                        Opcode::Iadd => {
                            module.builder.build_int_add(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::Isub => {
                            module.builder.build_int_sub(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::Imul => {
                            module.builder.build_int_mul(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::Umulhi => {
                            assert!(func.dfg.ctrl_typevar(inst) == types::I64);
                            let lhs = module.builder.build_int_z_extend(
                                lhs,
                                module.context.i128_type(),
                                "umulhi_lhs",
                            );
                            let rhs = module.builder.build_int_z_extend(
                                rhs,
                                module.context.i128_type(),
                                "umulhi_rhs",
                            );
                            let val = module.builder.build_int_mul(
                                lhs,
                                rhs,
                                &format!("{}_mul", res_vals[0]),
                            );
                            let msb_untruncated = module.builder.build_right_shift(
                                val,
                                module.context.i128_type().const_int(64, false),
                                false,
                                &format!("{}_shifted", res_vals[0]),
                            );
                            module.builder.build_int_truncate(
                                msb_untruncated,
                                module.context.i64_type(),
                                &res_vals[0].to_string(),
                            )
                        }
                        Opcode::Smulhi => {
                            assert!(func.dfg.ctrl_typevar(inst) == types::I64);
                            let lhs = module.builder.build_int_s_extend(
                                lhs,
                                module.context.i128_type(),
                                "umulhi_lhs",
                            );
                            let rhs = module.builder.build_int_s_extend(
                                rhs,
                                module.context.i128_type(),
                                "umulhi_rhs",
                            );
                            let val = module.builder.build_int_mul(
                                lhs,
                                rhs,
                                &format!("{}_mul", res_vals[0]),
                            );
                            let msb_untruncated = module.builder.build_right_shift(
                                val,
                                module.context.i128_type().const_int(64, false),
                                false,
                                &format!("{}_shifted", res_vals[0]),
                            );
                            module.builder.build_int_truncate(
                                msb_untruncated,
                                module.context.i64_type(),
                                &res_vals[0].to_string(),
                            )
                        }
                        Opcode::Udiv => module.builder.build_int_unsigned_div(
                            lhs,
                            rhs,
                            &res_vals[0].to_string(),
                        ),
                        Opcode::Sdiv => {
                            module.builder.build_int_signed_div(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::Urem => module.builder.build_int_unsigned_rem(
                            lhs,
                            rhs,
                            &res_vals[0].to_string(),
                        ),
                        Opcode::Srem => {
                            module.builder.build_int_signed_rem(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::Ishl => {
                            let rhs = module.builder.build_int_cast(rhs, lhs.get_type(), "amt");
                            module.builder.build_left_shift(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::Ushr => {
                            let rhs = module.builder.build_int_cast(rhs, lhs.get_type(), "amt");
                            module.builder.build_right_shift(
                                lhs,
                                rhs,
                                false,
                                &res_vals[0].to_string(),
                            )
                        }
                        Opcode::Sshr => {
                            let rhs = module.builder.build_int_cast(rhs, lhs.get_type(), "amt");
                            module.builder.build_right_shift(
                                lhs,
                                rhs,
                                true,
                                &res_vals[0].to_string(),
                            )
                        }
                        Opcode::Rotl => {
                            let rhs = module.builder.build_int_cast(rhs, lhs.get_type(), "amt");
                            let func = module.get_intrinsic(
                                format!("llvm.fshl.i{}", func.dfg.ctrl_typevar(inst).bits()),
                                lhs.get_type().fn_type(
                                    &[
                                        lhs.get_type().as_basic_type_enum(),
                                        lhs.get_type().as_basic_type_enum(),
                                        lhs.get_type().as_basic_type_enum(),
                                    ],
                                    false,
                                ),
                            );
                            let res = module.builder.build_call(
                                func,
                                &[lhs.into(), lhs.into(), rhs.into()],
                                &res_vals[0].to_string(),
                            );
                            res.try_as_basic_value().unwrap_left().into_int_value()
                        }
                        Opcode::Rotr => {
                            let rhs = module.builder.build_int_cast(rhs, lhs.get_type(), "amt");
                            let func = module.get_intrinsic(
                                format!("llvm.fshr.i{}", func.dfg.ctrl_typevar(inst).bits()),
                                lhs.get_type().fn_type(
                                    &[
                                        lhs.get_type().as_basic_type_enum(),
                                        lhs.get_type().as_basic_type_enum(),
                                        lhs.get_type().as_basic_type_enum(),
                                    ],
                                    false,
                                ),
                            );
                            let res = module.builder.build_call(
                                func,
                                &[lhs.into(), lhs.into(), rhs.into()],
                                &res_vals[0].to_string(),
                            );
                            res.try_as_basic_value().unwrap_left().into_int_value()
                        }
                        Opcode::Band => {
                            module.builder.build_and(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::Bor => module.builder.build_or(lhs, rhs, &res_vals[0].to_string()),
                        Opcode::Bxor => {
                            module.builder.build_xor(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::Iconcat => {
                            assert!(func.dfg.ctrl_typevar(inst) == types::I64);
                            let lsb = module.builder.build_int_z_extend(
                                lhs,
                                module.context.i128_type(),
                                "lsb",
                            );
                            let msb_unshifted = module.builder.build_int_z_extend(
                                rhs,
                                module.context.i128_type(),
                                "msb_unshifted",
                            );
                            let msb_shifted = module.builder.build_left_shift(
                                msb_unshifted,
                                module.context.i128_type().const_int(64, false),
                                "msb_shifted",
                            );
                            module.builder.build_or(lsb, msb_shifted, &res_vals[0].to_string())
                        }
                        _ => unreachable!(),
                    };
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }
                InstructionData::BinaryImm8 { opcode: opcode @ Opcode::Extractlane, arg, imm } => {
                    let lhs = use_val!(*arg);
                    let res = match opcode {
                        Opcode::Extractlane => module.builder.build_extract_element(
                            lhs.into_vector_value(),
                            module.context.i8_type().const_int((*imm).into(), false),
                            &res_vals[0].to_string(),
                        ),
                        _ => unreachable!(),
                    };
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }
                InstructionData::BinaryImm64 {
                    opcode:
                        opcode @ Opcode::IaddImm
                        | opcode @ Opcode::IrsubImm
                        | opcode @ Opcode::ImulImm
                        | opcode @ Opcode::UdivImm
                        | opcode @ Opcode::SdivImm
                        | opcode @ Opcode::UremImm
                        | opcode @ Opcode::SremImm
                        | opcode @ Opcode::IshlImm
                        | opcode @ Opcode::UshrImm
                        | opcode @ Opcode::SshrImm
                        | opcode @ Opcode::BandImm
                        | opcode @ Opcode::BorImm
                        | opcode @ Opcode::BxorImm,
                    arg,
                    imm,
                } => {
                    let lhs = use_val!(*arg).into_int_value();
                    let rhs = translate_imm64(module.context, func.dfg.ctrl_typevar(inst), *imm);
                    let res = match opcode {
                        Opcode::IaddImm => {
                            module.builder.build_int_add(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::IrsubImm => {
                            // Note: lhs and rhs are swapped
                            module.builder.build_int_sub(rhs, lhs, &res_vals[0].to_string())
                        }
                        Opcode::ImulImm => {
                            module.builder.build_int_mul(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::UdivImm => module.builder.build_int_unsigned_div(
                            lhs,
                            rhs,
                            &res_vals[0].to_string(),
                        ),
                        Opcode::SdivImm => {
                            module.builder.build_int_signed_div(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::UremImm => module.builder.build_int_unsigned_rem(
                            lhs,
                            rhs,
                            &res_vals[0].to_string(),
                        ),
                        Opcode::SremImm => {
                            module.builder.build_int_signed_rem(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::IshlImm => {
                            module.builder.build_left_shift(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::UshrImm => module.builder.build_right_shift(
                            lhs,
                            rhs,
                            false,
                            &res_vals[0].to_string(),
                        ),
                        Opcode::SshrImm => module.builder.build_right_shift(
                            lhs,
                            rhs,
                            true,
                            &res_vals[0].to_string(),
                        ),
                        Opcode::BandImm => {
                            module.builder.build_and(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::BorImm => {
                            module.builder.build_or(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::BxorImm => {
                            module.builder.build_xor(lhs, rhs, &res_vals[0].to_string())
                        }
                        _ => unreachable!(),
                    };
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }
                InstructionData::Binary {
                    opcode:
                        opcode @ Opcode::Fadd
                        | opcode @ Opcode::Fsub
                        | opcode @ Opcode::Fmul
                        | opcode @ Opcode::Fdiv,
                    args: [lhs, rhs],
                } => {
                    let lhs = use_val!(*lhs).into_float_value();
                    let rhs = use_val!(*rhs).into_float_value();
                    let res = match opcode {
                        Opcode::Fadd => {
                            module.builder.build_float_add(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::Fsub => {
                            module.builder.build_float_sub(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::Fmul => {
                            module.builder.build_float_mul(lhs, rhs, &res_vals[0].to_string())
                        }
                        Opcode::Fdiv => {
                            module.builder.build_float_div(lhs, rhs, &res_vals[0].to_string())
                        }
                        _ => unreachable!(),
                    };
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }
                InstructionData::Ternary {
                    opcode: opcode @ Opcode::Select,
                    args: [cond, lhs, rhs],
                } => {
                    let cond = use_val!(*cond).into_int_value();
                    let lhs = use_val!(*lhs);
                    let rhs = use_val!(*rhs);
                    let res = match opcode {
                        Opcode::Select => {
                            let cond = module.builder.build_int_truncate(
                                cond,
                                module.context.bool_type(),
                                "cond",
                            );
                            module.builder.build_select(cond, lhs, rhs, &res_vals[0].to_string())
                        }
                        _ => unreachable!(),
                    };
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }
                InstructionData::TernaryImm8 {
                    opcode: opcode @ Opcode::Insertlane,
                    args: [lhs, rhs],
                    imm,
                } => {
                    let lhs = use_val!(*lhs);
                    let rhs = use_val!(*rhs);
                    let res = match opcode {
                        Opcode::Insertlane => module.builder.build_insert_element(
                            lhs.into_vector_value(),
                            rhs,
                            module.context.i8_type().const_int((*imm).into(), false),
                            &res_vals[0].to_string(),
                        ),
                        _ => unreachable!(),
                    };
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }
                InstructionData::IntCompare { opcode: Opcode::Icmp, args: [lhs, rhs], cond } => {
                    let lhs = use_val!(*lhs).into_int_value();
                    let rhs = use_val!(*rhs).into_int_value();
                    let res = module.builder.build_int_compare(
                        match cond {
                            IntCC::Equal => IntPredicate::EQ,
                            IntCC::NotEqual => IntPredicate::NE,
                            IntCC::SignedLessThan => IntPredicate::SLT,
                            IntCC::SignedGreaterThanOrEqual => IntPredicate::SGE,
                            IntCC::SignedGreaterThan => IntPredicate::SGT,
                            IntCC::SignedLessThanOrEqual => IntPredicate::SLE,
                            IntCC::UnsignedLessThan => IntPredicate::ULT,
                            IntCC::UnsignedGreaterThanOrEqual => IntPredicate::UGE,
                            IntCC::UnsignedGreaterThan => IntPredicate::UGT,
                            IntCC::UnsignedLessThanOrEqual => IntPredicate::ULE,
                            IntCC::Overflow => todo!(),
                            IntCC::NotOverflow => todo!(),
                        },
                        lhs,
                        rhs,
                        &res_vals[0].to_string(),
                    );
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }
                InstructionData::IntCompareImm { opcode: Opcode::IcmpImm, arg, cond, imm } => {
                    let arg = use_val!(*arg).into_int_value();
                    let imm = translate_imm64(module.context, func.dfg.ctrl_typevar(inst), *imm);
                    let res = module.builder.build_int_compare(
                        match cond {
                            IntCC::Equal => IntPredicate::EQ,
                            IntCC::NotEqual => IntPredicate::NE,
                            IntCC::SignedLessThan => IntPredicate::SLT,
                            IntCC::SignedGreaterThanOrEqual => IntPredicate::SGE,
                            IntCC::SignedGreaterThan => IntPredicate::SGT,
                            IntCC::SignedLessThanOrEqual => IntPredicate::SLE,
                            IntCC::UnsignedLessThan => IntPredicate::ULT,
                            IntCC::UnsignedGreaterThanOrEqual => IntPredicate::UGE,
                            IntCC::UnsignedGreaterThan => IntPredicate::UGT,
                            IntCC::UnsignedLessThanOrEqual => IntPredicate::ULE,
                            IntCC::Overflow => todo!(),
                            IntCC::NotOverflow => todo!(),
                        },
                        arg,
                        imm,
                        &res_vals[0].to_string(),
                    );
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }
                InstructionData::FloatCompare { opcode: Opcode::Fcmp, args: [lhs, rhs], cond } => {
                    let lhs = use_val!(*lhs).into_float_value();
                    let rhs = use_val!(*rhs).into_float_value();
                    let res = module.builder.build_float_compare(
                        match cond {
                            FloatCC::Equal => FloatPredicate::OEQ,
                            FloatCC::NotEqual => FloatPredicate::UNE,
                            FloatCC::Ordered => FloatPredicate::ORD,
                            FloatCC::Unordered => FloatPredicate::UNO,
                            FloatCC::OrderedNotEqual => FloatPredicate::ONE,
                            FloatCC::UnorderedOrEqual => FloatPredicate::UEQ,
                            FloatCC::LessThan => FloatPredicate::OLT,
                            FloatCC::LessThanOrEqual => FloatPredicate::OLE,
                            FloatCC::GreaterThan => FloatPredicate::OGT,
                            FloatCC::GreaterThanOrEqual => FloatPredicate::OGE,
                            FloatCC::UnorderedOrLessThan => FloatPredicate::ULT,
                            FloatCC::UnorderedOrLessThanOrEqual => FloatPredicate::ULE,
                            FloatCC::UnorderedOrGreaterThan => FloatPredicate::UGT,
                            FloatCC::UnorderedOrGreaterThanOrEqual => FloatPredicate::UGE,
                        },
                        lhs,
                        rhs,
                        &res_vals[0].to_string(),
                    );
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }

                InstructionData::Load { opcode: Opcode::Load, arg, flags, offset } => {
                    let arg = use_val!(*arg).into_int_value();
                    let ptr = translate_ptr_offset32(
                        module.context,
                        &module.builder,
                        func.dfg.ctrl_typevar(inst),
                        arg,
                        *offset,
                    );
                    let res = module.builder.build_load(ptr, &res_vals[0].to_string());
                    if flags.aligned() {
                        res.as_instruction_value()
                            .unwrap()
                            .set_alignment(func.dfg.ctrl_typevar(inst).bytes())
                            .unwrap();
                    } else {
                        res.as_instruction_value().unwrap().set_alignment(1).unwrap();
                    }
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }

                InstructionData::Store {
                    opcode: Opcode::Store,
                    args: [arg, ptr],
                    flags,
                    offset,
                } => {
                    let arg = use_val!(*arg);
                    let ptr = use_val!(*ptr).into_int_value();
                    let ptr = translate_ptr_offset32(
                        module.context,
                        &module.builder,
                        func.dfg.ctrl_typevar(inst),
                        ptr,
                        *offset,
                    );
                    let inst_val = module.builder.build_store(ptr, arg);
                    if flags.aligned() {
                        inst_val.set_alignment(func.dfg.ctrl_typevar(inst).bytes()).unwrap();
                    } else {
                        inst_val.set_alignment(1).unwrap();
                    }
                }

                InstructionData::StackLoad { opcode: Opcode::StackLoad, stack_slot, offset } => {
                    let ptr = translate_ptr_offset32(
                        module.context,
                        &module.builder,
                        func.dfg.ctrl_typevar(inst),
                        stack_slot_map[stack_slot],
                        *offset,
                    );
                    let res = module.builder.build_load(ptr, &res_vals[0].to_string());
                    res.as_instruction_value().unwrap().set_alignment(1).unwrap(); // FIXME determine and set actual alignment
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }

                InstructionData::StackLoad { opcode: Opcode::StackAddr, stack_slot, offset } => {
                    let ptr = stack_slot_map[stack_slot];
                    let ptr_ty = ptr.get_type();
                    let offset: i64 = (*offset).into();
                    let offset =
                        ptr_ty.const_int(offset as u64, false /* FIXME right value? */);
                    let ptr = module.builder.build_int_add(ptr, offset, "ptr_val");
                    def_val!(res_vals[0], ptr.as_basic_value_enum());
                }

                InstructionData::StackStore {
                    opcode: Opcode::StackStore,
                    arg,
                    stack_slot,
                    offset,
                } => {
                    let arg = use_val!(*arg);
                    let ptr = translate_ptr_offset32(
                        module.context,
                        &module.builder,
                        func.dfg.ctrl_typevar(inst),
                        stack_slot_map[stack_slot],
                        *offset,
                    );
                    let inst_val = module.builder.build_store(ptr, arg);
                    inst_val.set_alignment(1).unwrap(); // FIXME determine and set actual alignment
                }

                InstructionData::LoadNoOffset { opcode: Opcode::AtomicLoad, arg, flags } => {
                    let arg = use_val!(*arg).into_int_value();
                    let ptr = translate_ptr_no_offset(
                        module.context,
                        &module.builder,
                        func.dfg.ctrl_typevar(inst),
                        arg,
                    );
                    let res = module.builder.build_load(ptr, &res_vals[0].to_string());
                    res.as_instruction_value()
                        .unwrap()
                        .set_atomic_ordering(AtomicOrdering::SequentiallyConsistent)
                        .unwrap();
                    if flags.aligned() {
                        res.as_instruction_value()
                            .unwrap()
                            .set_alignment(func.dfg.ctrl_typevar(inst).bytes())
                            .unwrap();
                    } else {
                        res.as_instruction_value().unwrap().set_alignment(1).unwrap();
                    }
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }

                InstructionData::StoreNoOffset {
                    opcode: Opcode::AtomicStore,
                    args: [arg, ptr],
                    flags,
                } => {
                    let arg = use_val!(*arg);
                    let ptr = use_val!(*ptr).into_int_value();
                    let ptr = translate_ptr_no_offset(
                        module.context,
                        &module.builder,
                        func.dfg.ctrl_typevar(inst),
                        ptr,
                    );
                    let inst_val = module.builder.build_store(ptr, arg);
                    inst_val.set_atomic_ordering(AtomicOrdering::SequentiallyConsistent).unwrap();
                    if flags.aligned() {
                        inst_val.set_alignment(func.dfg.ctrl_typevar(inst).bytes()).unwrap();
                    } else {
                        inst_val.set_alignment(1).unwrap();
                    }
                }

                InstructionData::AtomicRmw {
                    opcode: Opcode::AtomicRmw,
                    args: [ptr, val],
                    flags: _,
                    op,
                } => {
                    let ptr = use_val!(*ptr).into_int_value();
                    let val = use_val!(*val).into_int_value();
                    let ptr = translate_ptr_no_offset(
                        module.context,
                        &module.builder,
                        func.dfg.ctrl_typevar(inst),
                        ptr,
                    );
                    let res = module
                        .builder
                        .build_atomicrmw(
                            match op {
                                AtomicRmwOp::Add => AtomicRMWBinOp::Add,
                                AtomicRmwOp::Sub => AtomicRMWBinOp::Sub,
                                AtomicRmwOp::And => AtomicRMWBinOp::And,
                                AtomicRmwOp::Nand => AtomicRMWBinOp::Nand,
                                AtomicRmwOp::Or => AtomicRMWBinOp::Or,
                                AtomicRmwOp::Xor => AtomicRMWBinOp::Xor,
                                AtomicRmwOp::Xchg => AtomicRMWBinOp::Xchg,
                                AtomicRmwOp::Umin => AtomicRMWBinOp::UMin,
                                AtomicRmwOp::Umax => AtomicRMWBinOp::UMax,
                                AtomicRmwOp::Smin => AtomicRMWBinOp::Min,
                                AtomicRmwOp::Smax => AtomicRMWBinOp::Max,
                            },
                            ptr,
                            val,
                            AtomicOrdering::SequentiallyConsistent,
                        )
                        .unwrap();
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }

                InstructionData::AtomicCas {
                    opcode: Opcode::AtomicCas,
                    args: [ptr, test_old, new],
                    flags: _,
                } => {
                    let ptr = use_val!(*ptr).into_int_value();
                    let test_old = use_val!(*test_old);
                    let new = use_val!(*new);
                    let ptr = translate_ptr_no_offset(
                        module.context,
                        &module.builder,
                        func.dfg.ctrl_typevar(inst),
                        ptr,
                    );
                    let res_struct = module
                        .builder
                        .build_cmpxchg(
                            ptr,
                            test_old,
                            new,
                            AtomicOrdering::SequentiallyConsistent,
                            AtomicOrdering::SequentiallyConsistent,
                        )
                        .unwrap();
                    let res = module
                        .builder
                        .build_extract_value(res_struct, 0, &res_vals[0].to_string())
                        .unwrap();
                    def_val!(res_vals[0], res);
                }

                InstructionData::UnaryConst { opcode: Opcode::Vconst, constant_handle } => {
                    let vec_ty = translate_vector_ty(module.context, func.dfg.ctrl_typevar(inst));
                    let constant = func.dfg.constants.get(*constant_handle);
                    let const_vec = module.context.i8_type().const_array(
                        &constant
                            .as_slice()
                            .iter()
                            .map(|&byte| module.context.i8_type().const_int(byte.into(), false))
                            .collect::<Vec<_>>(),
                    );
                    let res =
                        module.builder.build_bitcast(const_vec, vec_ty, &res_vals[0].to_string());
                    def_val!(res_vals[0], res.as_basic_value_enum());
                }

                InstructionData::UnaryGlobalValue {
                    opcode: Opcode::SymbolValue | Opcode::TlsValue,
                    global_value,
                } => {
                    let ptr_ty = module.context.i64_type(); // FIXME
                    let data_id =
                        DataId::from_name(&func.global_values[*global_value].symbol_name());
                    def_val!(
                        res_vals[0],
                        module.data_object_refs[&data_id]
                            .as_pointer_value()
                            .const_to_int(ptr_ty)
                            .into(),
                    );
                }

                InstructionData::FuncAddr { opcode: Opcode::FuncAddr, func_ref } => {
                    let ptr_ty = module.context.i64_type(); // FIXME
                    let func_val = module.get_func(&func.dfg.ext_funcs[*func_ref].name);

                    def_val!(
                        res_vals[0],
                        func_val.as_global_value().as_pointer_value().const_to_int(ptr_ty).into(),
                    );
                }

                InstructionData::Call { opcode: Opcode::Call, args, func_ref } => {
                    let args = args
                        .as_slice(&func.dfg.value_lists)
                        .iter()
                        .map(|arg| use_val!(*arg))
                        .collect::<Vec<_>>();

                    let func_val = module.get_func(&func.dfg.ext_funcs[*func_ref].name);

                    let res = module.builder.build_call(
                        func_val,
                        &args,
                        &res_vals.get(0).map(|val| val.to_string()).unwrap_or_else(String::new),
                    );

                    match res_vals {
                        [] => {}
                        [res_val] => {
                            def_val!(*res_val, res.try_as_basic_value().unwrap_left());
                        }
                        [res_val_a, res_val_b] => {
                            let res = res.as_any_value_enum().into_struct_value();
                            let res_a = module
                                .builder
                                .build_extract_value(res, 0, &format!("{}", res_val_a))
                                .unwrap();
                            let res_b = module
                                .builder
                                .build_extract_value(res, 1, &format!("{}", res_val_b))
                                .unwrap();
                            def_val!(*res_val_a, res_a);
                            def_val!(*res_val_b, res_b);
                        }
                        _ => todo!(),
                    }
                }

                InstructionData::CallIndirect { opcode: Opcode::CallIndirect, args, sig_ref } => {
                    let args = args
                        .as_slice(&func.dfg.value_lists)
                        .iter()
                        .map(|arg| use_val!(*arg))
                        .collect::<Vec<_>>();

                    let func_type =
                        translate_sig(&module.context, &func.dfg.signatures[*sig_ref], false)
                            .ptr_type(AddressSpace::Generic);
                    let func_val = module.builder.build_int_to_ptr(
                        args[0].into_int_value(),
                        func_type,
                        "fnptr",
                    );
                    let func_val = CallableValue::try_from(func_val).unwrap();

                    let res = module.builder.build_call(
                        func_val,
                        &args[1..],
                        &res_vals.get(0).map(|val| val.to_string()).unwrap_or_else(String::new),
                    );

                    match res_vals {
                        [] => {}
                        [res_val] => {
                            def_val!(*res_val, res.try_as_basic_value().unwrap_left());
                        }
                        [res_val_a, res_val_b] => {
                            let res = res.as_any_value_enum().into_struct_value();
                            let res_a = module
                                .builder
                                .build_extract_value(res, 0, &format!("{}", res_val_a))
                                .unwrap();
                            let res_b = module
                                .builder
                                .build_extract_value(res, 1, &format!("{}", res_val_b))
                                .unwrap();
                            def_val!(*res_val_a, res_a);
                            def_val!(*res_val_b, res_b);
                        }
                        _ => todo!(),
                    }
                }

                InstructionData::Branch {
                    opcode: opcode @ Opcode::Brz | opcode @ Opcode::Brnz,
                    args,
                    destination: then_block,
                } => {
                    let args = args.as_slice(&func.dfg.value_lists);
                    let conditional = use_val!(args[0]).into_int_value();
                    let conditional = module.builder.build_int_compare(
                        if *opcode == Opcode::Brz { IntPredicate::EQ } else { IntPredicate::NE },
                        conditional,
                        conditional.get_type().const_zero(),
                        if *opcode == Opcode::Brz { "brz" } else { "brnz" },
                    );
                    let then_args = &args[1..];
                    for (arg, phi) in then_args.iter().zip(&phi_map[then_block]) {
                        phi.add_incoming(&[(
                            &use_val!(*arg) as _,
                            module.builder.get_insert_block().unwrap(),
                        )]);
                    }
                    let (else_block, else_args) =
                        match &func.dfg[func.layout.next_inst(inst).unwrap()] {
                            InstructionData::Jump {
                                opcode: Opcode::Jump,
                                args: else_args,
                                destination: else_block,
                            } => (else_block, else_args.as_slice(&func.dfg.value_lists)),
                            _ => unreachable!(),
                        };
                    for (arg, phi) in else_args.iter().zip(&phi_map[&else_block]) {
                        phi.add_incoming(&[(
                            &use_val!(*arg) as _,
                            module.builder.get_insert_block().unwrap(),
                        )]);
                    }
                    module.builder.build_conditional_branch(
                        conditional,
                        block_map[then_block],
                        block_map[&else_block],
                    );
                    break; // Don't codegen the following jump
                }

                InstructionData::Jump { opcode: Opcode::Jump, args, destination } => {
                    for (arg, phi) in
                        args.as_slice(&func.dfg.value_lists).iter().zip(&phi_map[destination])
                    {
                        phi.add_incoming(&[(
                            &use_val!(*arg) as _,
                            module.builder.get_insert_block().unwrap(),
                        )]);
                    }
                    module.builder.build_unconditional_branch(block_map[destination]);
                }

                InstructionData::BranchTable {
                    opcode: Opcode::BrTable,
                    arg,
                    destination,
                    table,
                } => {
                    let cond = use_val!(*arg).into_int_value();
                    module.builder.build_switch(
                        cond,
                        block_map[destination],
                        &func.jump_tables[*table]
                            .as_slice()
                            .iter()
                            .enumerate()
                            .map(|(i, block)| {
                                (cond.get_type().const_int(i as u64, false), block_map[block])
                            })
                            .collect::<Vec<_>>(),
                    );
                }

                InstructionData::MultiAry { opcode: Opcode::Return, args } => {
                    match args.as_slice(&func.dfg.value_lists) {
                        [] => {
                            module.builder.build_return(None);
                        }
                        [ret_val] => {
                            module.builder.build_return(Some(&use_val!(*ret_val) as _));
                        }
                        [ret_val_a, ret_val_b] => {
                            let ret_val_a = use_val!(*ret_val_a);
                            let ret_val_b = use_val!(*ret_val_b);
                            let ret_val = module
                                .context
                                .struct_type(&[ret_val_a.get_type(), ret_val_b.get_type()], false)
                                .get_undef();
                            let ret_val = module
                                .builder
                                .build_insert_value(ret_val, ret_val_a, 0, "ret_val")
                                .unwrap();
                            let ret_val = module
                                .builder
                                .build_insert_value(ret_val, ret_val_b, 1, "ret_val")
                                .unwrap();
                            module.builder.build_return(Some(&ret_val));
                        }
                        _ => todo!(),
                    }
                }
                InstructionData::Trap { opcode: Opcode::Trap, code } => {
                    let trap = module.get_intrinsic(
                        "llvm.trap".to_owned(),
                        module.context.void_type().fn_type(&[], false),
                    );
                    module.builder.build_call(trap, &[], &format!("trap {}", code));
                    module.builder.build_unreachable();
                }
                inst => {
                    panic!("[{}] {:?}", block, inst);
                }
            }
        }
    }

    if !func_val.verify(true) {
        panic!();
    }

    Ok(())
}
