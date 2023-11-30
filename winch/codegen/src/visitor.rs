//! This module is the central place for machine code emission.
//! It defines an implementation of wasmparser's Visitor trait
//! for `CodeGen`; which defines a visitor per op-code,
//! which validates and dispatches to the corresponding
//! machine code emitter.

use crate::abi::ABI;
use crate::codegen::{control_index, Callee, CodeGen, ControlStackFrame, FnCall};
use crate::masm::{
    DivKind, FloatCmpKind, IntCmpKind, MacroAssembler, OperandSize, RegImm, RemKind, RoundingMode,
    ShiftKind,
};
use crate::stack::{TypedReg, Val};
use cranelift_codegen::ir::TrapCode;
use smallvec::SmallVec;
use wasmparser::BrTable;
use wasmparser::{BlockType, Ieee32, Ieee64, VisitOperator};
use wasmtime_environ::{
    FuncIndex, GlobalIndex, TableIndex, TableStyle, TypeIndex, WasmHeapType, WasmType,
    FUNCREF_INIT_BIT,
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

    fn visit_f32_add(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_add(dst, dst, src, size);
            },
        );
    }

    fn visit_f64_add(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_add(dst, dst, src, size);
            },
        );
    }

    fn visit_f32_sub(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_sub(dst, dst, src, size);
            },
        );
    }

    fn visit_f64_sub(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_sub(dst, dst, src, size);
            },
        );
    }

    fn visit_f32_mul(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_mul(dst, dst, src, size);
            },
        );
    }

    fn visit_f64_mul(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_mul(dst, dst, src, size);
            },
        );
    }

    fn visit_f32_div(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_div(dst, dst, src, size);
            },
        );
    }

    fn visit_f64_div(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_div(dst, dst, src, size);
            },
        );
    }

    fn visit_f32_min(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_min(dst, dst, src, size);
            },
        );
    }

    fn visit_f64_min(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_min(dst, dst, src, size);
            },
        );
    }

    fn visit_f32_max(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_max(dst, dst, src, size);
            },
        );
    }

    fn visit_f64_max(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_max(dst, dst, src, size);
            },
        );
    }

    fn visit_f32_copysign(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S32,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_copysign(dst, dst, src, size);
            },
        );
    }

    fn visit_f64_copysign(&mut self) {
        self.context.binop(
            self.masm,
            OperandSize::S64,
            &mut |masm: &mut M, dst, src, size| {
                masm.float_copysign(dst, dst, src, size);
            },
        );
    }

    fn visit_f32_abs(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S32, &mut |masm, reg, size| {
                masm.float_abs(reg, size);
            });
    }

    fn visit_f64_abs(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S64, &mut |masm, reg, size| {
                masm.float_abs(reg, size);
            });
    }

    fn visit_f32_neg(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S32, &mut |masm, reg, size| {
                masm.float_neg(reg, size);
            });
    }

    fn visit_f64_neg(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S64, &mut |masm, reg, size| {
                masm.float_neg(reg, size);
            });
    }

    fn visit_f32_floor(&mut self) {
        self.masm
            .float_round(RoundingMode::Down, &mut self.context, OperandSize::S32);
    }

    fn visit_f64_floor(&mut self) {
        self.masm
            .float_round(RoundingMode::Down, &mut self.context, OperandSize::S64);
    }

    fn visit_f32_ceil(&mut self) {
        self.masm
            .float_round(RoundingMode::Up, &mut self.context, OperandSize::S32);
    }

    fn visit_f64_ceil(&mut self) {
        self.masm
            .float_round(RoundingMode::Up, &mut self.context, OperandSize::S64);
    }

    fn visit_f32_nearest(&mut self) {
        self.masm
            .float_round(RoundingMode::Nearest, &mut self.context, OperandSize::S32);
    }

    fn visit_f64_nearest(&mut self) {
        self.masm
            .float_round(RoundingMode::Nearest, &mut self.context, OperandSize::S64);
    }

    fn visit_f32_trunc(&mut self) {
        self.masm
            .float_round(RoundingMode::Zero, &mut self.context, OperandSize::S32);
    }

    fn visit_f64_trunc(&mut self) {
        self.masm
            .float_round(RoundingMode::Zero, &mut self.context, OperandSize::S64);
    }

    fn visit_f32_sqrt(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S32, &mut |masm, reg, size| {
                masm.float_sqrt(reg, reg, size);
            });
    }

    fn visit_f64_sqrt(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S64, &mut |masm, reg, size| {
                masm.float_sqrt(reg, reg, size);
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

    fn visit_i32_add(&mut self) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.add(dst, dst, src, size);
        });
    }

    fn visit_i64_add(&mut self) {
        self.context.i64_binop(self.masm, |masm, dst, src, size| {
            masm.add(dst, dst, src, size);
        });
    }

    fn visit_i32_sub(&mut self) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.sub(dst, dst, src, size);
        });
    }

    fn visit_i64_sub(&mut self) {
        self.context.i64_binop(self.masm, |masm, dst, src, size| {
            masm.sub(dst, dst, src, size);
        });
    }

    fn visit_i32_mul(&mut self) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.mul(dst, dst, src, size);
        });
    }

    fn visit_i64_mul(&mut self) {
        self.context.i64_binop(self.masm, |masm, dst, src, size| {
            masm.mul(dst, dst, src, size);
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
        });
    }

    fn visit_i64_eqz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S64, &mut |masm, reg, size| {
            masm.cmp_with_set(RegImm::i64(0), reg.into(), IntCmpKind::Eq, size);
        });
    }

    fn visit_i32_clz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S32, &mut |masm, reg, size| {
            masm.clz(reg, reg, size);
        });
    }

    fn visit_i64_clz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S64, &mut |masm, reg, size| {
            masm.clz(reg, reg, size);
        });
    }

    fn visit_i32_ctz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S32, &mut |masm, reg, size| {
            masm.ctz(reg, reg, size);
        });
    }

    fn visit_i64_ctz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S64, &mut |masm, reg, size| {
            masm.ctz(reg, reg, size);
        });
    }

    fn visit_i32_and(&mut self) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.and(dst, dst, src, size);
        });
    }

    fn visit_i64_and(&mut self) {
        self.context.i64_binop(self.masm, |masm, dst, src, size| {
            masm.and(dst, dst, src, size);
        });
    }

    fn visit_i32_or(&mut self) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.or(dst, dst, src, size);
        });
    }

    fn visit_i64_or(&mut self) {
        self.context.i64_binop(self.masm, |masm, dst, src, size| {
            masm.or(dst, dst, src, size);
        });
    }

    fn visit_i32_xor(&mut self) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.xor(dst, dst, src, size);
        });
    }

    fn visit_i64_xor(&mut self) {
        self.context.i64_binop(self.masm, |masm, dst, src, size| {
            masm.xor(dst, dst, src, size);
        });
    }

    fn visit_i32_shl(&mut self) {
        use OperandSize::*;
        use ShiftKind::*;

        self.masm.shift(&mut self.context, Shl, S32);
    }

    fn visit_i64_shl(&mut self) {
        use OperandSize::*;
        use ShiftKind::*;

        self.masm.shift(&mut self.context, Shl, S64);
    }

    fn visit_i32_shr_s(&mut self) {
        use OperandSize::*;
        use ShiftKind::*;

        self.masm.shift(&mut self.context, ShrS, S32);
    }

    fn visit_i64_shr_s(&mut self) {
        use OperandSize::*;
        use ShiftKind::*;

        self.masm.shift(&mut self.context, ShrS, S64);
    }

    fn visit_i32_shr_u(&mut self) {
        use OperandSize::*;
        use ShiftKind::*;

        self.masm.shift(&mut self.context, ShrU, S32);
    }

    fn visit_i64_shr_u(&mut self) {
        use OperandSize::*;
        use ShiftKind::*;

        self.masm.shift(&mut self.context, ShrU, S64);
    }

    fn visit_i32_rotl(&mut self) {
        use OperandSize::*;
        use ShiftKind::*;

        self.masm.shift(&mut self.context, Rotl, S32);
    }

    fn visit_i64_rotl(&mut self) {
        use OperandSize::*;
        use ShiftKind::*;

        self.masm.shift(&mut self.context, Rotl, S64);
    }

    fn visit_i32_rotr(&mut self) {
        use OperandSize::*;
        use ShiftKind::*;

        self.masm.shift(&mut self.context, Rotr, S32);
    }

    fn visit_i64_rotr(&mut self) {
        use OperandSize::*;
        use ShiftKind::*;

        self.masm.shift(&mut self.context, Rotr, S64);
    }

    fn visit_end(&mut self) {
        if !self.context.reachable {
            self.handle_unreachable_end();
        } else {
            let mut control = self.control_frames.pop().unwrap();
            let is_outermost = self.control_frames.len() == 0;
            // If it's not the outermost control stack frame, emit the the full "end" sequence,
            // which involves, popping results from the value stack, pushing results back to the
            // value stack and binding the exit label.
            // Else, pop values from the value stack and bind the exit label.
            if !is_outermost {
                control.emit_end(self.masm, &mut self.context);
            } else {
                if let Some(data) = control.results() {
                    self.context.pop_abi_results(data, self.masm);
                }
                control.bind_exit_label(self.masm);
            }
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

    fn visit_local_get(&mut self, index: u32) {
        use WasmType::*;
        let context = &mut self.context;
        let slot = context
            .frame
            .get_local(index)
            .unwrap_or_else(|| panic!("valid local at slot = {}", index));
        match slot.ty {
            I32 | I64 | F32 | F64 => context.stack.push(Val::local(index, slot.ty)),
            Ref(rt) => match rt.heap_type {
                WasmHeapType::Func => context.stack.push(Val::local(index, slot.ty)),
                ht => unimplemented!("Support for WasmHeapType: {ht}"),
            },
            t => unimplemented!("Support local type: {t}"),
        }
    }

    fn visit_local_set(&mut self, index: u32) {
        let src = self.emit_set_local(index);
        self.context.free_reg(src);
    }

    fn visit_call(&mut self, index: u32) {
        let callee = self.env.callee_from_index(FuncIndex::from_u32(index));
        FnCall::emit::<M, M::Ptr, _>(self.masm, &mut self.context, |_| callee.clone());
    }

    fn visit_call_indirect(&mut self, type_index: u32, table_index: u32, _: u8) {
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
            TableStyle::CallerChecksSignature => {
                let funcref_ptr = self.context.stack.peek().map(|v| v.unwrap_reg()).unwrap();
                self.masm
                    .trapz(funcref_ptr.into(), TrapCode::IndirectCallToNull);
                self.emit_typecheck_funcref(funcref_ptr.into(), type_index);
            }
        }

        FnCall::emit::<M, M::Ptr, _>(self.masm, &mut self.context, |_| {
            self.env.funcref(type_index)
        })
    }

    fn visit_table_init(&mut self, elem: u32, table: u32) {
        let ptr_type = self.env.ptr_type();
        let vmctx = TypedReg::new(ptr_type, <M::ABI as ABI>::vmctx_reg());

        debug_assert!(self.context.stack.len() >= 3);
        let at = self.context.stack.len() - 3;

        self.context.stack.insert_many(
            at,
            [
                vmctx.into(),
                table.try_into().unwrap(),
                elem.try_into().unwrap(),
            ],
        );
        FnCall::emit::<M, M::Ptr, _>(self.masm, &mut self.context, |cx| {
            Callee::Builtin(cx.builtins.table_init::<M::ABI, M::Ptr>())
        });
    }

    fn visit_table_copy(&mut self, dst: u32, src: u32) {
        let ptr_type = self.env.ptr_type();
        let vmctx = TypedReg::new(ptr_type, <M::ABI as ABI>::vmctx_reg());
        debug_assert!(self.context.stack.len() >= 3);
        let at = self.context.stack.len() - 3;
        self.context.stack.insert_many(
            at,
            [
                vmctx.into(),
                dst.try_into().unwrap(),
                src.try_into().unwrap(),
            ],
        );

        FnCall::emit::<M, M::Ptr, _>(self.masm, &mut self.context, |context| {
            Callee::Builtin(context.builtins.table_copy::<M::ABI, M::Ptr>())
        });
    }

    fn visit_table_get(&mut self, table: u32) {
        let table_index = TableIndex::from_u32(table);
        let plan = self.env.table_plan(table_index);
        let heap_type = plan.table.wasm_ty.heap_type;
        let style = &plan.style;

        match heap_type {
            WasmHeapType::Func => match style {
                TableStyle::CallerChecksSignature => self.emit_lazy_init_funcref(table_index),
            },
            t => unimplemented!("Support for WasmHeapType: {t}"),
        }
    }

    fn visit_table_grow(&mut self, table: u32) {
        let ptr_type = self.env.ptr_type();
        let vmctx = TypedReg::new(ptr_type, <M::ABI as ABI>::vmctx_reg());
        let table_index = TableIndex::from_u32(table);
        let table_plan = self.env.table_plan(table_index);
        let builtin = match table_plan.table.wasm_ty.heap_type {
            WasmHeapType::Func => self
                .context
                .builtins
                .table_grow_func_ref::<M::ABI, M::Ptr>(),
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
            .insert_many(at, [vmctx.into(), table.try_into().unwrap()]);

        FnCall::emit::<M, M::Ptr, _>(self.masm, &mut self.context, |_| {
            Callee::Builtin(builtin.clone())
        });
    }

    fn visit_table_size(&mut self, table: u32) {
        let table_index = TableIndex::from_u32(table);
        let table_data = self.env.resolve_table_data(table_index);
        self.masm.table_size(&table_data, &mut self.context);
    }

    fn visit_table_fill(&mut self, table: u32) {
        let ptr_type = self.env.ptr_type();
        let vmctx = TypedReg::new(ptr_type, <M::ABI as ABI>::vmctx_reg());
        let table_index = TableIndex::from_u32(table);
        let table_plan = self.env.table_plan(table_index);
        let builtin = match table_plan.table.wasm_ty.heap_type {
            WasmHeapType::Func => self
                .context
                .builtins
                .table_fill_func_ref::<M::ABI, M::Ptr>(),
            ty => unimplemented!("Support for heap type: {ty}"),
        };

        let len = self.context.stack.len();
        debug_assert!(len >= 3);
        let at = len - 3;
        self.context
            .stack
            .insert_many(at, [vmctx.into(), table.try_into().unwrap()]);
        FnCall::emit::<M, M::Ptr, _>(self.masm, &mut self.context, |_| {
            Callee::Builtin(builtin.clone())
        })
    }

    fn visit_table_set(&mut self, table: u32) {
        let ptr_type = self.env.ptr_type();
        let table_index = TableIndex::from_u32(table);
        let table_data = self.env.resolve_table_data(table_index);
        let plan = self.env.table_plan(table_index);
        match plan.table.wasm_ty.heap_type {
            WasmHeapType::Func => match plan.style {
                TableStyle::CallerChecksSignature => {
                    let value = self.context.pop_to_reg(self.masm, None);
                    let index = self.context.pop_to_reg(self.masm, None);
                    let base = self.context.any_gpr(self.masm);
                    let elem_addr = self.masm.table_elem_address(
                        index.into(),
                        base,
                        &table_data,
                        &mut self.context,
                    );

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
            },
            ty => unimplemented!("Support for WasmHeapType: {ty}"),
        };
    }

    fn visit_elem_drop(&mut self, index: u32) {
        let ptr_type = self.env.ptr_type();
        let elem_drop = self.context.builtins.elem_drop::<M::ABI, M::Ptr>();
        let vmctx = TypedReg::new(ptr_type, <M::ABI as ABI>::vmctx_reg());
        self.context
            .stack
            .extend([vmctx.into(), index.try_into().unwrap()]);
        FnCall::emit::<M, M::Ptr, _>(self.masm, &mut self.context, |_| {
            Callee::Builtin(elem_drop.clone())
        });
    }

    fn visit_nop(&mut self) {}

    fn visit_if(&mut self, blockty: BlockType) {
        self.control_frames.push(ControlStackFrame::r#if(
            self.env.resolve_block_results_data::<M::ABI>(blockty),
            self.env.resolve_block_type_info(blockty),
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
            self.env.resolve_block_results_data::<M::ABI>(blockty),
            self.env.resolve_block_type_info(blockty),
            self.masm,
            &mut self.context,
        ));
    }

    fn visit_loop(&mut self, blockty: BlockType) {
        self.control_frames.push(ControlStackFrame::r#loop(
            self.env.resolve_block_type_info(blockty),
            self.masm,
            &mut self.context,
        ));
    }

    fn visit_br(&mut self, depth: u32) {
        let index = control_index(depth, self.control_frames.len());
        let frame = &mut self.control_frames[index];
        self.context
            .unconditional_jump(frame, self.masm, |masm, cx, frame| {
                if let Some(r) = frame.as_target_results() {
                    cx.pop_abi_results(r, masm);
                }
            });
    }

    fn visit_br_if(&mut self, depth: u32) {
        let index = control_index(depth, self.control_frames.len());
        let frame = &mut self.control_frames[index];
        frame.set_as_target();

        let top = if let Some(data) = frame.as_target_results() {
            let top = self.context.without::<TypedReg, M, _>(
                data.results.regs(),
                self.masm,
                |ctx, masm| ctx.pop_to_reg(masm, None),
            );
            self.context.top_abi_results(data, self.masm);
            top
        } else {
            self.context.pop_to_reg(self.masm, None)
        };

        // Emit instructions to balance the machine stack if the frame has
        // a different offset.
        let current_sp_offset = self.masm.sp_offset();
        let (_, frame_sp_offset) = frame.base_stack_len_and_sp();
        let (label, cmp, needs_cleanup) = if current_sp_offset > frame_sp_offset {
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
            self.masm.ensure_sp_for_jump(frame_sp_offset);
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
        let default_result = self.control_frames[default_index].as_target_results();

        let (index, tmp) = if let Some(data) = default_result {
            let index_and_tmp = self.context.without::<(TypedReg, _), M, _>(
                data.results.regs(),
                self.masm,
                |cx, masm| (cx.pop_to_reg(masm, None), cx.any_gpr(masm)),
            );

            // Materialize any constants or locals into their result representation,
            // so that when reachability is restored, they are correctly located.
            self.context.top_abi_results(data, self.masm);
            index_and_tmp
        } else {
            (
                self.context.pop_to_reg(self.masm, None),
                self.context.any_gpr(self.masm),
            )
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
            let (_, offset) = frame.base_stack_len_and_sp();
            self.masm.ensure_sp_for_jump(offset);
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
                if let Some(data) = frame.as_target_results() {
                    cx.pop_abi_results(data, masm);
                }
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
        let (ty, offset) = self.env.resolve_global_type_and_offset(index);
        let addr = self
            .masm
            .address_at_reg(<M::ABI as ABI>::vmctx_reg(), offset);
        let dst = self.context.reg_for_type(ty, self.masm);
        self.masm.load(addr, dst, ty.into());
        self.context.stack.push(Val::reg(dst, ty));
    }

    fn visit_global_set(&mut self, global_index: u32) {
        let index = GlobalIndex::from_u32(global_index);
        let (ty, offset) = self.env.resolve_global_type_and_offset(index);
        let addr = self
            .masm
            .address_at_reg(<M::ABI as ABI>::vmctx_reg(), offset);
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
            .cmp(RegImm::i32(0), cond.reg.into(), OperandSize::S32);
        // Conditionally move val1 to val2 if the the comparision is
        // not zero.
        self.masm
            .cmov(val1.into(), val2.into(), IntCmpKind::Ne, val1.ty.into());
        self.context.stack.push(val2.into());
        self.context.free_reg(val1.reg);
        self.context.free_reg(cond);
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
        });
    }

    fn cmp_i64s(&mut self, kind: IntCmpKind) {
        self.context
            .i64_binop(self.masm, move |masm, dst, src, size| {
                masm.cmp_with_set(src, dst, kind, size);
            });
    }
}

impl From<WasmType> for OperandSize {
    fn from(ty: WasmType) -> OperandSize {
        match ty {
            WasmType::I32 | WasmType::F32 => OperandSize::S32,
            WasmType::I64 | WasmType::F64 => OperandSize::S64,
            WasmType::Ref(rt) => {
                match rt.heap_type {
                    // TODO: Harcoded size, assuming 64-bit support only. Once
                    // Wasmtime supports 32-bit architectures, this will need
                    // to be updated in such a way that the calculation of the
                    // OperandSize will depend on the target's  pointer size.
                    WasmHeapType::Func => OperandSize::S64,
                    t => unimplemented!("Support for WasmHeapType: {t}"),
                }
            }
            ty => unimplemented!("Support for WasmType {ty}"),
        }
    }
}
