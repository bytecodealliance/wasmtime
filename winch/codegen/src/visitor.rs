//! This module is the central place for machine code emission.
//! It defines an implementation of wasmparser's Visitor trait
//! for `CodeGen`; which defines a visitor per op-code,
//! which validates and dispatches to the corresponding
//! machine code emitter.

use crate::codegen::CodeGen;
use crate::masm::{CmpKind, DivKind, MacroAssembler, OperandSize, RegImm, RemKind};
use crate::stack::Val;
use wasmparser::VisitOperator;
use wasmtime_environ::{FuncIndex, WasmType};

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
    (emit LocalGet $($rest:tt)*) => {};
    (emit LocalSet $($rest:tt)*) => {};
    (emit Call $($rest:tt)*) => {};
    (emit End $($rest:tt)*) => {};
    (emit Nop $($rest:tt)*) => {};

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

    fn visit_i32_add(&mut self) {
        self.context
            .i32_binop(self.masm, &mut |masm, dst, src, size| {
                masm.add(dst, dst, src, size);
            });
    }

    fn visit_i64_add(&mut self) {
        self.context
            .i64_binop(self.masm, &mut |masm, dst, src, size| {
                masm.add(dst, dst, src, size);
            });
    }

    fn visit_i32_sub(&mut self) {
        self.context
            .i32_binop(self.masm, &mut |masm, dst, src, size| {
                masm.sub(dst, dst, src, size);
            });
    }

    fn visit_i64_sub(&mut self) {
        self.context
            .i64_binop(self.masm, &mut |masm, dst, src, size| {
                masm.sub(dst, dst, src, size);
            });
    }

    fn visit_i32_mul(&mut self) {
        self.context
            .i32_binop(self.masm, &mut |masm, dst, src, size| {
                masm.mul(dst, dst, src, size);
            });
    }

    fn visit_i64_mul(&mut self) {
        self.context
            .i64_binop(self.masm, &mut |masm, dst, src, size| {
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
            masm.cmp_with_set(RegImm::imm(0), reg, CmpKind::Eq, size);
        });
    }

    fn visit_i64_eqz(&mut self) {
        use OperandSize::*;

        self.context.unop(self.masm, S64, &mut |masm, reg, size| {
            masm.cmp_with_set(RegImm::imm(0), reg, CmpKind::Eq, size);
        });
    }

    fn visit_end(&mut self) {}

    fn visit_local_get(&mut self, index: u32) {
        let context = &mut self.context;
        let slot = context
            .frame
            .get_local(index)
            .expect(&format!("valid local at slot = {}", index));
        match slot.ty {
            WasmType::I32 | WasmType::I64 => context.stack.push(Val::local(index)),
            _ => panic!("Unsupported type {:?} for local", slot.ty),
        }
    }

    // TODO verify the case where the target local is on the stack.
    fn visit_local_set(&mut self, index: u32) {
        let context = &mut self.context;
        let frame = context.frame;
        let slot = frame
            .get_local(index)
            .expect(&format!("vald local at slot = {}", index));
        let size: OperandSize = slot.ty.into();
        let src = self.context.pop_to_reg(self.masm, None, size);
        let addr = self.masm.local_address(&slot);
        self.masm.store(RegImm::reg(src), addr, size);
        self.context.regalloc.free_gpr(src);
    }

    fn visit_call(&mut self, index: u32) {
        self.emit_call(FuncIndex::from_u32(index));
    }

    fn visit_nop(&mut self) {}

    wasmparser::for_each_operator!(def_unsupported);
}

impl<'a, M> CodeGen<'a, M>
where
    M: MacroAssembler,
{
    fn cmp_i32s(&mut self, kind: CmpKind) {
        self.context
            .i32_binop(self.masm, &mut |masm, dst, src, size| {
                masm.cmp_with_set(src, dst, kind, size);
            });
    }

    fn cmp_i64s(&mut self, kind: CmpKind) {
        self.context
            .i64_binop(self.masm, &mut move |masm, dst, src, size| {
                masm.cmp_with_set(src, dst, kind, size);
            });
    }
}

impl From<WasmType> for OperandSize {
    fn from(ty: WasmType) -> OperandSize {
        match ty {
            WasmType::I32 => OperandSize::S32,
            WasmType::I64 => OperandSize::S64,
            ty => todo!("unsupported type {:?}", ty),
        }
    }
}
