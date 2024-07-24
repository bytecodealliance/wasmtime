//! This module is the central place for machine code emission.
//! It defines an implementation of wasmparser's Visitor trait
//! for `CodeGen`; which defines a visitor per op-code,
//! which validates and dispatches to the corresponding
//! machine code emitter.

use crate::abi::RetArea;
use crate::codegen::{control_index, Callee, CodeGen, ControlStackFrame, FnCall};
use crate::masm::{
    DivKind, ExtendKind, FloatCmpKind, IntCmpKind, MacroAssembler, MemMoveDirection, OperandSize,
    RegImm, RemKind, RoundingMode, SPOffset, ShiftKind, TruncKind,
};
use crate::reg::Reg;
use crate::stack::{TypedReg, Val};
use cranelift_codegen::ir::TrapCode;
use regalloc2::RegClass;
use smallvec::SmallVec;
use wasmparser::{BlockType, BrTable, Ieee32, Ieee64, MemArg, VisitOperator, V128};
use wasmtime_environ::{
    FuncIndex, GlobalIndex, MemoryIndex, TableIndex, TableStyle, TypeIndex, WasmHeapType,
    WasmValType, FUNCREF_INIT_BIT,
};

/// A macro to define unsupported WebAssembly operators.
///
/// This macro calls itself recursively;
/// 1. It no-ops when matching a supported operator.
/// 2. Defines the visitor function and panics when
/// matching an unsupported operator.
macro_rules! def_unsupported {
    ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })? => $visit:ident)*) => {
        $(
            def_unsupported!(
                emit
                    $op

                fn $visit(&mut self $($(,$arg: $argty)*)?) -> Self::Output {
                    $($(let _ = $arg;)*)?
                        todo!(stringify!($op))
                }
            );
        )*
    };

    (emit I32Const $($rest:tt)*) => {};
    (emit I64Const $($rest:tt)*) => {};
    (emit F32Const $($rest:tt)*) => {};
    (emit F64Const $($rest:tt)*) => {};
    (emit V128Const $($rest:tt)*) => {};
    (emit F32Add $($rest:tt)*) => {};
    (emit F64Add $($rest:tt)*) => {};
    (emit F32Sub $($rest:tt)*) => {};
    (emit F64Sub $($rest:tt)*) => {};
    (emit F32Mul $($rest:tt)*) => {};
    (emit F64Mul $($rest:tt)*) => {};
    (emit F32Div $($rest:tt)*) => {};
    (emit F64Div $($rest:tt)*) => {};
    (emit F32Min $($rest:tt)*) => {};
    (emit F64Min $($rest:tt)*) => {};
    (emit F32Max $($rest:tt)*) => {};
    (emit F64Max $($rest:tt)*) => {};
    (emit F32Copysign $($rest:tt)*) => {};
    (emit F64Copysign $($rest:tt)*) => {};
    (emit F32Abs $($rest:tt)*) => {};
    (emit F64Abs $($rest:tt)*) => {};
    (emit F32Neg $($rest:tt)*) => {};
    (emit F64Neg $($rest:tt)*) => {};
    (emit F32Floor $($rest:tt)*) => {};
    (emit F64Floor $($rest:tt)*) => {};
    (emit F32Ceil $($rest:tt)*) => {};
    (emit F64Ceil $($rest:tt)*) => {};
    (emit F32Nearest $($rest:tt)*) => {};
    (emit F64Nearest $($rest:tt)*) => {};
    (emit F32Trunc $($rest:tt)*) => {};
    (emit F64Trunc $($rest:tt)*) => {};
    (emit F32Sqrt $($rest:tt)*) => {};
    (emit F64Sqrt $($rest:tt)*) => {};
    (emit F32Eq $($rest:tt)*) => {};
    (emit F64Eq $($rest:tt)*) => {};
    (emit F32Ne $($rest:tt)*) => {};
    (emit F64Ne $($rest:tt)*) => {};
    (emit F32Lt $($rest:tt)*) => {};
    (emit F64Lt $($rest:tt)*) => {};
    (emit F32Gt $($rest:tt)*) => {};
    (emit F64Gt $($rest:tt)*) => {};
    (emit F32Le $($rest:tt)*) => {};
    (emit F64Le $($rest:tt)*) => {};
    (emit F32Ge $($rest:tt)*) => {};
    (emit F64Ge $($rest:tt)*) => {};
    (emit F32ConvertI32S $($rest:tt)*) => {};
    (emit F32ConvertI32U $($rest:tt)*) => {};
    (emit F32ConvertI64S $($rest:tt)*) => {};
    (emit F32ConvertI64U $($rest:tt)*) => {};
    (emit F64ConvertI32S $($rest:tt)*) => {};
    (emit F64ConvertI32U $($rest:tt)*) => {};
    (emit F64ConvertI64S $($rest:tt)*) => {};
    (emit F64ConvertI64U $($rest:tt)*) => {};
    (emit F32ReinterpretI32 $($rest:tt)*) => {};
    (emit F64ReinterpretI64 $($rest:tt)*) => {};
    (emit F32DemoteF64 $($rest:tt)*) => {};
    (emit F64PromoteF32 $($rest:tt)*) => {};
    (emit I32Add $($rest:tt)*) => {};
    (emit I64Add $($rest:tt)*) => {};
    (emit I32Sub $($rest:tt)*) => {};
    (emit I32Mul $($rest:tt)*) => {};
    (emit I32DivS $($rest:tt)*) => {};
    (emit I32DivU $($rest:tt)*) => {};
    (emit I64DivS $($rest:tt)*) => {};
    (emit I64DivU $($rest:tt)*) => {};
    (emit I64RemU $($rest:tt)*) => {};
    (emit I64RemS $($rest:tt)*) => {};
    (emit I32RemU $($rest:tt)*) => {};
    (emit I32RemS $($rest:tt)*) => {};
    (emit I64Mul $($rest:tt)*) => {};
    (emit I64Sub $($rest:tt)*) => {};
    (emit I32Eq $($rest:tt)*) => {};
    (emit I64Eq $($rest:tt)*) => {};
    (emit I32Ne $($rest:tt)*) => {};
    (emit I64Ne $($rest:tt)*) => {};
    (emit I32LtS $($rest:tt)*) => {};
    (emit I64LtS $($rest:tt)*) => {};
    (emit I32LtU $($rest:tt)*) => {};
    (emit I64LtU $($rest:tt)*) => {};
    (emit I32LeS $($rest:tt)*) => {};
    (emit I64LeS $($rest:tt)*) => {};
    (emit I32LeU $($rest:tt)*) => {};
    (emit I64LeU $($rest:tt)*) => {};
    (emit I32GtS $($rest:tt)*) => {};
    (emit I64GtS $($rest:tt)*) => {};
    (emit I32GtU $($rest:tt)*) => {};
    (emit I64GtU $($rest:tt)*) => {};
    (emit I32GeS $($rest:tt)*) => {};
    (emit I64GeS $($rest:tt)*) => {};
    (emit I32GeU $($rest:tt)*) => {};
    (emit I64GeU $($rest:tt)*) => {};
    (emit I32Eqz $($rest:tt)*) => {};
    (emit I64Eqz $($rest:tt)*) => {};
    (emit I32And $($rest:tt)*) => {};
    (emit I64And $($rest:tt)*) => {};
    (emit I32Or $($rest:tt)*) => {};
    (emit I64Or $($rest:tt)*) => {};
    (emit I32Xor $($rest:tt)*) => {};
    (emit I64Xor $($rest:tt)*) => {};
    (emit I32Shl $($rest:tt)*) => {};
    (emit I64Shl $($rest:tt)*) => {};
    (emit I32ShrS $($rest:tt)*) => {};
    (emit I64ShrS $($rest:tt)*) => {};
    (emit I32ShrU $($rest:tt)*) => {};
    (emit I64ShrU $($rest:tt)*) => {};
    (emit I32Rotl $($rest:tt)*) => {};
    (emit I64Rotl $($rest:tt)*) => {};
    (emit I32Rotr $($rest:tt)*) => {};
    (emit I64Rotr $($rest:tt)*) => {};
    (emit I32Clz $($rest:tt)*) => {};
    (emit I64Clz $($rest:tt)*) => {};
    (emit I32Ctz $($rest:tt)*) => {};
    (emit I64Ctz $($rest:tt)*) => {};
    (emit I32Popcnt $($rest:tt)*) => {};
    (emit I64Popcnt $($rest:tt)*) => {};
    (emit I32WrapI64 $($rest:tt)*) => {};
    (emit I64ExtendI32S $($rest:tt)*) => {};
    (emit I64ExtendI32U $($rest:tt)*) => {};
    (emit I32Extend8S $($rest:tt)*) => {};
    (emit I32Extend16S $($rest:tt)*) => {};
    (emit I64Extend8S $($rest:tt)*) => {};
    (emit I64Extend16S $($rest:tt)*) => {};
    (emit I64Extend32S $($rest:tt)*) => {};
    (emit I32TruncF32S $($rest:tt)*) => {};
    (emit I32TruncF32U $($rest:tt)*) => {};
    (emit I32TruncF64S $($rest:tt)*) => {};
    (emit I32TruncF64U $($rest:tt)*) => {};
    (emit I64TruncF32S $($rest:tt)*) => {};
    (emit I64TruncF32U $($rest:tt)*) => {};
    (emit I64TruncF64S $($rest:tt)*) => {};
    (emit I64TruncF64U $($rest:tt)*) => {};
    (emit I32ReinterpretF32 $($rest:tt)*) => {};
    (emit I64ReinterpretF64 $($rest:tt)*) => {};
    (emit LocalGet $($rest:tt)*) => {};
    (emit LocalSet $($rest:tt)*) => {};
    (emit Call $($rest:tt)*) => {};
    (emit End $($rest:tt)*) => {};
    (emit Nop $($rest:tt)*) => {};
    (emit If $($rest:tt)*) => {};
    (emit Else $($rest:tt)*) => {};
    (emit Block $($rest:tt)*) => {};
    (emit Loop $($rest:tt)*) => {};
    (emit Br $($rest:tt)*) => {};
    (emit BrIf $($rest:tt)*) => {};
    (emit Return $($rest:tt)*) => {};
    (emit Unreachable $($rest:tt)*) => {};
    (emit LocalTee $($rest:tt)*) => {};
    (emit GlobalGet $($rest:tt)*) => {};
    (emit GlobalSet $($rest:tt)*) => {};
    (emit Select $($rest:tt)*) => {};
    (emit Drop $($rest:tt)*) => {};
    (emit BrTable $($rest:tt)*) => {};
    (emit CallIndirect $($rest:tt)*) => {};
    (emit TableInit $($rest:tt)*) => {};
    (emit TableCopy $($rest:tt)*) => {};
    (emit TableGet $($rest:tt)*) => {};
    (emit TableSet $($rest:tt)*) => {};
    (emit TableGrow $($rest:tt)*) => {};
    (emit TableSize $($rest:tt)*) => {};
    (emit TableFill $($rest:tt)*) => {};
    (emit ElemDrop $($rest:tt)*) => {};
    (emit MemoryInit $($rest:tt)*) => {};
    (emit MemoryCopy $($rest:tt)*) => {};
    (emit DataDrop $($rest:tt)*) => {};
    (emit MemoryFill $($rest:tt)*) => {};
    (emit MemorySize $($rest:tt)*) => {};
    (emit MemoryGrow $($rest:tt)*) => {};
    (emit I32Load $($rest:tt)*) => {};
    (emit I32Load8S $($rest:tt)*) => {};
    (emit I32Load8U $($rest:tt)*) => {};
    (emit I32Load16S $($rest:tt)*) => {};
    (emit I32Load16U $($rest:tt)*) => {};
    (emit I64Load8S $($rest:tt)*) => {};
    (emit I64Load8U $($rest:tt)*) => {};
    (emit I64Load16S $($rest:tt)*) => {};
    (emit I64Load16U $($rest:tt)*) => {};
    (emit I64Load32S $($rest:tt)*) => {};
    (emit I64Load32U $($rest:tt)*) => {};
    (emit I64Load $($rest:tt)*) => {};
    (emit I32Store $($rest:tt)*) => {};
    (emit I32Store $($rest:tt)*) => {};
    (emit I32Store8 $($rest:tt)*) => {};
    (emit I32Store16 $($rest:tt)*) => {};
    (emit I64Store $($rest:tt)*) => {};
    (emit I64Store8 $($rest:tt)*) => {};
    (emit I64Store16 $($rest:tt)*) => {};
    (emit I64Store32 $($rest:tt)*) => {};
    (emit F32Load $($rest:tt)*) => {};
    (emit F32Store $($rest:tt)*) => {};
    (emit F64Load $($rest:tt)*) => {};
    (emit F64Store $($rest:tt)*) => {};
    (emit I32TruncSatF32S $($rest:tt)*) => {};
    (emit I32TruncSatF32U $($rest:tt)*) => {};
    (emit I32TruncSatF64S $($rest:tt)*) => {};
    (emit I32TruncSatF64U $($rest:tt)*) => {};
    (emit I64TruncSatF32S $($rest:tt)*) => {};
    (emit I64TruncSatF32U $($rest:tt)*) => {};
    (emit I64TruncSatF64S $($rest:tt)*) => {};
    (emit I64TruncSatF64U $($rest:tt)*) => {};
    (emit V128Load $($rest:tt)*) => {};
    (emit V128Store $($rest:tt)*) => {};

    (emit $unsupported:tt $($rest:tt)*) => {$($rest)*};
}

