//! This module is the central place for machine code emission.
//! It is essentially an implementation of `CodeGen`, with `visit` as
//! its single entry point, exclusively responsible for matching each WebAssembly
//! operator to its corresponding machine instruction sequence.
//!
//! The definitions in this file are expected to grow as more
//! WebAssembly operators are supported, so the intention
//! behind having a separate implementation is to avoid bloating
//! the main codegen module.

use crate::codegen::CodeGen;
use crate::masm::{MacroAssembler, OperandSize, RegImm};
use crate::stack::Val;
use anyhow::Result;
use wasmparser::Operator;
use wasmtime_environ::WasmType;

impl<'c, 'a: 'c, M> CodeGen<'a, 'c, M>
where
    M: MacroAssembler,
{
    /// Match each supported WebAssembly operator and dispatch to
    /// the corresponding machine code emitter.
    pub fn visit(&mut self, operator: Operator) -> Result<()> {
        match operator {
            Operator::I32Add => self.emit_i32_add(),
            Operator::I32Const { value } => self.emit_i32_const(value),
            Operator::LocalSet { local_index } => self.emit_local_set(local_index),
            Operator::LocalGet { local_index } => self.emit_local_get(local_index),
            Operator::End => Ok(()),
            op => todo!("Unsupported operator {:?}", op),
        }
    }

    fn emit_i32_add(&mut self) -> Result<()> {
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

        Ok(())
    }

    fn add_imm_i32(&mut self) {
        let val = self
            .context
            .stack
            .pop_i32_const()
            .expect("i32 constant at stack top");
        let reg = self
            .regalloc
            .pop_to_reg(&mut self.context, OperandSize::S32);
        self.context
            .masm
            .add(RegImm::imm(val), RegImm::reg(reg), OperandSize::S32);
        self.context.stack.push(Val::reg(reg));
    }

    fn add_i32(&mut self) {
        let src = self
            .regalloc
            .pop_to_reg(&mut self.context, OperandSize::S32);
        let dst = self
            .regalloc
            .pop_to_reg(&mut self.context, OperandSize::S32);

        self.context
            .masm
            .add(RegImm::reg(src), RegImm::reg(dst), OperandSize::S32);
        self.regalloc.free_gpr(src);
        self.context.stack.push(Val::reg(dst));
    }

    fn emit_i32_const(&mut self, val: i32) -> Result<()> {
        self.context.stack.push(Val::i32(val));
        Ok(())
    }

    fn emit_local_get(&mut self, index: u32) -> Result<()> {
        let context = &mut self.context;
        let slot = context
            .frame
            .get_local(index)
            .expect(&format!("valid local at slot = {}", index));
        match slot.ty {
            WasmType::I32 | WasmType::I64 => context.stack.push(Val::local(index)),
            _ => panic!("Unsupported type {} for local", slot.ty),
        }

        Ok(())
    }

    // TODO verify the case where the target local is on the stack
    fn emit_local_set(&mut self, index: u32) -> Result<()> {
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

        Ok(())
    }
}

impl From<WasmType> for OperandSize {
    fn from(ty: WasmType) -> OperandSize {
        match ty {
            WasmType::I32 => OperandSize::S32,
            WasmType::I64 => OperandSize::S64,
            ty => todo!("unsupported type {}", ty),
        }
    }
}
