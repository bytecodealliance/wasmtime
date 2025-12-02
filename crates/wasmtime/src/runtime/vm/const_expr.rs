//! Evaluating const expressions.

use crate::prelude::*;
use crate::runtime::vm;
use crate::store::{AutoAssertNoGc, InstanceId, StoreOpaque, StoreResourceLimiter};
#[cfg(feature = "gc")]
use crate::{
    AnyRef, ArrayRef, ArrayRefPre, ArrayType, ExternRef, I31, StructRef, StructRefPre, StructType,
};
use crate::{OpaqueRootScope, Val};
use wasmtime_environ::{ConstExpr, ConstOp, FuncIndex, GlobalIndex};
#[cfg(feature = "gc")]
use wasmtime_environ::{VMSharedTypeIndex, WasmCompositeInnerType, WasmCompositeType, WasmSubType};

/// An interpreter for const expressions.
///
/// This can be reused across many const expression evaluations to reuse
/// allocated resources, if any.
pub struct ConstExprEvaluator {
    stack: Vec<Val>,
    simple: Val,
}

impl Default for ConstExprEvaluator {
    fn default() -> ConstExprEvaluator {
        ConstExprEvaluator {
            stack: Vec::new(),
            simple: Val::I32(0),
        }
    }
}

/// The context within which a particular const expression is evaluated.
pub struct ConstEvalContext {
    pub(crate) instance: InstanceId,
}

impl ConstEvalContext {
    /// Create a new context.
    pub fn new(instance: InstanceId) -> Self {
        Self { instance }
    }

    fn global_get(&mut self, store: &mut StoreOpaque, index: GlobalIndex) -> Result<Val> {
        let id = store.id();
        Ok(store
            .instance_mut(self.instance)
            .get_exported_global(id, index)
            ._get(&mut AutoAssertNoGc::new(store)))
    }

    fn ref_func(&mut self, store: &mut StoreOpaque, index: FuncIndex) -> Result<Val> {
        let id = store.id();
        let (instance, registry) = store.instance_and_module_registry_mut(self.instance);
        // SAFETY: `id` is the correct store-owner of the function being looked
        // up
        let func = unsafe { instance.get_exported_func(registry, id, index) };
        Ok(func.into())
    }

    #[cfg(feature = "gc")]
    fn struct_fields_len(&self, store: &mut StoreOpaque, shared_ty: VMSharedTypeIndex) -> usize {
        let struct_ty = StructType::from_shared_type_index(store.engine(), shared_ty);
        let fields = struct_ty.fields();
        fields.len()
    }

    #[cfg(feature = "gc")]
    async fn struct_new(
        &mut self,
        store: &mut StoreOpaque,
        limiter: Option<&mut StoreResourceLimiter<'_>>,
        shared_ty: VMSharedTypeIndex,
        fields: &[Val],
    ) -> Result<Val> {
        let struct_ty = StructType::from_shared_type_index(store.engine(), shared_ty);
        let allocator = StructRefPre::_new(store, struct_ty);
        let struct_ref = StructRef::_new_async(store, limiter, &allocator, &fields).await?;
        Ok(Val::AnyRef(Some(struct_ref.into())))
    }

    #[cfg(feature = "gc")]
    async fn struct_new_default(
        &mut self,
        store: &mut StoreOpaque,
        limiter: Option<&mut StoreResourceLimiter<'_>>,
        shared_ty: VMSharedTypeIndex,
    ) -> Result<Val> {
        let module = store
            .instance(self.instance)
            .runtime_module()
            .expect("should never be allocating a struct type defined in a dummy module");

        let borrowed = module
            .engine()
            .signatures()
            .borrow(shared_ty)
            .expect("should have a registered type for struct");
        let WasmSubType {
            composite_type:
                WasmCompositeType {
                    shared: false,
                    inner: WasmCompositeInnerType::Struct(struct_ty),
                },
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
                    Val::I32(0)
                }
                wasmtime_environ::WasmStorageType::Val(v) => match v {
                    wasmtime_environ::WasmValType::I32 => Val::I32(0),
                    wasmtime_environ::WasmValType::I64 => Val::I64(0),
                    wasmtime_environ::WasmValType::F32 => Val::F32(0.0f32.to_bits()),
                    wasmtime_environ::WasmValType::F64 => Val::F64(0.0f64.to_bits()),
                    wasmtime_environ::WasmValType::V128 => Val::V128(0u128.into()),
                    wasmtime_environ::WasmValType::Ref(r) => {
                        assert!(r.nullable);
                        Val::null_top(r.heap_type.top())
                    }
                },
            })
            .collect::<smallvec::SmallVec<[_; 8]>>();

        self.struct_new(store, limiter, shared_ty, &fields).await
    }
}

