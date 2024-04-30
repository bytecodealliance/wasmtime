//! Evaluating const expressions.

use crate::runtime::vm::{Instance, VMGcRef, ValRaw, I31};
use anyhow::{anyhow, bail, Result};
use smallvec::SmallVec;
use wasmtime_environ::{ConstExpr, ConstOp, FuncIndex, GlobalIndex, Module};

/// An interpreter for const expressions.
///
/// This can be reused across many const expression evaluations to reuse
/// allocated resources, if any.
#[derive(Default)]
pub struct ConstExprEvaluator {
    stack: SmallVec<[ValRaw; 2]>,
}

/// The context within which a particular const expression is evaluated.
pub struct ConstEvalContext<'a, 'b> {
    instance: &'a mut Instance,
    module: &'b Module,
}

impl<'a, 'b> ConstEvalContext<'a, 'b> {
    /// Create a new context.
    pub fn new(instance: &'a mut Instance, module: &'b Module) -> Self {
        Self { instance, module }
    }

    fn global_get(&mut self, index: GlobalIndex) -> Result<ValRaw> {
        unsafe {
            let gc_store = (*self.instance.store()).gc_store();
            let global = self
                .instance
                .defined_or_imported_global_ptr(index)
                .as_ref()
                .unwrap();
            Ok(global.to_val_raw(gc_store, self.module.globals[index].wasm_ty))
        }
    }

    fn ref_func(&mut self, index: FuncIndex) -> Result<ValRaw> {
        Ok(ValRaw::funcref(
            self.instance.get_func_ref(index).unwrap().cast(),
        ))
    }
}

impl ConstExprEvaluator {
    /// Evaluate the given const expression in the given context.
    ///
    /// # Unsafety
    ///
    /// The given const expression must be valid within the given context,
    /// e.g. the const expression must be well-typed and the context must return
    /// global values of the expected types. This evaluator operates directly on
    /// untyped `ValRaw`s and does not and cannot check that its operands are of
    /// the correct type.
    pub unsafe fn eval(
        &mut self,
        context: &mut ConstEvalContext<'_, '_>,
        expr: &ConstExpr,
    ) -> Result<ValRaw> {
        self.stack.clear();

        for op in expr.ops() {
            match op {
                ConstOp::I32Const(i) => self.stack.push(ValRaw::i32(*i)),
                ConstOp::I64Const(i) => self.stack.push(ValRaw::i64(*i)),
                ConstOp::F32Const(f) => self.stack.push(ValRaw::f32(*f)),
                ConstOp::F64Const(f) => self.stack.push(ValRaw::f64(*f)),
                ConstOp::V128Const(v) => self.stack.push(ValRaw::v128(*v)),
                ConstOp::I32Add => {
                    self.binop(|a, b| ValRaw::i32(a.get_i32().wrapping_add(b.get_i32())))?
                }
                ConstOp::I64Add => {
                    self.binop(|a, b| ValRaw::i64(a.get_i64().wrapping_add(b.get_i64())))?
                }
                ConstOp::I32Sub => {
                    self.binop(|a, b| ValRaw::i32(a.get_i32().wrapping_sub(b.get_i32())))?
                }
                ConstOp::I64Sub => {
                    self.binop(|a, b| ValRaw::i64(a.get_i64().wrapping_sub(b.get_i64())))?
                }
                ConstOp::I32Mul => {
                    self.binop(|a, b| ValRaw::i32(a.get_i32().wrapping_mul(b.get_i32())))?
                }
                ConstOp::I64Mul => {
                    self.binop(|a, b| ValRaw::i64(a.get_i64().wrapping_mul(b.get_i64())))?
                }
                ConstOp::GlobalGet(g) => self.stack.push(context.global_get(*g)?),
                ConstOp::RefNull => self.stack.push(ValRaw::null()),
                ConstOp::RefFunc(f) => self.stack.push(context.ref_func(*f)?),
                ConstOp::RefI31 => {
                    let i = self.pop()?.get_i32();
                    let i31 = I31::wrapping_i32(i);
                    let raw = VMGcRef::from_i31(i31).as_raw_u32();
                    self.stack.push(ValRaw::anyref(raw));
                }
            }
        }

        if self.stack.len() == 1 {
            Ok(self.stack[0])
        } else {
            bail!(
                "const expr evaluation error: expected 1 resulting value, found {}",
                self.stack.len()
            )
        }
    }

    fn pop(&mut self) -> Result<ValRaw> {
        self.stack.pop().ok_or_else(|| {
            anyhow!(
                "const expr evaluation error: attempted to pop from an empty \
                 evaluation stack"
            )
        })
    }

    fn binop(&mut self, f: impl FnOnce(ValRaw, ValRaw) -> ValRaw) -> Result<()> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.stack.push(f(a, b));
        Ok(())
    }
}