impl<'a, 'translation, 'data, M> VisitOperator<'a> for CodeGen<'a, 'translation, 'data, M>
where
    M: MacroAssembler,
{
    type Output = ();

    fn visit_i32_const(&mut self, val: i32) {
        self.context.stack.push(Val::i32(val));
    }

    fn visit_i64_const(&mut self, val: i64) {
        self.context.stack.push(Val::i64(val));
    }

    fn visit_f32_const(&mut self, val: Ieee32) {
        self.context.stack.push(Val::f32(val));
    }

    fn visit_f64_const(&mut self, val: Ieee64) {
        self.context.stack.push(Val::f64(val));
    }

    fn visit_v128_const(&mut self, val: V128) {
        self.context.stack.push(Val::v128(val.i128()))
    }

    fn visit_f32_add(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_add(dst, dst, src, size);
                TypedReg::f32(dst)
            },
        );
    }

    fn visit_f64_add(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_add(dst, dst, src, size);
                TypedReg::f64(dst)
            },
        );
    }

    fn visit_f32_sub(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_sub(dst, dst, src, size);
                TypedReg::f32(dst)
            },
        );
    }

    fn visit_f64_sub(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_sub(dst, dst, src, size);
                TypedReg::f64(dst)
            },
        );
    }

    fn visit_f32_mul(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_mul(dst, dst, src, size);
                TypedReg::f32(dst)
            },
        );
    }

    fn visit_f64_mul(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_mul(dst, dst, src, size);
                TypedReg::f64(dst)
            },
        );
    }

    fn visit_f32_div(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_div(dst, dst, src, size);
                TypedReg::f32(dst)
            },
        );
    }

    fn visit_f64_div(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_div(dst, dst, src, size);
                TypedReg::f64(dst)
            },
        );
    }

    fn visit_f32_min(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_min(dst, dst, src, size);
                TypedReg::f32(dst)
            },
        );
    }

    fn visit_f64_min(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_min(dst, dst, src, size);
                TypedReg::f64(dst)
            },
        );
    }

    fn visit_f32_max(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_max(dst, dst, src, size);
                TypedReg::f32(dst)
            },
        );
    }

    fn visit_f64_max(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_max(dst, dst, src, size);
                TypedReg::f64(dst)
            },
        );
    }

    fn visit_f32_copysign(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_copysign(dst, dst, src, size);
                TypedReg::f32(dst)
            },
        );
    }

    fn visit_f64_copysign(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_copysign(dst, dst, src, size);
                TypedReg::f64(dst)
            },
        );
    }

    fn visit_f32_abs(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S32, &mut |masm, reg, size| {
                masm.float_abs(reg, size);
                TypedReg::f32(reg)
            });
    }

    fn visit_f64_abs(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S64, &mut |masm, reg, size| {
                masm.float_abs(reg, size);
                TypedReg::f64(reg)
            });
    }

    fn visit_f32_neg(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S32, &mut |masm, reg, size| {
                masm.float_neg(reg, size);
                TypedReg::f32(reg)
            });
    }

    fn visit_f64_neg(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S64, &mut |masm, reg, size| {
                masm.float_neg(reg, size);
                TypedReg::f64(reg)
            });
    }

    fn visit_f32_floor(&mut self) {
        self.masm.float_round(
            RoundingMode::Down,
            &mut self.env,
            &mut self.context,
            OperandSize::S32,
            |env, cx, masm| {
                let builtin = env.builtins.floor_f32::<M::ABI>();
                FnCall::emit::<M>(env, masm, cx, Callee::Builtin(builtin));
            },
        );
    }

    fn visit_f64_floor(&mut self) {
        self.masm.float_round(
            RoundingMode::Down,
            &mut self.env,
            &mut self.context,
            OperandSize::S64,
            |env, cx, masm| {
                let builtin = env.builtins.floor_f64::<M::ABI>();
                FnCall::emit::<M>(env, masm, cx, Callee::Builtin(builtin));
            },
        );
    }

    fn visit_f32_ceil(&mut self) {
        self.masm.float_round(
            RoundingMode::Up,
            &mut self.env,
            &mut self.context,
            OperandSize::S32,
            |env, cx, masm| {
                let builtin = env.builtins.ceil_f32::<M::ABI>();
                FnCall::emit::<M>(env, masm, cx, Callee::Builtin(builtin));
            },
        );
    }

    fn visit_f64_ceil(&mut self) {
        self.masm.float_round(
            RoundingMode::Up,
            &mut self.env,
            &mut self.context,
            OperandSize::S64,
            |env, cx, masm| {
                let builtin = env.builtins.ceil_f64::<M::ABI>();
                FnCall::emit::<M>(env, masm, cx, Callee::Builtin(builtin));
            },
        );
    }

    fn visit_f32_nearest(&mut self) {
        self.masm.float_round(
            RoundingMode::Nearest,
            &mut self.env,
            &mut self.context,
            OperandSize::S32,
            |env, cx, masm| {
                let builtin = env.builtins.nearest_f32::<M::ABI>();
                FnCall::emit::<M>(env, masm, cx, Callee::Builtin(builtin))
            },
        );
    }

    fn visit_f64_nearest(&mut self) {
        self.masm.float_round(
            RoundingMode::Nearest,
            &mut self.env,
            &mut self.context,
            OperandSize::S64,
            |env, cx, masm| {
                let builtin = env.builtins.nearest_f64::<M::ABI>();
                FnCall::emit::<M>(env, masm, cx, Callee::Builtin(builtin));
            },
        );
    }

    fn visit_f32_trunc(&mut self) {
        self.masm.float_round(
            RoundingMode::Zero,
            &mut self.env,
            &mut self.context,
            OperandSize::S32,
            |env, cx, masm| {
                let builtin = env.builtins.trunc_f32::<M::ABI>();
                FnCall::emit::<M>(env, masm, cx, Callee::Builtin(builtin));
            },
        );
    }

    fn visit_f64_trunc(&mut self) {
        self.masm.float_round(
            RoundingMode::Zero,
            &mut self.env,
            &mut self.context,
            OperandSize::S64,
            |env, cx, masm| {
                let builtin = env.builtins.trunc_f64::<M::ABI>();
                FnCall::emit::<M>(env, masm, cx, Callee::Builtin(builtin));
            },
        );
    }

    fn visit_f32_sqrt(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S32, &mut |masm, reg, size| {
                masm.float_sqrt(reg, reg, size);
                TypedReg::f32(reg)
            });
    }

    fn visit_f64_sqrt(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S64, &mut |masm, reg, size| {
                masm.float_sqrt(reg, reg, size);
                TypedReg::f64(reg)
            });
    }

    fn visit_f32_eq(&mut self) {
        self.context.float_cmp_op(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src1, src2, size| {
                masm.float_cmp_with_set(src1, src2, dst, FloatCmpKind::Eq, size);
            },
        );
    }

    fn visit_f64_eq(&mut self) {
        self.context.float_cmp_op(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src1, src2, size| {
                masm.float_cmp_with_set(src1, src2, dst, FloatCmpKind::Eq, size);
            },
        );
    }

    fn visit_f32_ne(&mut self) {
        self.context.float_cmp_op(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src1, src2, size| {
                masm.float_cmp_with_set(src1, src2, dst, FloatCmpKind::Ne, size);
            },
        );
    }

    fn visit_f64_ne(&mut self) {
        self.context.float_cmp_op(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src1, src2, size| {
                masm.float_cmp_with_set(src1, src2, dst, FloatCmpKind::Ne, size);
            },
        );
    }

    fn visit_f32_lt(&mut self) {
        self.context.float_cmp_op(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src1, src2, size| {
                masm.float_cmp_with_set(src1, src2, dst, FloatCmpKind::Lt, size);
            },
        );
    }

    fn visit_f64_lt(&mut self) {
        self.context.float_cmp_op(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src1, src2, size| {
                masm.float_cmp_with_set(src1, src2, dst, FloatCmpKind::Lt, size);
            },
        );
    }

    fn visit_f32_gt(&mut self) {
        self.context.float_cmp_op(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src1, src2, size| {
                masm.float_cmp_with_set(src1, src2, dst, FloatCmpKind::Gt, size);
            },
        );
    }

    fn visit_f64_gt(&mut self) {
        self.context.float_cmp_op(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src1, src2, size| {
                masm.float_cmp_with_set(src1, src2, dst, FloatCmpKind::Gt, size);
            },
        );
    }

    fn visit_f32_le(&mut self) {
        self.context.float_cmp_op(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src1, src2, size| {
                masm.float_cmp_with_set(src1, src2, dst, FloatCmpKind::Le, size);
            },
        );
    }

    fn visit_f64_le(&mut self) {
        self.context.float_cmp_op(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src1, src2, size| {
                masm.float_cmp_with_set(src1, src2, dst, FloatCmpKind::Le, size);
            },
        );
    }

    fn visit_f32_ge(&mut self) {
        self.context.float_cmp_op(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src1, src2, size| {
                masm.float_cmp_with_set(src1, src2, dst, FloatCmpKind::Ge, size);
            },
        );
    }

    fn visit_f64_ge(&mut self) {
        self.context.float_cmp_op(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src1, src2, size| {
                masm.float_cmp_with_set(src1, src2, dst, FloatCmpKind::Ge, size);
            },
        );
    }

    fn visit_f32_convert_i32_s(&mut self) {
        self.context
            .convert_op(self.masm, WasmValType::F32, |masm, dst, src, dst_size| {
                masm.signed_convert(src, dst, OperandSize::S32, dst_size);
            });
    }

    fn visit_f32_convert_i32_u(&mut self) {
        self.context.convert_op_with_tmp_reg(
            self.masm,
            WasmValType::F32,
            RegClass::Int,
            |masm, dst, src, tmp_gpr, dst_size| {
                masm.unsigned_convert(src, dst, tmp_gpr, OperandSize::S32, dst_size);
            },
        );
    }

    fn visit_f32_convert_i64_s(&mut self) {
        self.context
            .convert_op(self.masm, WasmValType::F32, |masm, dst, src, dst_size| {
                masm.signed_convert(src, dst, OperandSize::S64, dst_size);
            });
    }

    fn visit_f32_convert_i64_u(&mut self) {
        self.context.convert_op_with_tmp_reg(
            self.masm,
            WasmValType::F32,
            RegClass::Int,
            |masm, dst, src, tmp_gpr, dst_size| {
                masm.unsigned_convert(src, dst, tmp_gpr, OperandSize::S64, dst_size);
            },
        );
    }

    fn visit_f64_convert_i32_s(&mut self) {
        self.context
            .convert_op(self.masm, WasmValType::F64, |masm, dst, src, dst_size| {
                masm.signed_convert(src, dst, OperandSize::S32, dst_size);
            });
    }

    fn visit_f64_convert_i32_u(&mut self) {
        self.context.convert_op_with_tmp_reg(
            self.masm,
            WasmValType::F64,
            RegClass::Int,
            |masm, dst, src, tmp_gpr, dst_size| {
                masm.unsigned_convert(src, dst, tmp_gpr, OperandSize::S32, dst_size);
            },
        );
    }

    fn visit_f64_convert_i64_s(&mut self) {
        self.context
            .convert_op(self.masm, WasmValType::F64, |masm, dst, src, dst_size| {
                masm.signed_convert(src, dst, OperandSize::S64, dst_size);
            });
    }

    fn visit_f64_convert_i64_u(&mut self) {
        self.context.convert_op_with_tmp_reg(
            self.masm,
            WasmValType::F64,
            RegClass::Int,
            |masm, dst, src, tmp_gpr, dst_size| {
                masm.unsigned_convert(src, dst, tmp_gpr, OperandSize::S64, dst_size);
            },
        );
    }

    fn visit_f32_reinterpret_i32(&mut self) {
        self.context
            .convert_op(self.masm, WasmValType::F32, |masm, dst, src, size| {
                masm.reinterpret_int_as_float(src.into(), dst, size);
            });
    }

    fn visit_f64_reinterpret_i64(&mut self) {
        self.context
            .convert_op(self.masm, WasmValType::F64, |masm, dst, src, size| {
                masm.reinterpret_int_as_float(src.into(), dst, size);
            });
    }

    fn visit_f32_demote_f64(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S64, &mut |masm, reg, _size| {
                masm.demote(reg, reg);
                TypedReg::f32(reg)
            });
    }

    fn visit_f64_promote_f32(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S32, &mut |masm, reg, _size| {
                masm.promote(reg, reg);
                TypedReg::f64(reg)
            });
    }

    fn visit_i32_add(&mut self) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.add(dst, dst, src, size);
            TypedReg::i32(dst)
        });
    }

    fn visit_i64_add(&mut self) {
        self.context.i64_binop(self.masm, |masm, dst, src, size| {
            masm.add(dst, dst, src, size);
            TypedReg::i64(dst)
        });
    }

    fn visit_i32_sub(&mut self) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.sub(dst, dst, src, size);
            TypedReg::i32(dst)
        });
    }

    fn visit_i64_sub(&mut self) {
        self.context.i64_binop(self.masm, |masm, dst, src, size| {
            masm.sub(dst, dst, src, size);
            TypedReg::i64(dst)
        });
    }

    fn visit_i32_mul(&mut self) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.mul(dst, dst, src, size);
            TypedReg::i32(dst)
        });
    }

    fn visit_i64_mul(&mut self) {
        self.context.i64_binop(self.masm, |masm, dst, src, size| {
            masm.mul(dst, dst, src, size);
            TypedReg::i64(dst)
        });
    }

    fn visit_i32_div_s(&mut self) {
        use DivKind::*;
        use OperandSize::*;

        self.masm.div(&mut self.context, Signed, S32);
    }

    fn visit_i32_div_u(&mut self) {
        use DivKind::*;
        use OperandSize::*;

        self.masm.div(&mut self.context, Unsigned, S32);
    }

    fn visit_i64_div_s(&mut self) {
        use DivKind::*;
        use OperandSize::*;

        self.masm.div(&mut self.context, Signed, S64);
    }

    fn visit_i64_div_u(&mut self) {
        use DivKind::*;
        use OperandSize::*;

        self.masm.div(&mut self.context, Unsigned, S64);
    }

    fn visit_i32_rem_s(&mut self) {
        use OperandSize::*;
        use RemKind::*;

        self.masm.rem(&mut self.context, Signed, S32);
    }

    fn visit_i32_rem_u(&mut self) {
        use OperandSize::*;
        use RemKind::*;

        self.masm.rem(&mut self.context, Unsigned, S32);
    }

    fn visit_i64_rem_s(&mut self) {
        use OperandSize::*;
        use RemKind::*;

        self.masm.rem(&mut self.context, Signed, S64);
    }

    fn visit_i64_rem_u(&mut self) {
        use OperandSize::*;
        use RemKind::*;

        self.masm.rem(&mut self.context, Unsigned, S64);
    }

    fn visit_i32_eq(&mut self) {
        self.cmp_i32s(IntCmpKind::Eq);
    }

    fn visit_i64_eq(&mut self) {
        self.cmp_i64s(IntCmpKind::Eq);
    }

    fn visit_i32_ne(&mut self) {
        self.cmp_i32s(IntCmpKind::Ne);
    }

    fn visit_i64_ne(&mut self) {
        self.cmp_i64s(IntCmpKind::Ne);
    }

    fn visit_i32_lt_s(&mut self) {
        self.cmp_i32s(IntCmpKind::LtS);
    }

    fn visit_i64_lt_s(&mut self) {
        self.cmp_i64s(IntCmpKind::LtS);
    }

    fn visit_i32_lt_u(&mut self) {
        self.cmp_i32s(IntCmpKind::LtU);
    }

    fn visit_i64_lt_u(&mut self) {
        self.cmp_i64s(IntCmpKind::LtU);
    }

    fn visit_i32_le_s(&mut self) {
        self.cmp_i32s(IntCmpKind::LeS);
    }

    fn visit_i64_le_s(&mut self) {
        self.cmp_i64s(IntCmpKind::LeS);
    }

    fn visit_i32_le_u(&mut self) {
        self.cmp_i32s(IntCmpKind::LeU);
    }

    fn visit_i64_le_u(&mut self) {
        self.cmp_i64s(IntCmpKind::LeU);
    }

    fn visit_i32_gt_s(&mut self) {
        self.cmp_i32s(IntCmpKind::GtS);
    }

    fn visit_i64_gt_s(&mut self) {
        self.cmp_i64s(IntCmpKind::GtS);
    }

    fn visit_i32_gt_u(&mut self) {
        self.cmp_i32s(IntCmpKind::GtU);
    }

    fn visit_i64_gt_u(&mut self) {
        self.cmp_i64s(IntCmpKind::GtU);
    }

    fn visit_i32_ge_s(&mut self) {
        self.cmp_i32s(IntCmpKind::GeS);
    }

    fn visit_i64_ge_s(&mut self) {
        self.cmp_i64s(IntCmpKind::GeS);
    }

    fn visit_i32_ge_u(&mut self) {
        self.cmp_i32s(IntCmpKind::GeU);
    }

    fn visit_i64_ge_u(&mut self) {
        self.cmp_i64s(IntCmpKind::GeU);
    }

    fn visit_i32_eqz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S32, &mut |masm, reg, size| {
            masm.cmp_with_set(RegImm::i32(0), reg.into(), IntCmpKind::Eq, size);
            TypedReg::i32(reg)
        });
    }

    fn visit_i64_eqz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S64, &mut |masm, reg, size| {
            masm.cmp_with_set(RegImm::i64(0), reg.into(), IntCmpKind::Eq, size);
            TypedReg::i32(reg) // Return value for `i64.eqz` is an `i32`.
        });
    }

    fn visit_i32_clz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S32, &mut |masm, reg, size| {
            masm.clz(reg, reg, size);
            TypedReg::i32(reg)
        });
    }

    fn visit_i64_clz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S64, &mut |masm, reg, size| {
            masm.clz(reg, reg, size);
            TypedReg::i64(reg)
        });
    }

    fn visit_i32_ctz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S32, &mut |masm, reg, size| {
            masm.ctz(reg, reg, size);
            TypedReg::i32(reg)
        });
    }

    fn visit_i64_ctz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S64, &mut |masm, reg, size| {
            masm.ctz(reg, reg, size);
            TypedReg::i64(reg)
        });
    }

    fn visit_i32_and(&mut self) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.and(dst, dst, src, size);
            TypedReg::i32(dst)
        });
    }

    fn visit_i64_and(&mut self) {
        self.context.i64_binop(self.masm, |masm, dst, src, size| {
            masm.and(dst, dst, src, size);
            TypedReg::i64(dst)
        });
    }

    fn visit_i32_or(&mut self) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.or(dst, dst, src, size);
            TypedReg::i32(dst)
        });
    }

    fn visit_i64_or(&mut self) {
        self.context.i64_binop(self.masm, |masm, dst, src, size| {
            masm.or(dst, dst, src, size);
            TypedReg::i64(dst)
        });
    }

    fn visit_i32_xor(&mut self) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.xor(dst, dst, src, size);
            TypedReg::i32(dst)
        });
    }

    fn visit_i64_xor(&mut self) {
        self.context.i64_binop(self.masm, |masm, dst, src, size| {
            masm.xor(dst, dst, src, size);
            TypedReg::i64(dst)
        });
    }

    fn visit_i32_shl(&mut self) {
        use ShiftKind::*;

        self.context.i32_shift(self.masm, Shl);
    }

    fn visit_i64_shl(&mut self) {
        use ShiftKind::*;

        self.context.i64_shift(self.masm, Shl);
    }

    fn visit_i32_shr_s(&mut self) {
        use ShiftKind::*;

        self.context.i32_shift(self.masm, ShrS);
    }

    fn visit_i64_shr_s(&mut self) {
        use ShiftKind::*;

        self.context.i64_shift(self.masm, ShrS);
    }

    fn visit_i32_shr_u(&mut self) {
        use ShiftKind::*;

        self.context.i32_shift(self.masm, ShrU);
    }

    fn visit_i64_shr_u(&mut self) {
        use ShiftKind::*;

        self.context.i64_shift(self.masm, ShrU);
    }

    fn visit_i32_rotl(&mut self) {
        use ShiftKind::*;

        self.context.i32_shift(self.masm, Rotl);
    }

    fn visit_i64_rotl(&mut self) {
        use ShiftKind::*;

        self.context.i64_shift(self.masm, Rotl);
    }

    fn visit_i32_rotr(&mut self) {
        use ShiftKind::*;

        self.context.i32_shift(self.masm, Rotr);
    }

    fn visit_i64_rotr(&mut self) {
        use ShiftKind::*;

        self.context.i64_shift(self.masm, Rotr);
    }

    fn visit_end(&mut self) {
        if !self.context.reachable {
            self.handle_unreachable_end();
        } else {
            let mut control = self.control_frames.pop().unwrap();
            control.emit_end(self.masm, &mut self.context);
        }
    }

    fn visit_i32_popcnt(&mut self) {
        use OperandSize::*;
        self.masm.popcnt(&mut self.context, S32);
    }

    fn visit_i64_popcnt(&mut self) {
        use OperandSize::*;

        self.masm.popcnt(&mut self.context, S64);
    }

    fn visit_i32_wrap_i64(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S64, &mut |masm, reg, _size| {
            masm.wrap(reg, reg);
            TypedReg::i32(reg)
        });
    }

    fn visit_i64_extend_i32_s(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S32, &mut |masm, reg, _size| {
            masm.extend(reg, reg, ExtendKind::I64ExtendI32S);
            TypedReg::i64(reg)
        });
    }

    fn visit_i64_extend_i32_u(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S32, &mut |masm, reg, _size| {
            masm.extend(reg, reg, ExtendKind::I64ExtendI32U);
            TypedReg::i64(reg)
        });
    }

    fn visit_i32_extend8_s(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S32, &mut |masm, reg, _size| {
            masm.extend(reg, reg, ExtendKind::I32Extend8S);
            TypedReg::i32(reg)
        });
    }

    fn visit_i32_extend16_s(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S32, &mut |masm, reg, _size| {
            masm.extend(reg, reg, ExtendKind::I32Extend16S);
            TypedReg::i32(reg)
        });
    }

    fn visit_i64_extend8_s(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S64, &mut |masm, reg, _size| {
            masm.extend(reg, reg, ExtendKind::I64Extend8S);
            TypedReg::i64(reg)
        });
    }

    fn visit_i64_extend16_s(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S64, &mut |masm, reg, _size| {
            masm.extend(reg, reg, ExtendKind::I64Extend16S);
            TypedReg::i64(reg)
        });
    }

    fn visit_i64_extend32_s(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S64, &mut |masm, reg, _size| {
            masm.extend(reg, reg, ExtendKind::I64Extend32S);
            TypedReg::i64(reg)
        });
    }

    fn visit_i32_trunc_f32_s(&mut self) {
        use OperandSize::*;

        self.context
            .convert_op(self.masm, WasmValType::I32, |masm, dst, src, dst_size| {
                masm.signed_truncate(src, dst, S32, dst_size, TruncKind::Unchecked);
            });
    }

    fn visit_i32_trunc_f32_u(&mut self) {
        use OperandSize::*;

        self.context.convert_op_with_tmp_reg(
            self.masm,
            WasmValType::I32,
            RegClass::Float,
            |masm, dst, src, tmp_fpr, dst_size| {
                masm.unsigned_truncate(src, dst, tmp_fpr, S32, dst_size, TruncKind::Unchecked);
            },
        );
    }

    fn visit_i32_trunc_f64_s(&mut self) {
        use OperandSize::*;

        self.context
            .convert_op(self.masm, WasmValType::I32, |masm, dst, src, dst_size| {
                masm.signed_truncate(src, dst, S64, dst_size, TruncKind::Unchecked);
            });
    }

    fn visit_i32_trunc_f64_u(&mut self) {
        use OperandSize::*;

        self.context.convert_op_with_tmp_reg(
            self.masm,
            WasmValType::I32,
            RegClass::Float,
            |masm, dst, src, tmp_fpr, dst_size| {
                masm.unsigned_truncate(src, dst, tmp_fpr, S64, dst_size, TruncKind::Unchecked);
            },
        );
    }

    fn visit_i64_trunc_f32_s(&mut self) {
        use OperandSize::*;

        self.context
            .convert_op(self.masm, WasmValType::I64, |masm, dst, src, dst_size| {
                masm.signed_truncate(src, dst, S32, dst_size, TruncKind::Unchecked);
            });
    }

    fn visit_i64_trunc_f32_u(&mut self) {
        use OperandSize::*;

        self.context.convert_op_with_tmp_reg(
            self.masm,
            WasmValType::I64,
            RegClass::Float,
            |masm, dst, src, tmp_fpr, dst_size| {
                masm.unsigned_truncate(src, dst, tmp_fpr, S32, dst_size, TruncKind::Unchecked);
            },
        );
    }

    fn visit_i64_trunc_f64_s(&mut self) {
        use OperandSize::*;

        self.context
            .convert_op(self.masm, WasmValType::I64, |masm, dst, src, dst_size| {
                masm.signed_truncate(src, dst, S64, dst_size, TruncKind::Unchecked);
            });
    }

    fn visit_i64_trunc_f64_u(&mut self) {
        use OperandSize::*;

        self.context.convert_op_with_tmp_reg(
            self.masm,
            WasmValType::I64,
            RegClass::Float,
            |masm, dst, src, tmp_fpr, dst_size| {
                masm.unsigned_truncate(src, dst, tmp_fpr, S64, dst_size, TruncKind::Unchecked);
            },
        );
    }

    fn visit_i32_reinterpret_f32(&mut self) {
        self.context
            .convert_op(self.masm, WasmValType::I32, |masm, dst, src, size| {
                masm.reinterpret_float_as_int(src.into(), dst, size);
            });
    }

    fn visit_i64_reinterpret_f64(&mut self) {
        self.context
            .convert_op(self.masm, WasmValType::I64, |masm, dst, src, size| {
                masm.reinterpret_float_as_int(src.into(), dst, size);
            });
    }

    fn visit_local_get(&mut self, index: u32) {
        use WasmValType::*;
        let context = &mut self.context;
        let slot = context.frame.get_wasm_local(index);
        match slot.ty {
            I32 | I64 | F32 | F64 | V128 => context.stack.push(Val::local(index, slot.ty)),
            Ref(rt) => match rt.heap_type {
                WasmHeapType::Func => context.stack.push(Val::local(index, slot.ty)),
                ht => unimplemented!("Support for WasmHeapType: {ht}"),
            },
        }
    }

    fn visit_local_set(&mut self, index: u32) {
        let src = self.emit_set_local(index);
        self.context.free_reg(src);
    }

    fn visit_call(&mut self, index: u32) {
        let callee = self.env.callee_from_index(FuncIndex::from_u32(index));
        FnCall::emit::<M>(&mut self.env, self.masm, &mut self.context, callee)
    }

    fn visit_call_indirect(&mut self, type_index: u32, table_index: u32) {
        // Spill now because `emit_lazy_init_funcref` and the `FnCall::emit`
        // invocations will both trigger spills since they both call functions.
        // However, the machine instructions for the spill emitted by
        // `emit_lazy_funcref` will be jumped over if the funcref was previously
        // initialized which may result in the machine stack becoming
        // unbalanced.
        self.context.spill(self.masm);

        let type_index = TypeIndex::from_u32(type_index);
        let table_index = TableIndex::from_u32(table_index);

        self.emit_lazy_init_funcref(table_index);

        // Perform the indirect call.
        // This code assumes that [`Self::emit_lazy_init_funcref`] will
        // push the funcref to the value stack.
        match self.env.translation.module.table_plans[table_index].style {
            TableStyle::CallerChecksSignature { lazy_init: true } => {
                let funcref_ptr = self.context.stack.peek().map(|v| v.unwrap_reg()).unwrap();
                self.masm
                    .trapz(funcref_ptr.into(), TrapCode::IndirectCallToNull);
                self.emit_typecheck_funcref(funcref_ptr.into(), type_index);
            }
            _ => unimplemented!("Support for eager table init"),
        }

        let callee = self.env.funcref(type_index);
        FnCall::emit::<M>(&mut self.env, self.masm, &mut self.context, callee)
    }

    fn visit_table_init(&mut self, elem: u32, table: u32) {
        debug_assert!(self.context.stack.len() >= 3);
        let at = self.context.stack.len() - 3;

        self.context
            .stack
            .insert_many(at, &[table.try_into().unwrap(), elem.try_into().unwrap()]);

        let builtin = self.env.builtins.table_init::<M::ABI, M::Ptr>();
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(builtin.clone()),
        )
    }

    fn visit_table_copy(&mut self, dst: u32, src: u32) {
        debug_assert!(self.context.stack.len() >= 3);
        let at = self.context.stack.len() - 3;
        self.context
            .stack
            .insert_many(at, &[dst.try_into().unwrap(), src.try_into().unwrap()]);

        let builtin = self.env.builtins.table_copy::<M::ABI, M::Ptr>();
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(builtin),
        )
    }

    fn visit_table_get(&mut self, table: u32) {
        let table_index = TableIndex::from_u32(table);
        let plan = self.env.table_plan(table_index);
        let heap_type = plan.table.wasm_ty.heap_type;
        let style = &plan.style;

        match heap_type {
            WasmHeapType::Func => match style {
                TableStyle::CallerChecksSignature { lazy_init: true } => {
                    self.emit_lazy_init_funcref(table_index)
                }
                _ => unimplemented!("Support for eager table init"),
            },
            t => unimplemented!("Support for WasmHeapType: {t}"),
        }
    }

    fn visit_table_grow(&mut self, table: u32) {
        let table_index = TableIndex::from_u32(table);
        let table_plan = self.env.table_plan(table_index);
        let builtin = match table_plan.table.wasm_ty.heap_type {
            WasmHeapType::Func => self.env.builtins.table_grow_func_ref::<M::ABI, M::Ptr>(),
            ty => unimplemented!("Support for HeapType: {ty}"),
        };

        let len = self.context.stack.len();
        // table.grow` requires at least 2 elements on the value stack.
        debug_assert!(len >= 2);
        let at = len - 2;

        // The table_grow builtin expects the parameters in a different
        // order.
        // The value stack at this point should contain:
        // [ init_value | delta ] (stack top)
        // but the builtin function expects the init value as the last
        // argument.
        self.context.stack.inner_mut().swap(len - 1, len - 2);
        self.context
            .stack
            .insert_many(at, &[table.try_into().unwrap()]);

        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(builtin.clone()),
        )
    }

    fn visit_table_size(&mut self, table: u32) {
        let table_index = TableIndex::from_u32(table);
        let table_data = self.env.resolve_table_data(table_index);
        self.emit_compute_table_size(&table_data);
    }

    fn visit_table_fill(&mut self, table: u32) {
        let table_index = TableIndex::from_u32(table);
        let table_plan = self.env.table_plan(table_index);
        let builtin = match table_plan.table.wasm_ty.heap_type {
            WasmHeapType::Func => self.env.builtins.table_fill_func_ref::<M::ABI, M::Ptr>(),
            ty => unimplemented!("Support for heap type: {ty}"),
        };

        let len = self.context.stack.len();
        debug_assert!(len >= 3);
        let at = len - 3;
        self.context
            .stack
            .insert_many(at, &[table.try_into().unwrap()]);
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(builtin.clone()),
        )
    }

    fn visit_table_set(&mut self, table: u32) {
        let ptr_type = self.env.ptr_type();
        let table_index = TableIndex::from_u32(table);
        let table_data = self.env.resolve_table_data(table_index);
        let plan = self.env.table_plan(table_index);
        match plan.table.wasm_ty.heap_type {
            WasmHeapType::Func => match plan.style {
                TableStyle::CallerChecksSignature { lazy_init: true } => {
                    let value = self.context.pop_to_reg(self.masm, None);
                    let index = self.context.pop_to_reg(self.masm, None);
                    let base = self.context.any_gpr(self.masm);
                    let elem_addr =
                        self.emit_compute_table_elem_addr(index.into(), base, &table_data);
                    // Set the initialized bit.
                    self.masm.or(
                        value.into(),
                        value.into(),
                        RegImm::i64(FUNCREF_INIT_BIT as i64),
                        ptr_type.into(),
                    );

                    self.masm.store_ptr(value.into(), elem_addr);

                    self.context.free_reg(value);
                    self.context.free_reg(index);
                    self.context.free_reg(base);
                }
                _ => unimplemented!("Support for eager table init"),
            },
            ty => unimplemented!("Support for WasmHeapType: {ty}"),
        };
    }

    fn visit_elem_drop(&mut self, index: u32) {
        let elem_drop = self.env.builtins.elem_drop::<M::ABI, M::Ptr>();
        self.context.stack.extend([index.try_into().unwrap()]);
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(elem_drop),
        )
    }

    fn visit_memory_init(&mut self, data_index: u32, mem: u32) {
        debug_assert!(self.context.stack.len() >= 3);
        let at = self.context.stack.len() - 3;
        self.context.stack.insert_many(
            at,
            &[mem.try_into().unwrap(), data_index.try_into().unwrap()],
        );
        let builtin = self.env.builtins.memory_init::<M::ABI, M::Ptr>();
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(builtin),
        )
    }

    fn visit_memory_copy(&mut self, dst_mem: u32, src_mem: u32) {
        // At this point, the stack is expected to contain:
        //     [ dst_offset, src_offset, len ]
        // The following code inserts the missing params, so that stack contains:
        //     [ vmctx, dst_mem, dst_offset, src_mem, src_offset, len ]
        // Which is the order expected by the builtin function.
        debug_assert!(self.context.stack.len() >= 3);
        let at = self.context.stack.len() - 2;
        self.context
            .stack
            .insert_many(at, &[src_mem.try_into().unwrap()]);

        // One element was inserted above, so instead of 3, we use 4.
        let at = self.context.stack.len() - 4;
        self.context
            .stack
            .insert_many(at, &[dst_mem.try_into().unwrap()]);

        let builtin = self.env.builtins.memory_copy::<M::ABI, M::Ptr>();

        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(builtin),
        )
    }

    fn visit_memory_fill(&mut self, mem: u32) {
        debug_assert!(self.context.stack.len() >= 3);
        let at = self.context.stack.len() - 3;

        self.context
            .stack
            .insert_many(at, &[mem.try_into().unwrap()]);

        let builtin = self.env.builtins.memory_fill::<M::ABI, M::Ptr>();
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(builtin),
        )
    }

    fn visit_memory_size(&mut self, mem: u32) {
        let heap = self.env.resolve_heap(MemoryIndex::from_u32(mem));
        self.emit_compute_memory_size(&heap);
    }

    fn visit_memory_grow(&mut self, mem: u32) {
        debug_assert!(self.context.stack.len() >= 1);
        // The stack at this point contains: [ delta ]
        // The desired state is
        //   [ vmctx, delta, index ]
        self.context.stack.extend([mem.try_into().unwrap()]);

        let heap = self.env.resolve_heap(MemoryIndex::from_u32(mem));
        let builtin = self.env.builtins.memory32_grow::<M::ABI, M::Ptr>();
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(builtin),
        );

        // The memory32_grow builtin returns a pointer type, therefore we must
        // ensure that the return type is representative of the address space of
        // the heap type.
        match (self.env.ptr_type(), heap.ty) {
            (WasmValType::I64, WasmValType::I64) => {}
            // When the heap type is smaller than the pointer type, we adjust
            // the result of the memory32_grow builtin.
            (WasmValType::I64, WasmValType::I32) => {
                let top: Reg = self.context.pop_to_reg(self.masm, None).into();
                self.masm.wrap(top.into(), top.into());
                self.context.stack.push(TypedReg::i32(top).into());
            }
            _ => unimplemented!("Support for 32-bit platforms"),
        }
    }

    fn visit_data_drop(&mut self, data_index: u32) {
        self.context.stack.extend([data_index.try_into().unwrap()]);

        let builtin = self.env.builtins.data_drop::<M::ABI, M::Ptr>();
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(builtin),
        )
    }

    fn visit_nop(&mut self) {}

    fn visit_if(&mut self, blockty: BlockType) {
        self.control_frames.push(ControlStackFrame::r#if(
            self.env.resolve_block_sig(blockty),
            self.masm,
            &mut self.context,
        ));
    }

    fn visit_else(&mut self) {
        if !self.context.reachable {
            self.handle_unreachable_else();
        } else {
            let control = self
                .control_frames
                .last_mut()
                .unwrap_or_else(|| panic!("Expected active control stack frame for else"));
            control.emit_else(self.masm, &mut self.context);
        }
    }

    fn visit_block(&mut self, blockty: BlockType) {
        self.control_frames.push(ControlStackFrame::block(
            self.env.resolve_block_sig(blockty),
            self.masm,
            &mut self.context,
        ));
    }

    fn visit_loop(&mut self, blockty: BlockType) {
        self.control_frames.push(ControlStackFrame::r#loop(
            self.env.resolve_block_sig(blockty),
            self.masm,
            &mut self.context,
        ));
    }

    fn visit_br(&mut self, depth: u32) {
        let index = control_index(depth, self.control_frames.len());
        let frame = &mut self.control_frames[index];
        self.context
            .unconditional_jump(frame, self.masm, |masm, cx, frame| {
                frame
                    .pop_abi_results::<M, _>(cx, masm, |results, _, _| results.ret_area().copied());
            });
    }

    fn visit_br_if(&mut self, depth: u32) {
        let index = control_index(depth, self.control_frames.len());
        let frame = &mut self.control_frames[index];
        frame.set_as_target();

        let top = {
            let top = self.context.without::<TypedReg, M, _>(
                frame.results::<M>().regs(),
                self.masm,
                |ctx, masm| ctx.pop_to_reg(masm, None),
            );
            // Explicitly save any live registers and locals before setting up
            // the branch state.
            // In some cases, calculating the `top` value above, will result in
            // a spill, thus the following one will result in a no-op.
            self.context.spill(self.masm);
            frame.top_abi_results::<M, _>(
                &mut self.context,
                self.masm,
                |results, context, masm| {
                    // In the case of `br_if` there's a possibility that we'll
                    // exit early from the block or fallthrough, for
                    // a fallthrough, we cannot rely on the pre-computed return area;
                    // it must be recalculated so that any values that are
                    // generated are correctly placed near the current stack
                    // pointer.
                    results.on_stack().then(|| {
                        let stack_consumed = context.stack.sizeof(results.stack_operands_len());
                        let base = masm.sp_offset().as_u32() - stack_consumed;
                        let offs = base + results.size();
                        RetArea::sp(SPOffset::from_u32(offs))
                    })
                },
            );
            top
        };

        // Emit instructions to balance the machine stack if the frame has
        // a different offset.
        let current_sp_offset = self.masm.sp_offset();
        let results_size = frame.results::<M>().size();
        let state = frame.stack_state();
        let (label, cmp, needs_cleanup) = if current_sp_offset > state.target_offset {
            (self.masm.get_label(), IntCmpKind::Eq, true)
        } else {
            (*frame.label(), IntCmpKind::Ne, false)
        };

        self.masm
            .branch(cmp, top.reg.into(), top.reg.into(), label, OperandSize::S32);
        self.context.free_reg(top);

        if needs_cleanup {
            // Emit instructions to balance the stack and jump if not falling
            // through.
            self.masm.memmove(
                current_sp_offset,
                state.target_offset,
                results_size,
                MemMoveDirection::LowToHigh,
            );
            self.masm.ensure_sp_for_jump(state.target_offset);
            self.masm.jmp(*frame.label());

            // Restore sp_offset to what it was for falling through and emit
            // fallthrough label.
            self.masm.reset_stack_pointer(current_sp_offset);
            self.masm.bind(label);
        }
    }

    fn visit_br_table(&mut self, targets: BrTable<'a>) {
        // +1 to account for the default target.
        let len = targets.len() + 1;
        // SmallVec<[_; 5]> to match the binary emission layer (e.g
        // see `JmpTableSeq'), but here we use 5 instead since we
        // bundle the default target as the last element in the array.
        let labels: SmallVec<[_; 5]> = (0..len).map(|_| self.masm.get_label()).collect();

        let default_index = control_index(targets.default(), self.control_frames.len());
        let default_frame = &mut self.control_frames[default_index];
        let default_result = default_frame.results::<M>();

        let (index, tmp) = {
            let index_and_tmp = self.context.without::<(TypedReg, _), M, _>(
                default_result.regs(),
                self.masm,
                |cx, masm| (cx.pop_to_reg(masm, None), cx.any_gpr(masm)),
            );

            // Materialize any constants or locals into their result representation,
            // so that when reachability is restored, they are correctly located.
            default_frame.top_abi_results::<M, _>(&mut self.context, self.masm, |results, _, _| {
                results.ret_area().copied()
            });
            index_and_tmp
        };

        self.masm.jmp_table(&labels, index.into(), tmp);
        // Save the original stack pointer offset; we will reset the stack
        // pointer to this offset after jumping to each of the targets. Each
        // jump might adjust the stack according to the base offset of the
        // target.
        let current_sp = self.masm.sp_offset();

        for (t, l) in targets
            .targets()
            .into_iter()
            .chain(std::iter::once(Ok(targets.default())))
            .zip(labels.iter())
        {
            let control_index = control_index(t.unwrap(), self.control_frames.len());
            let frame = &mut self.control_frames[control_index];
            // Reset the stack pointer to its original offset. This is needed
            // because each jump will potentially adjust the stack pointer
            // according to the base offset of the target.
            self.masm.reset_stack_pointer(current_sp);

            // NB: We don't perform any result handling as it was
            // already taken care of above before jumping to the
            // jump table.
            self.masm.bind(*l);
            // Ensure that the stack pointer is correctly positioned before
            // jumping to the jump table code.
            let state = frame.stack_state();
            self.masm.ensure_sp_for_jump(state.target_offset);
            self.masm.jmp(*frame.label());
            frame.set_as_target();
        }
        // Finally reset the stack pointer to the original location.
        // The reachability analysis, will ensure it's correctly located
        // once reachability is restored.
        self.masm.reset_stack_pointer(current_sp);
        self.context.reachable = false;
        self.context.free_reg(index.reg);
        self.context.free_reg(tmp);
    }

    fn visit_return(&mut self) {
        // Grab the outermost frame, which is the function's body
        // frame. We don't rely on [`codegen::control_index`] since
        // this frame is implicit and we know that it should exist at
        // index 0.
        let outermost = &mut self.control_frames[0];
        self.context
            .unconditional_jump(outermost, self.masm, |masm, cx, frame| {
                frame
                    .pop_abi_results::<M, _>(cx, masm, |results, _, _| results.ret_area().copied());
            });
    }

    fn visit_unreachable(&mut self) {
        self.masm.unreachable();
        self.context.reachable = false;
        // Set the implicit outermost frame as target to perform the necessary
        // stack clean up.
        let outermost = &mut self.control_frames[0];
        outermost.set_as_target();
    }

    fn visit_local_tee(&mut self, index: u32) {
        let typed_reg = self.emit_set_local(index);
        self.context.stack.push(typed_reg.into());
    }

    fn visit_global_get(&mut self, global_index: u32) {
        let index = GlobalIndex::from_u32(global_index);
        let (ty, addr) = self.emit_get_global_addr(index);
        let dst = self.context.reg_for_type(ty, self.masm);
        self.masm.load(addr, dst, ty.into());
        self.context.stack.push(Val::reg(dst, ty));
    }

    fn visit_global_set(&mut self, global_index: u32) {
        let index = GlobalIndex::from_u32(global_index);
        let (ty, addr) = self.emit_get_global_addr(index);

        let typed_reg = self.context.pop_to_reg(self.masm, None);
        self.context.free_reg(typed_reg.reg);
        self.masm.store(typed_reg.reg.into(), addr, ty.into());
    }

    fn visit_drop(&mut self) {
        self.context.drop_last(1, |regalloc, val| match val {
            Val::Reg(tr) => regalloc.free(tr.reg.into()),
            Val::Memory(m) => self.masm.free_stack(m.slot.size),
            _ => {}
        });
    }

    fn visit_select(&mut self) {
        let cond = self.context.pop_to_reg(self.masm, None);
        let val2 = self.context.pop_to_reg(self.masm, None);
        let val1 = self.context.pop_to_reg(self.masm, None);
        self.masm
            .cmp(cond.reg.into(), RegImm::i32(0), OperandSize::S32);
        // Conditionally move val1 to val2 if the comparison is
        // not zero.
        self.masm
            .cmov(val1.into(), val2.into(), IntCmpKind::Ne, val1.ty.into());
        self.context.stack.push(val2.into());
        self.context.free_reg(val1.reg);
        self.context.free_reg(cond);
    }

    fn visit_i32_load(&mut self, memarg: MemArg) {
        self.emit_wasm_load(&memarg, WasmValType::I32, OperandSize::S32, None);
    }

    fn visit_i32_load8_s(&mut self, memarg: MemArg) {
        self.emit_wasm_load(
            &memarg,
            WasmValType::I32,
            OperandSize::S8,
            Some(ExtendKind::I32Extend8S),
        );
    }

    fn visit_i32_load8_u(&mut self, memarg: MemArg) {
        self.emit_wasm_load(&memarg, WasmValType::I32, OperandSize::S8, None);
    }

    fn visit_i32_load16_s(&mut self, memarg: MemArg) {
        self.emit_wasm_load(
            &memarg,
            WasmValType::I32,
            OperandSize::S16,
            Some(ExtendKind::I32Extend16S),
        )
    }

    fn visit_i32_load16_u(&mut self, memarg: MemArg) {
        self.emit_wasm_load(&memarg, WasmValType::I32, OperandSize::S16, None)
    }

    fn visit_i32_store(&mut self, memarg: MemArg) {
        self.emit_wasm_store(&memarg, OperandSize::S32);
    }

    fn visit_i32_store8(&mut self, memarg: MemArg) {
        self.emit_wasm_store(&memarg, OperandSize::S8)
    }

    fn visit_i32_store16(&mut self, memarg: MemArg) {
        self.emit_wasm_store(&memarg, OperandSize::S16)
    }

    fn visit_i64_load8_s(&mut self, memarg: MemArg) {
        self.emit_wasm_load(
            &memarg,
            WasmValType::I64,
            OperandSize::S8,
            Some(ExtendKind::I64Extend8S),
        )
    }

    fn visit_i64_load8_u(&mut self, memarg: MemArg) {
        self.emit_wasm_load(&memarg, WasmValType::I64, OperandSize::S8, None)
    }

    fn visit_i64_load16_u(&mut self, memarg: MemArg) {
        self.emit_wasm_load(&memarg, WasmValType::I64, OperandSize::S16, None)
    }

    fn visit_i64_load16_s(&mut self, memarg: MemArg) {
        self.emit_wasm_load(
            &memarg,
            WasmValType::I64,
            OperandSize::S16,
            Some(ExtendKind::I64Extend16S),
        )
    }

    fn visit_i64_load32_u(&mut self, memarg: MemArg) {
        self.emit_wasm_load(&memarg, WasmValType::I64, OperandSize::S32, None)
    }

    fn visit_i64_load32_s(&mut self, memarg: MemArg) {
        self.emit_wasm_load(
            &memarg,
            WasmValType::I64,
            OperandSize::S32,
            Some(ExtendKind::I64Extend32S),
        )
    }

    fn visit_i64_load(&mut self, memarg: MemArg) {
        self.emit_wasm_load(&memarg, WasmValType::I64, OperandSize::S64, None)
    }

    fn visit_i64_store(&mut self, memarg: MemArg) -> Self::Output {
        self.emit_wasm_store(&memarg, OperandSize::S64)
    }

    fn visit_i64_store8(&mut self, memarg: MemArg) -> Self::Output {
        self.emit_wasm_store(&memarg, OperandSize::S8)
    }

    fn visit_i64_store16(&mut self, memarg: MemArg) -> Self::Output {
        self.emit_wasm_store(&memarg, OperandSize::S16)
    }

    fn visit_i64_store32(&mut self, memarg: MemArg) -> Self::Output {
        self.emit_wasm_store(&memarg, OperandSize::S32)
    }

    fn visit_f32_load(&mut self, memarg: MemArg) {
        self.emit_wasm_load(&memarg, WasmValType::F32, OperandSize::S32, None)
    }

    fn visit_f32_store(&mut self, memarg: MemArg) {
        self.emit_wasm_store(&memarg, OperandSize::S32)
    }

    fn visit_f64_load(&mut self, memarg: MemArg) {
        self.emit_wasm_load(&memarg, WasmValType::F64, OperandSize::S64, None)
    }

    fn visit_f64_store(&mut self, memarg: MemArg) {
        self.emit_wasm_store(&memarg, OperandSize::S64)
    }

    fn visit_v128_load(&mut self, memarg: MemArg) {
        self.emit_wasm_load(&memarg, WasmValType::V128, OperandSize::S128, None)
    }

    fn visit_v128_store(&mut self, memarg: MemArg) {
        self.emit_wasm_store(&memarg, OperandSize::S128)
    }

    fn visit_i32_trunc_sat_f32_s(&mut self) {
        use OperandSize::*;

        self.context
            .convert_op(self.masm, WasmValType::I32, |masm, dst, src, dst_size| {
                masm.signed_truncate(src, dst, S32, dst_size, TruncKind::Checked);
            });
    }

    fn visit_i32_trunc_sat_f32_u(&mut self) {
        use OperandSize::*;

        self.context.convert_op_with_tmp_reg(
            self.masm,
            WasmValType::I32,
            RegClass::Float,
            |masm, dst, src, tmp_fpr, dst_size| {
                masm.unsigned_truncate(src, dst, tmp_fpr, S32, dst_size, TruncKind::Checked);
            },
        );
    }

    fn visit_i32_trunc_sat_f64_s(&mut self) {
        use OperandSize::*;

        self.context
            .convert_op(self.masm, WasmValType::I32, |masm, dst, src, dst_size| {
                masm.signed_truncate(src, dst, S64, dst_size, TruncKind::Checked);
            });
    }

    fn visit_i32_trunc_sat_f64_u(&mut self) {
        use OperandSize::*;

        self.context.convert_op_with_tmp_reg(
            self.masm,
            WasmValType::I32,
            RegClass::Float,
            |masm, dst, src, tmp_fpr, dst_size| {
                masm.unsigned_truncate(src, dst, tmp_fpr, S64, dst_size, TruncKind::Checked);
            },
        );
    }

    fn visit_i64_trunc_sat_f32_s(&mut self) {
        use OperandSize::*;

        self.context
            .convert_op(self.masm, WasmValType::I64, |masm, dst, src, dst_size| {
                masm.signed_truncate(src, dst, S32, dst_size, TruncKind::Checked);
            });
    }

    fn visit_i64_trunc_sat_f32_u(&mut self) {
        use OperandSize::*;

        self.context.convert_op_with_tmp_reg(
            self.masm,
            WasmValType::I64,
            RegClass::Float,
            |masm, dst, src, tmp_fpr, dst_size| {
                masm.unsigned_truncate(src, dst, tmp_fpr, S32, dst_size, TruncKind::Checked);
            },
        );
    }

    fn visit_i64_trunc_sat_f64_s(&mut self) {
        use OperandSize::*;

        self.context
            .convert_op(self.masm, WasmValType::I64, |masm, dst, src, dst_size| {
                masm.signed_truncate(src, dst, S64, dst_size, TruncKind::Checked);
            });
    }

    fn visit_i64_trunc_sat_f64_u(&mut self) {
        use OperandSize::*;

        self.context.convert_op_with_tmp_reg(
            self.masm,
            WasmValType::I64,
            RegClass::Float,
            |masm, dst, src, tmp_fpr, dst_size| {
                masm.unsigned_truncate(src, dst, tmp_fpr, S64, dst_size, TruncKind::Checked);
            },
        );
    }

    wasmparser::for_each_operator!(def_unsupported);
}

