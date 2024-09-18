//! Evaluating const expressions.

use crate::runtime::vm::{Instance, VMGcRef, ValRaw, I31};
use crate::store::AutoAssertNoGc;
use crate::{prelude::*, OpaqueRootScope, StructRef, StructRefPre, StructType, Val};
use smallvec::SmallVec;
use wasmtime_environ::{
    ConstExpr, ConstOp, FuncIndex, GlobalIndex, Module, ModuleInternedTypeIndex, WasmCompositeType,
    WasmSubType,
};

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
            let global = self
                .instance
                .defined_or_imported_global_ptr(index)
                .as_ref()
                .unwrap();
            let mut gc_store = (*self.instance.store()).unwrap_gc_store_mut();
            Ok(global.to_val_raw(&mut gc_store, self.module.globals[index].wasm_ty))
        }
    }

    fn ref_func(&mut self, index: FuncIndex) -> Result<ValRaw> {
        Ok(ValRaw::funcref(
            self.instance.get_func_ref(index).unwrap().cast(),
        ))
    }

    fn struct_fields_len(&self, struct_type_index: ModuleInternedTypeIndex) -> usize {
        let module = self
            .instance
            .runtime_module()
            .expect("should never be allocating a struct type defined in a dummy module");

        let struct_ty = match &module.types()[struct_type_index].composite_type {
            WasmCompositeType::Struct(s) => s,
            _ => unreachable!(),
        };

        struct_ty.fields.len()
    }

    /// Safety: field values must be of the correct types.
    unsafe fn struct_new(
        &mut self,
        struct_type_index: ModuleInternedTypeIndex,
        fields: &[ValRaw],
    ) -> Result<ValRaw> {
        let module = self
            .instance
            .runtime_module()
            .expect("should never be allocating a struct type defined in a dummy module");
        let shared_ty = module
            .signatures()
            .shared_type(struct_type_index)
            .expect("should have an engine type for module type");

        let store = unsafe { (*self.instance.store()).store_opaque_mut() };

        let struct_ty = StructType::from_shared_type_index(store.engine(), shared_ty);
        let fields = fields
            .iter()
            .zip(struct_ty.fields())
            .map(|(raw, ty)| {
                let ty = ty.element_type().unpack();
                let mut store = AutoAssertNoGc::new(store);
                Val::_from_raw(&mut store, *raw, ty)
            })
            .collect::<Vec<_>>();

        let allocator = StructRefPre::_new(store, struct_ty);
        let mut store = OpaqueRootScope::new(store);

        // TODO: if this fails with `GcHeapOutOfMemory<()>`, then we should do a
        // GC and try to allocate again, hoping that the GC freed up
        // space. However, we would need to choose between sync and async GC,
        // and we can't use `AsyncCx::block_on` here to fake it because we
        // aren't guaranteed to be running on a fiber yet, so we would be forced
        // to create sync and async versions of every function up to at least
        // `ConstExprEvaluator::eval`.
        //
        // In the meantime, just give up early without trying a GC if allocation
        // fails.
        let struct_ref = StructRef::_new(&mut store, &allocator, &fields)?;

        let mut store = AutoAssertNoGc::new(&mut store);
        let raw = struct_ref.to_anyref()._to_raw(&mut store)?;
        Ok(ValRaw::anyref(raw))
    }

    fn struct_new_default(&mut self, struct_type_index: ModuleInternedTypeIndex) -> Result<ValRaw> {
        let module = self
            .instance
            .runtime_module()
            .expect("should never be allocating a struct type defined in a dummy module");

        let shared_ty = module
            .signatures()
            .shared_type(struct_type_index)
            .expect("should have an engine type for module type");

        let borrowed = module
            .engine()
            .signatures()
            .borrow(shared_ty)
            .expect("should have a registered type for struct");
        let WasmSubType {
            composite_type: WasmCompositeType::Struct(struct_ty),
            ..
        } = &*borrowed
        else {
            unreachable!("registered type should be a struct");
        };

        let fields = struct_ty
            .fields
            .iter()
            .map(|ty| match &ty.element_type {
                wasmtime_environ::WasmStorageType::I8 | wasmtime_environ::WasmStorageType::I16 => {
                    ValRaw::i32(0)
                }
                wasmtime_environ::WasmStorageType::Val(v) => match v {
                    wasmtime_environ::WasmValType::I32 => ValRaw::i32(0),
                    wasmtime_environ::WasmValType::I64 => ValRaw::i64(0),
                    wasmtime_environ::WasmValType::F32 => ValRaw::f32(0.0f32.to_bits()),
                    wasmtime_environ::WasmValType::F64 => ValRaw::f64(0.0f64.to_bits()),
                    wasmtime_environ::WasmValType::V128 => ValRaw::v128(0),
                    wasmtime_environ::WasmValType::Ref(r) => {
                        assert!(r.nullable);
                        ValRaw::null()
                    }
                },
            })
            .collect::<SmallVec<[_; 8]>>();

        unsafe { self.struct_new(struct_type_index, &fields) }
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
                ConstOp::GlobalGet(g) => self.stack.push(context.global_get(*g)?),
                ConstOp::RefNull => self.stack.push(ValRaw::null()),
                ConstOp::RefFunc(f) => self.stack.push(context.ref_func(*f)?),
                ConstOp::RefI31 => {
                    let i = self.pop()?.get_i32();
                    let i31 = I31::wrapping_i32(i);
                    let raw = VMGcRef::from_i31(i31).as_raw_u32();
                    self.stack.push(ValRaw::anyref(raw));
                }
                ConstOp::I32Add => {
                    let b = self.pop()?.get_i32();
                    let a = self.pop()?.get_i32();
                    self.stack.push(ValRaw::i32(a.wrapping_add(b)));
                }
                ConstOp::I32Sub => {
                    let b = self.pop()?.get_i32();
                    let a = self.pop()?.get_i32();
                    self.stack.push(ValRaw::i32(a.wrapping_sub(b)));
                }
                ConstOp::I32Mul => {
                    let b = self.pop()?.get_i32();
                    let a = self.pop()?.get_i32();
                    self.stack.push(ValRaw::i32(a.wrapping_mul(b)));
                }
                ConstOp::I64Add => {
                    let b = self.pop()?.get_i64();
                    let a = self.pop()?.get_i64();
                    self.stack.push(ValRaw::i64(a.wrapping_add(b)));
                }
                ConstOp::I64Sub => {
                    let b = self.pop()?.get_i64();
                    let a = self.pop()?.get_i64();
                    self.stack.push(ValRaw::i64(a.wrapping_sub(b)));
                }
                ConstOp::I64Mul => {
                    let b = self.pop()?.get_i64();
                    let a = self.pop()?.get_i64();
                    self.stack.push(ValRaw::i64(a.wrapping_mul(b)));
                }
                ConstOp::StructNew { struct_type_index } => {
                    let interned_type_index = context.module.types[*struct_type_index];
                    let len = context.struct_fields_len(interned_type_index);

                    if self.stack.len() < len {
                        bail!(
                            "const expr evaluation error: expected at least {len} values on the stack, found {}",
                            self.stack.len()
                        )
                    }

                    let start = self.stack.len() - len;
                    let s = context.struct_new(interned_type_index, &self.stack[start..])?;
                    self.stack.truncate(start);
                    self.stack.push(s);
                }
                ConstOp::StructNewDefault { struct_type_index } => {
                    let interned_type_index = context.module.types[*struct_type_index];
                    self.stack
                        .push(context.struct_new_default(interned_type_index)?);
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
}
