//! This module is the central place for machine code emission.
//! It defines an implementation of wasmparser's Visitor trait
//! for `CodeGen`; which defines a visitor per op-code,
//! which validates and dispatches to the corresponding
//! machine code emitter.

use crate::codegen::CodeGen;
use crate::masm::{MacroAssembler, OperandSize, RegImm};
use crate::stack::Val;
use wasmparser::ValType;
use wasmparser::VisitOperator;

impl<'a, M> CodeGen<'a, M>
where
    M: MacroAssembler,
{
    fn add_imm_i32(&mut self) {
        let val = self
            .context
            .stack
            .pop_i32_const()
            .expect("i32 constant at stack top");
        let reg = self
            .regalloc
            .pop_to_reg(&mut self.context, OperandSize::S32);

        let dst = RegImm::reg(reg);
        self.context
            .masm
            .add(dst, dst, RegImm::imm(val), OperandSize::S32);
        self.context.stack.push(Val::reg(reg));
    }

    fn add_i32(&mut self) {
        let src = self
            .regalloc
            .pop_to_reg(&mut self.context, OperandSize::S32);
        let dst = self
            .regalloc
            .pop_to_reg(&mut self.context, OperandSize::S32);

        let lhs = RegImm::reg(dst);
        self.context
            .masm
            .add(lhs, lhs, RegImm::reg(src), OperandSize::S32);

        self.regalloc.free_gpr(src);
        self.context.stack.push(Val::reg(dst));
    }
}

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
		    $($(drop($arg);)*)?
		    todo!(stringify!($op))
		}
	    );
        )*
    };

    (emit I32Const $($rest:tt)*) => {};
    (emit I32Add $($rest:tt)*) => {};
    (emit LocalGet $($rest:tt)*) => {};
    (emit LocalSet $($rest:tt)*) => {};
    (emit End $($rest:tt)*) => {};

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

    fn visit_i32_add(&mut self) {
        let is_const = self
            .context
            .stack
            .peek()
            .expect("value at stack top")
            .is_i32_const();

        if is_const {
            self.add_imm_i32();
        } else {
            self.add_i32();
        }
    }

    fn visit_end(&mut self) {}

    fn visit_local_get(&mut self, index: u32) {
        let context = &mut self.context;
        let slot = context
            .frame
            .get_local(index)
            .expect(&format!("valid local at slot = {}", index));
        match slot.ty {
            ValType::I32 | ValType::I64 => context.stack.push(Val::local(index)),
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
        let src = self.regalloc.pop_to_reg(context, size);
        let addr = context.masm.local_address(&slot);
        context.masm.store(RegImm::reg(src), addr, size);
        self.regalloc.free_gpr(src);
    }

    wasmparser::for_each_operator!(def_unsupported);
}

impl From<ValType> for OperandSize {
    fn from(ty: ValType) -> OperandSize {
        match ty {
            ValType::I32 => OperandSize::S32,
            ValType::I64 => OperandSize::S64,
            ty => todo!("unsupported type {:?}", ty),
        }
    }
}
