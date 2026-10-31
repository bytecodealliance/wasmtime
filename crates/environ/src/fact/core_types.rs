use crate::prelude::*;
use crate::{
    EngineOrModuleTypeIndex, ModuleInternedRecGroupIndex, ModuleInternedTypeIndex, ModuleTypes,
    TypeTrace, WasmCompositeInnerType, WasmFieldType, WasmHeapType, WasmRefType, WasmStorageType,
    WasmSubType, WasmValType,
};
use std::collections::HashMap;
use std::convert::Infallible;
use wasm_encoder::{
    AbstractHeapType, ArrayType, CompositeInnerType, CompositeType, FieldType, FuncType, HeapType,
    RefType, StorageType, StructType, SubType, TypeSection, ValType,
};

/// The rec group currently being encoded, if any, and its members.
type RecGroupCtx<'a> = Option<(ModuleInternedRecGroupIndex, &'a [ModuleInternedTypeIndex])>;

/// A simple representation of the type section which automatically intern's
/// types and ensures they're only defined once.
#[derive(Default)]
pub struct CoreTypes {
    pub section: TypeSection,

    /// The next core type index to hand out. Not `section.len()`, which counts
    /// rec groups rather than types.
    next_ty: u32,

    intern: HashMap<(Vec<ValType>, Vec<ValType>), u32>,

    /// Cloned rec groups, mapped to the adapter-local index of each member, in
    /// order.
    rec_groups: HashMap<ModuleInternedRecGroupIndex, Vec<u32>>,

    /// The inverse of `rec_groups`.
    module_types: HashMap<u32, ModuleInternedTypeIndex>,
}

impl CoreTypes {
    pub fn function(&mut self, params: &[ValType], results: &[ValType]) -> u32 {
        if let Some(idx) = self.intern.get(&(params.to_vec(), results.to_vec())) {
            return *idx;
        }

        let idx = self.push_ty();
        self.section
            .ty()
            .function(params.iter().copied(), results.iter().copied());
        self.intern.insert((params.to_vec(), results.to_vec()), idx);
        idx
    }

    fn push_ty(&mut self) -> u32 {
        let idx = self.next_ty;
        self.next_ty += 1;
        idx
    }

    /// Clone `group` into this adapter module, returning the adapter-local
    /// index of each member, in order.
    pub fn rec_group(&mut self, types: &ModuleTypes, group: ModuleInternedRecGroupIndex) -> &[u32] {
        let mut stack = vec![(group, false)];
        while let Some((group, cloned_deps)) = stack.pop() {
            if self.rec_groups.contains_key(&group) {
                continue;
            }

            let members = types.rec_group_elements(group).collect::<Vec<_>>();

            if !cloned_deps {
                stack.push((group, true));
                let rec_groups = &self.rec_groups;
                for ty in members.iter() {
                    types[*ty]
                        .trace::<_, Infallible>(&mut |idx| {
                            if let EngineOrModuleTypeIndex::Module(idx) = idx {
                                if !members.contains(&idx) {
                                    let dep = types.rec_group_of(idx);
                                    debug_assert!(dep < group);
                                    if !rec_groups.contains_key(&dep) {
                                        stack.push((dep, false));
                                    }
                                }
                            }
                            Ok(())
                        })
                        .unwrap();
                }
                continue;
            }

            // Assign indices before encoding, so references between members of
            // this group resolve.
            let indices = members.iter().map(|_| self.push_ty()).collect::<Vec<_>>();
            for (idx, ty) in indices.iter().zip(members.iter()) {
                self.module_types.insert(*idx, *ty);
            }
            self.rec_groups.insert(group, indices);

            let encoded = members
                .iter()
                .map(|ty| self.sub_type(types, &types[*ty], Some((group, &members))))
                .collect::<Vec<_>>();
            self.section.ty().rec(encoded);
        }

        &self.rec_groups[&group]
    }

    /// The adapter-local index of `ty`, if its group has been cloned.
    fn interned_type_index(&self, types: &ModuleTypes, ty: ModuleInternedTypeIndex) -> Option<u32> {
        let group = types.rec_group_of(ty);
        let indices = self.rec_groups.get(&group)?;
        let position = types
            .rec_group_elements(group)
            .position(|member| member == ty)
            .expect("`rec_group_of` returned a group that does not contain `ty`");
        Some(indices[position])
    }

    /// The component type that adapter-local type `index` was cloned from.
    pub fn module_type_of(&self, index: u32) -> ModuleInternedTypeIndex {
        self.module_types[&index]
    }

    fn sub_type(&self, types: &ModuleTypes, ty: &WasmSubType, ctx: RecGroupCtx<'_>) -> SubType {
        let inner = match &ty.composite_type.inner {
            WasmCompositeInnerType::Struct(s) => CompositeInnerType::Struct(StructType {
                fields: s
                    .fields
                    .iter()
                    .map(|f| self.field_type(types, f, ctx))
                    .collect(),
            }),
            WasmCompositeInnerType::Array(a) => {
                CompositeInnerType::Array(ArrayType(self.field_type(types, &a.0, ctx)))
            }
            WasmCompositeInnerType::Func(f) => CompositeInnerType::Func(FuncType::new(
                f.params().iter().map(|t| self.val_type(types, t, ctx)),
                f.results().iter().map(|t| self.val_type(types, t, ctx)),
            )),

            // No component type's GC lowering uses these, and `Exn` is
            // Wasmtime-internal with no core Wasm encoding.
            WasmCompositeInnerType::Cont(_) | WasmCompositeInnerType::Exn(_) => todo!("CM+GC"),
        };

        SubType {
            is_final: ty.is_final,
            supertype_idxs: ty
                .supertype
                .map(|s| self.type_index(types, s, ctx))
                .into_iter()
                .collect(),
            composite_type: CompositeType {
                inner,
                shared: ty.composite_type.shared,
                descriptor: None,
                describes: None,
            },
        }
    }