impl<'a, 'translation, 'data, M> CodeGen<'a, 'translation, 'data, M>
where
    M: MacroAssembler,
{
    fn cmp_i32s(&mut self, kind: IntCmpKind) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.cmp_with_set(src, dst, kind, size);
            TypedReg::i32(dst)
        });
    }

    fn cmp_i64s(&mut self, kind: IntCmpKind) {
        self.context
            .i64_binop(self.masm, move |masm, dst, src, size| {
                masm.cmp_with_set(src, dst, kind, size);
                TypedReg::i32(dst) // Return value for comparisons is an `i32`.
            });
    }
}

impl From<WasmValType> for OperandSize {
    fn from(ty: WasmValType) -> OperandSize {
        match ty {
            WasmValType::I32 | WasmValType::F32 => OperandSize::S32,
            WasmValType::I64 | WasmValType::F64 => OperandSize::S64,
            WasmValType::V128 => OperandSize::S128,
            WasmValType::Ref(rt) => {
                match rt.heap_type {
                    // TODO: Hardcoded size, assuming 64-bit support only. Once
                    // Wasmtime supports 32-bit architectures, this will need
                    // to be updated in such a way that the calculation of the
                    // OperandSize will depend on the target's  pointer size.
                    WasmHeapType::Func => OperandSize::S64,
                    t => unimplemented!("Support for WasmHeapType: {t}"),
                }
            }
        }
    }
}
