//! This module is the central place for machine code emission.
//! It defines an implementation of wasmparser's Visitor trait
//! for `CodeGen`; which defines a visitor per op-code,
//! which validates and dispatches to the corresponding
//! machine code emitter.

use crate::abi::ABI;
use crate::codegen::ControlStackFrame;
use crate::codegen::{control_index, CodeGen};
use crate::masm::{
    CmpKind, DivKind, MacroAssembler, OperandSize, RegImm, RemKind, RoundingMode, ShiftKind,
};
use crate::stack::{TypedReg, Val};
use smallvec::SmallVec;
use wasmparser::BrTable;
use wasmparser::{BlockType, Ieee32, Ieee64, VisitOperator};
use wasmtime_environ::{FuncIndex, GlobalIndex, WasmType};

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

    (emit $unsupported:tt $($rest:tt)*) => {$($rest)*};
}

impl<'a, M> VisitOperator<'a> for CodeGen<'a, M>
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
        self.context
            .unop(self.masm, OperandSize::S32, &mut |masm, reg, size| {
                masm.float_round(RoundingMode::Down, reg, RegImm::Reg(reg), size);
            });
    }

    fn visit_f64_floor(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S64, &mut |masm, reg, size| {
                masm.float_round(RoundingMode::Down, reg, RegImm::Reg(reg), size);
            });
    }

    fn visit_f32_ceil(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S32, &mut |masm, reg, size| {
                masm.float_round(RoundingMode::Up, reg, RegImm::Reg(reg), size);
            });
    }

    fn visit_f64_ceil(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S64, &mut |masm, reg, size| {
                masm.float_round(RoundingMode::Up, reg, RegImm::Reg(reg), size);
            });
    }

    fn visit_f32_nearest(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S32, &mut |masm, reg, size| {
                masm.float_round(RoundingMode::Nearest, reg, RegImm::Reg(reg), size);
            });
    }

    fn visit_f64_nearest(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S64, &mut |masm, reg, size| {
                masm.float_round(RoundingMode::Nearest, reg, RegImm::Reg(reg), size);
            });
    }

    fn visit_f32_trunc(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S32, &mut |masm, reg, size| {
                masm.float_round(RoundingMode::Zero, reg, RegImm::Reg(reg), size);
            });
    }

    fn visit_f64_trunc(&mut self) {
        self.context
            .unop(self.masm, OperandSize::S64, &mut |masm, reg, size| {
                masm.float_round(RoundingMode::Zero, reg, RegImm::Reg(reg), size);
            });
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
        self.cmp_i32s(CmpKind::Eq);
    }

    fn visit_i64_eq(&mut self) {
        self.cmp_i64s(CmpKind::Eq);
    }

    fn visit_i32_ne(&mut self) {
        self.cmp_i32s(CmpKind::Ne);
    }

    fn visit_i64_ne(&mut self) {
        self.cmp_i64s(CmpKind::Ne);
    }

    fn visit_i32_lt_s(&mut self) {
        self.cmp_i32s(CmpKind::LtS);
    }

    fn visit_i64_lt_s(&mut self) {
        self.cmp_i64s(CmpKind::LtS);
    }

    fn visit_i32_lt_u(&mut self) {
        self.cmp_i32s(CmpKind::LtU);
    }

    fn visit_i64_lt_u(&mut self) {
        self.cmp_i64s(CmpKind::LtU);
    }

    fn visit_i32_le_s(&mut self) {
        self.cmp_i32s(CmpKind::LeS);
    }

    fn visit_i64_le_s(&mut self) {
        self.cmp_i64s(CmpKind::LeS);
    }

    fn visit_i32_le_u(&mut self) {
        self.cmp_i32s(CmpKind::LeU);
    }

    fn visit_i64_le_u(&mut self) {
        self.cmp_i64s(CmpKind::LeU);
    }

    fn visit_i32_gt_s(&mut self) {
        self.cmp_i32s(CmpKind::GtS);
    }

    fn visit_i64_gt_s(&mut self) {
        self.cmp_i64s(CmpKind::GtS);
    }

    fn visit_i32_gt_u(&mut self) {
        self.cmp_i32s(CmpKind::GtU);
    }

    fn visit_i64_gt_u(&mut self) {
        self.cmp_i64s(CmpKind::GtU);
    }

    fn visit_i32_ge_s(&mut self) {
        self.cmp_i32s(CmpKind::GeS);
    }

    fn visit_i64_ge_s(&mut self) {
        self.cmp_i64s(CmpKind::GeS);
    }

    fn visit_i32_ge_u(&mut self) {
        self.cmp_i32s(CmpKind::GeU);
    }

    fn visit_i64_ge_u(&mut self) {
        self.cmp_i64s(CmpKind::GeU);
    }

    fn visit_i32_eqz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S32, &mut |masm, reg, size| {
            masm.cmp_with_set(RegImm::i32(0), reg.into(), CmpKind::Eq, size);
        });
    }

    fn visit_i64_eqz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S64, &mut |masm, reg, size| {
            masm.cmp_with_set(RegImm::i64(0), reg.into(), CmpKind::Eq, size);
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
                self.context.pop_abi_results(control.result(), self.masm);
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
            _ => panic!("Unsupported type {:?} for local", slot.ty),
        }
    }

    // TODO verify the case where the target local is on the stack.
    fn visit_local_set(&mut self, index: u32) {
        let src = self.context.set_local(self.masm, index);
        self.context.free_reg(src);
    }

    fn visit_call(&mut self, index: u32) {
        self.emit_call(FuncIndex::from_u32(index));
    }

    fn visit_nop(&mut self) {}

    fn visit_if(&mut self, blockty: BlockType) {
        self.control_frames.push(ControlStackFrame::if_(
            &self.env.resolve_block_type(blockty),
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
            &self.env.resolve_block_type(blockty),
            self.masm,
            &mut self.context,
        ));
    }

    fn visit_loop(&mut self, blockty: BlockType) {
        self.control_frames.push(ControlStackFrame::loop_(
            &self.env.resolve_block_type(blockty),
            self.masm,
            &mut self.context,
        ));
    }

    fn visit_br(&mut self, depth: u32) {
        let index = control_index(depth, self.control_frames.len());
        let frame = &mut self.control_frames[index];
        self.context
            .unconditional_jump(frame, self.masm, |masm, cx, frame| {
                cx.pop_abi_results(&frame.as_target_result(), masm);
            });
    }

    fn visit_br_if(&mut self, depth: u32) {
        let index = control_index(depth, self.control_frames.len());
        let frame = &mut self.control_frames[index];
        frame.set_as_target();
        let result = frame.as_target_result();
        let top =
            self.context
                .without::<TypedReg, M, _>(result.result_reg(), self.masm, |ctx, masm| {
                    ctx.pop_to_reg(masm, None)
                });
        self.context.pop_abi_results(&result, self.masm);
        self.context.push_abi_results(&result, self.masm);
        self.masm.branch(
            CmpKind::Ne,
            top.reg.into(),
            top.reg.into(),
            *frame.label(),
            OperandSize::S32,
        );
        self.context.free_reg(top);
    }

    fn visit_br_table(&mut self, targets: BrTable<'a>) {
        // +1 to account for the default target.
        let len = targets.len() + 1;
        // SmallVec<[_; 5]> to match the binary emission layer (e.g
        // see `JmpTableSeq'), but here we use 5 instead since we
        // bundle the default target as the last element in the array.
        let labels: SmallVec<[_; 5]> = (0..len).map(|_| self.masm.get_label()).collect();

        let default_index = control_index(targets.default(), self.control_frames.len());
        let default_result = self.control_frames[default_index].as_target_result();
        let (index, tmp) = self.context.without::<(TypedReg, _), M, _>(
            default_result.result_reg(),
            self.masm,
            |cx, masm| (cx.pop_to_reg(masm, None), cx.any_gpr(masm)),
        );

        self.context.pop_abi_results(&default_result, self.masm);
        self.context.push_abi_results(&default_result, self.masm);
        self.masm.jmp_table(&labels, index.into(), tmp);

        for (t, l) in targets
            .targets()
            .into_iter()
            .chain(std::iter::once(Ok(targets.default())))
            .zip(labels.iter())
        {
            let control_index = control_index(t.unwrap(), self.control_frames.len());

            self.masm.bind(*l);
            self.context.unconditional_jump(
                &mut self.control_frames[control_index],
                self.masm,
                // NB: We don't perform any result handling as it was
                // already taken care of above before jumping to the
                // jump table above. The call to `unconditional_jump`,
                // will stil take care of the proper stack alignment.
                |_, _, _| {},
            )
        }
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
                cx.pop_abi_results(&frame.as_target_result(), masm);
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
        let typed_reg = self.context.set_local(self.masm, index);
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
            .cmov(val1.into(), val2.into(), CmpKind::Ne, val1.ty.into());
        self.context.stack.push(val2.into());
        self.context.free_reg(val1.reg);
        self.context.free_reg(cond);
    }

    wasmparser::for_each_operator!(def_unsupported);
}

impl<'a, M> CodeGen<'a, M>
where
    M: MacroAssembler,
{
    fn cmp_i32s(&mut self, kind: CmpKind) {
        self.context.i32_binop(self.masm, |masm, dst, src, size| {
            masm.cmp_with_set(src, dst, kind, size);
        });
    }

    fn cmp_i64s(&mut self, kind: CmpKind) {
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
            ty => todo!("unsupported type {:?}", ty),
        }
    }
}