impl ConstExprEvaluator {
    /// Same as [`Self::eval`] except that this is specifically intended for
    /// integral constant expression.
    ///
    /// # Panics
    ///
    /// Panics if `ConstExpr` contains GC ops (e.g. it's not for an integral
    /// type).
    pub fn eval_int(
        &mut self,
        store: &mut StoreOpaque,
        context: &mut ConstEvalContext,
        expr: &ConstExpr,
    ) -> Result<&Val> {
        // Try to evaluate a simple expression first before doing the more
        // complicated eval loop below.
        if self.try_simple(expr).is_some() {
            return Ok(&self.simple);
        }

        // Note that `assert_ready` here should be valid as production of an
        // integer cannot involve GC meaning that async operations aren't used.
        let mut scope = OpaqueRootScope::new(store);
        vm::assert_ready(self.eval_loop(&mut scope, None, context, expr))
    }

    /// Attempts to peek into `expr` to see if it's trivial to evaluate, e.g.
    /// for `i32.const N`.
    #[inline]
    pub fn try_simple(&mut self, expr: &ConstExpr) -> Option<&Val> {
        match expr.ops() {
            [ConstOp::I32Const(i)] => Some(self.return_one(Val::I32(*i))),
            [ConstOp::I64Const(i)] => Some(self.return_one(Val::I64(*i))),
            [ConstOp::F32Const(f)] => Some(self.return_one(Val::F32(*f))),
            [ConstOp::F64Const(f)] => Some(self.return_one(Val::F64(*f))),
            _ => None,
        }
    }

    /// Evaluate the given const expression in the given context.
    ///
    /// Note that the `store` argument is an `OpaqueRootScope` which is used to
    /// require that a GC rooting scope external to evaluation of this constant
    /// is required. Constant expression evaluation may perform GC allocations
    /// and itself trigger a GC meaning that all references must be rooted,
    /// hence the external requirement of a rooting scope.
    ///
    /// # Panics
    ///
    /// This function will panic if `expr` is an invalid constant expression.
    pub async fn eval(
        &mut self,
        store: &mut OpaqueRootScope<&mut StoreOpaque>,
        limiter: Option<&mut StoreResourceLimiter<'_>>,
        context: &mut ConstEvalContext,
        expr: &ConstExpr,
    ) -> Result<&Val> {
        // Same structure as `eval_int` above, except using `.await` and with a
        // slightly different type signature here for this function.
        if self.try_simple(expr).is_some() {
            return Ok(&self.simple);
        }
        self.eval_loop(store, limiter, context, expr).await
    }

    #[inline]
    fn return_one(&mut self, val: Val) -> &Val {
        self.simple = val;
        &self.simple
        // self.stack.clear();
        // self.stack.push(val);
        // &self.stack[0]
    }