    fn field_type(
        &self,
        types: &ModuleTypes,
        ty: &WasmFieldType,
        ctx: RecGroupCtx<'_>,
    ) -> FieldType {
        FieldType {
            element_type: match &ty.element_type {
                WasmStorageType::I8 => StorageType::I8,
                WasmStorageType::I16 => StorageType::I16,
                WasmStorageType::Val(v) => StorageType::Val(self.val_type(types, v, ctx)),
            },
            mutable: ty.mutable,
        }
    }

    fn val_type(&self, types: &ModuleTypes, ty: &WasmValType, ctx: RecGroupCtx<'_>) -> ValType {
        match ty {
            WasmValType::I32 => ValType::I32,
            WasmValType::I64 => ValType::I64,
            WasmValType::F32 => ValType::F32,
            WasmValType::F64 => ValType::F64,
            WasmValType::V128 => ValType::V128,
            WasmValType::Ref(r) => ValType::Ref(self.ref_type(types, r, ctx)),
        }
    }

    pub(super) fn ref_type(
        &self,
        types: &ModuleTypes,
        ty: &WasmRefType,
        ctx: RecGroupCtx<'_>,
    ) -> RefType {
        let abstract_ = |ty| HeapType::Abstract { shared: false, ty };
        let heap_type = match ty.heap_type {
            WasmHeapType::Extern => abstract_(AbstractHeapType::Extern),
            WasmHeapType::NoExtern => abstract_(AbstractHeapType::NoExtern),
            WasmHeapType::Func => abstract_(AbstractHeapType::Func),
            WasmHeapType::NoFunc => abstract_(AbstractHeapType::NoFunc),
            WasmHeapType::Any => abstract_(AbstractHeapType::Any),
            WasmHeapType::Eq => abstract_(AbstractHeapType::Eq),
            WasmHeapType::I31 => abstract_(AbstractHeapType::I31),
            WasmHeapType::Array => abstract_(AbstractHeapType::Array),
            WasmHeapType::Struct => abstract_(AbstractHeapType::Struct),
            WasmHeapType::None => abstract_(AbstractHeapType::None),

            WasmHeapType::ConcreteStruct(i)
            | WasmHeapType::ConcreteArray(i)
            | WasmHeapType::ConcreteFunc(i) => HeapType::Concrete(self.type_index(types, i, ctx)),

            WasmHeapType::Exn
            | WasmHeapType::NoExn
            | WasmHeapType::ConcreteExn(_)
            | WasmHeapType::Cont
            | WasmHeapType::NoCont
            | WasmHeapType::ConcreteCont(_) => todo!("CM+GC"),
        };
        RefType {
            nullable: ty.nullable,
            heap_type,
        }
    }

    /// Translate a type reference from the component's index space into this
    /// adapter module's.
    fn type_index(
        &self,
        types: &ModuleTypes,
        index: EngineOrModuleTypeIndex,
        ctx: RecGroupCtx<'_>,
    ) -> u32 {
        let index = match index {
            EngineOrModuleTypeIndex::Module(i) => i,
            EngineOrModuleTypeIndex::Engine(_) | EngineOrModuleTypeIndex::RecGroup(_) => {
                panic!("adapter modules only reference module-interned types")
            }
        };
        if let Some((group, members)) = ctx {
            if let Some(i) = members.iter().position(|m| *m == index) {
                return self.rec_groups[&group][i];
            }
        }
        self.interned_type_index(types, index)
            .expect("referenced rec groups are declared before the groups that reference them")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WasmCompositeType, WasmStructType};

    fn struct_referencing(field: Option<ModuleInternedTypeIndex>) -> WasmSubType {
        let element_type = match field {
            Some(idx) => WasmStorageType::Val(WasmValType::Ref(WasmRefType {
                nullable: true,
                heap_type: WasmHeapType::ConcreteStruct(EngineOrModuleTypeIndex::Module(idx)),
            })),
            None => WasmStorageType::I8,
        };
        WasmSubType {
            is_final: true,
            supertype: None,
            composite_type: WasmCompositeType {
                inner: WasmCompositeInnerType::Struct(WasmStructType {
                    fields: Box::new([WasmFieldType {
                        element_type,
                        mutable: true,
                    }]),
                }),
                shared: false,
            },
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn deep_rec_group_chain_does_not_overflow_the_stack() {
        const LEN: usize = 100_000;

        let mut types = ModuleTypes::default();
        let mut prev = None;
        for _ in 0..LEN {
            let idx = types.push(struct_referencing(prev));
            types.push_rec_group(idx..types.next_ty());
            prev = Some(idx);
        }

        let last = types.rec_group_of(prev.unwrap());
        let mut core_types = CoreTypes::default();
        assert_eq!(core_types.rec_group(&types, last).len(), 1);

        // The whole chain is pulled in, since each group references the last.
        assert_eq!(core_types.next_ty as usize, LEN);
    }
}