    #[cold]
    async fn eval_loop(
        &mut self,
        store: &mut OpaqueRootScope<&mut StoreOpaque>,
        mut limiter: Option<&mut StoreResourceLimiter<'_>>,
        context: &mut ConstEvalContext,
        expr: &ConstExpr,
    ) -> Result<&Val> {
        log::trace!("evaluating const expr: {expr:?}");

        self.stack.clear();

        // On GC-less builds ensure that this is always considered used an
        // needed-mutable.
        let _ = &mut limiter;

        for op in expr.ops() {
            log::trace!("const-evaluating op: {op:?}");
            match op {
                ConstOp::I32Const(i) => self.stack.push(Val::I32(*i)),
                ConstOp::I64Const(i) => self.stack.push(Val::I64(*i)),
                ConstOp::F32Const(f) => self.stack.push(Val::F32(*f)),
                ConstOp::F64Const(f) => self.stack.push(Val::F64(*f)),
                ConstOp::V128Const(v) => self.stack.push(Val::V128((*v).into())),
                ConstOp::GlobalGet(g) => self.stack.push(context.global_get(store, *g)?),
                ConstOp::RefNull(ty) => self.stack.push(Val::null_top(*ty)),
                ConstOp::RefFunc(f) => self.stack.push(context.ref_func(store, *f)?),
                #[cfg(feature = "gc")]
                ConstOp::RefI31 => {
                    let i = self.pop()?.unwrap_i32();
                    let i31 = I31::wrapping_i32(i);
                    let r = AnyRef::_from_i31(&mut AutoAssertNoGc::new(store), i31);
                    self.stack.push(Val::AnyRef(Some(r)));
                }
                #[cfg(not(feature = "gc"))]
                ConstOp::RefI31 => panic!("should not have validated"),
                ConstOp::I32Add => {
                    let b = self.pop()?.unwrap_i32();
                    let a = self.pop()?.unwrap_i32();
                    self.stack.push(Val::I32(a.wrapping_add(b)));
                }
                ConstOp::I32Sub => {
                    let b = self.pop()?.unwrap_i32();
                    let a = self.pop()?.unwrap_i32();
                    self.stack.push(Val::I32(a.wrapping_sub(b)));
                }
                ConstOp::I32Mul => {
                    let b = self.pop()?.unwrap_i32();
                    let a = self.pop()?.unwrap_i32();
                    self.stack.push(Val::I32(a.wrapping_mul(b)));
                }
                ConstOp::I64Add => {
                    let b = self.pop()?.unwrap_i64();
                    let a = self.pop()?.unwrap_i64();
                    self.stack.push(Val::I64(a.wrapping_add(b)));
                }
                ConstOp::I64Sub => {
                    let b = self.pop()?.unwrap_i64();
                    let a = self.pop()?.unwrap_i64();
                    self.stack.push(Val::I64(a.wrapping_sub(b)));
                }
                ConstOp::I64Mul => {
                    let b = self.pop()?.unwrap_i64();
                    let a = self.pop()?.unwrap_i64();
                    self.stack.push(Val::I64(a.wrapping_mul(b)));
                }

                #[cfg(not(feature = "gc"))]
                ConstOp::StructNew { .. }
                | ConstOp::StructNewDefault { .. }
                | ConstOp::ArrayNew { .. }
                | ConstOp::ArrayNewDefault { .. }
                | ConstOp::ArrayNewFixed { .. }
                | ConstOp::ExternConvertAny
                | ConstOp::AnyConvertExtern => {
                    bail!(
                        "const expr evaluation error: struct operations are not \
                         supported without the `gc` feature"
                    )
                }

                #[cfg(feature = "gc")]
                ConstOp::StructNew { struct_type_index } => {
                    let interned_type_index = store.instance(context.instance).env_module().types
                        [*struct_type_index]
                        .unwrap_engine_type_index();
                    let len = context.struct_fields_len(store, interned_type_index);

                    if self.stack.len() < len {
                        bail!(
                            "const expr evaluation error: expected at least {len} values on the stack, found {}",
                            self.stack.len()
                        )
                    }

                    let start = self.stack.len() - len;
                    let s = context
                        .struct_new(
                            store,
                            limiter.as_deref_mut(),
                            interned_type_index,
                            &self.stack[start..],
                        )
                        .await?;
                    self.stack.truncate(start);
                    self.stack.push(s);
                }

                #[cfg(feature = "gc")]
                ConstOp::StructNewDefault { struct_type_index } => {
                    let ty = store.instance(context.instance).env_module().types
                        [*struct_type_index]
                        .unwrap_engine_type_index();
                    self.stack.push(
                        context
                            .struct_new_default(store, limiter.as_deref_mut(), ty)
                            .await?,
                    );
                }

                #[cfg(feature = "gc")]
                ConstOp::ArrayNew { array_type_index } => {
                    let ty = store.instance(context.instance).env_module().types[*array_type_index]
                        .unwrap_engine_type_index();
                    let ty = ArrayType::from_shared_type_index(store.engine(), ty);

                    let len = self.pop()?.unwrap_i32().cast_unsigned();

                    let elem = self.pop()?;

                    let pre = ArrayRefPre::_new(store, ty);
                    let array =
                        ArrayRef::_new_async(store, limiter.as_deref_mut(), &pre, &elem, len)
                            .await?;

                    self.stack.push(Val::AnyRef(Some(array.into())));
                }

                #[cfg(feature = "gc")]
                ConstOp::ArrayNewDefault { array_type_index } => {
                    let ty = store.instance(context.instance).env_module().types[*array_type_index]
                        .unwrap_engine_type_index();
                    let ty = ArrayType::from_shared_type_index(store.engine(), ty);

                    let len = self.pop()?.unwrap_i32().cast_unsigned();

                    let elem = Val::default_for_ty(ty.element_type().unpack())
                        .expect("type should have a default value");

                    let pre = ArrayRefPre::_new(store, ty);
                    let array =
                        ArrayRef::_new_async(store, limiter.as_deref_mut(), &pre, &elem, len)
                            .await?;

                    self.stack.push(Val::AnyRef(Some(array.into())));
                }

                #[cfg(feature = "gc")]
                ConstOp::ArrayNewFixed {
                    array_type_index,
                    array_size,
                } => {
                    let ty = store.instance(context.instance).env_module().types[*array_type_index]
                        .unwrap_engine_type_index();
                    let ty = ArrayType::from_shared_type_index(store.engine(), ty);

                    let array_size = usize::try_from(*array_size).unwrap();
                    if self.stack.len() < array_size {
                        bail!(
                            "const expr evaluation error: expected at least {array_size} values on the stack, found {}",
                            self.stack.len()
                        )
                    }

                    let start = self.stack.len() - array_size;

                    let elems = self
                        .stack
                        .drain(start..)
                        .collect::<smallvec::SmallVec<[_; 8]>>();

                    let pre = ArrayRefPre::_new(store, ty);
                    let array =
                        ArrayRef::_new_fixed_async(store, limiter.as_deref_mut(), &pre, &elems)
                            .await?;

                    self.stack.push(Val::AnyRef(Some(array.into())));
                }

                #[cfg(feature = "gc")]
                ConstOp::ExternConvertAny => {
                    let mut store = AutoAssertNoGc::new(store);
                    let result = match self.pop()?.unwrap_anyref() {
                        Some(anyref) => Some(ExternRef::_convert_any(&mut store, *anyref)?),
                        None => None,
                    };
                    self.stack.push(Val::ExternRef(result));
                }

                #[cfg(feature = "gc")]
                ConstOp::AnyConvertExtern => {
                    let mut store = AutoAssertNoGc::new(store);
                    let result = match self.pop()?.unwrap_externref() {
                        Some(externref) => Some(AnyRef::_convert_extern(&mut store, *externref)?),
                        None => None,
                    };
                    self.stack.push(result.into());
                }
            }
        }

        if self.stack.len() == 1 {
            log::trace!("const expr evaluated to {:?}", self.stack[0]);
            Ok(&self.stack[0])
        } else {
            bail!(
                "const expr evaluation error: expected 1 resulting value, found {}",
                self.stack.len()
            )
        }
    }

    fn pop(&mut self) -> Result<Val> {
        self.stack.pop().ok_or_else(|| {
            anyhow!(
                "const expr evaluation error: attempted to pop from an empty \
                 evaluation stack"
            )
        })
    }
}
