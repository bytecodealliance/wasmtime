//! Embedder API for defining recursion groups of Wasm types.
//!
//! The one-off constructors `StructType::new`, `ArrayType::new`, and
//! `FuncType::new` cannot describe types that reference themselves or each
//! other, because the constructors require all referenced types to already
//! exist. [`RecGroupBuilder`] lifts that restriction: you *declare* a type to
//! get a kind-typed label (a [`PendingStructId`], [`PendingArrayId`], or
//! [`PendingFuncId`]), use that label as a forward reference while defining
//! other types, and then *define* it later. The whole group is validated and
//! registered together when [`RecGroupBuilder::build`] is called.
//!
//! ```
//! # use wasmtime::*;
//! # fn main() -> Result<()> {
//! let engine = Engine::default();
//!
//! // Two mutually-recursive struct types.
//! let mut builder = RecGroupBuilder::new(&engine);
//! let s1 = builder.declare_struct();
//! let s2 = builder.declare_struct();
//! builder.define_struct(s1, [FieldTemplate::ref_(Mutability::Var, true, s2)]);
//! builder.define_struct(s2, [FieldTemplate::ref_(Mutability::Const, false, s1)]);
//! let group = builder.build()?;
//!
//! let s1: StructType = group.struct_(s1);
//! let s2: StructType = group.struct_(s2);
//! assert!(s1.field(0).unwrap().element_type().is_val_type());
//! # Ok(())
//! # }
//! ```

use crate::prelude::*;
use crate::type_registry::RegisteredType;
use crate::{
    ArrayType, Engine, FieldType, Finality, FuncType, HeapType, Mutability, StorageType,
    StructType, ValType,
};
use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use wasmtime_environ::{
    EngineOrModuleTypeIndex, EntityRef, ModuleInternedTypeIndex, WasmArrayType,
    WasmCompositeInnerType, WasmCompositeType, WasmFieldType, WasmFuncType, WasmHeapType,
    WasmRefType, WasmStorageType, WasmStructType, WasmSubType, WasmValType,
};

/// Maximum number of fields in a struct, mirroring `StructType::from_wasm_struct_type`.
const MAX_FIELDS: usize = 10_000;

/// A process-global counter used to give each [`RecGroupBuilder`] a distinct id
/// so that labels from one builder cannot be accidentally used with another.
static NEXT_BUILDER_ID: AtomicUsize = AtomicUsize::new(0);

fn next_builder_id() -> usize {
    NEXT_BUILDER_ID.fetch_add(1, Relaxed)
}

/// The 0-based index of a member within the rec group being built, as a
/// module-level type reference (the form `register_rec_group` expects for
/// intra-group references).
fn module_index(index: u32) -> EngineOrModuleTypeIndex {
    EngineOrModuleTypeIndex::Module(ModuleInternedTypeIndex::new(index as usize))
}

/// A kind-typed label for a struct type being defined in a [`RecGroupBuilder`].
///
/// Obtained from [`RecGroupBuilder::declare_struct`] and used both to define the
/// type and to forward-reference it from other types in the same group.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PendingStructId {
    builder_id: usize,
    index: u32,
}

/// A kind-typed label for an array type being defined in a [`RecGroupBuilder`].
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PendingArrayId {
    builder_id: usize,
    index: u32,
}

/// A kind-typed label for a function type being defined in a [`RecGroupBuilder`].
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PendingFuncId {
    builder_id: usize,
    index: u32,
}

/// A heap type usable while building a recursion group.
///
/// This mirrors [`HeapType`], with one extra variant per kind for forward
/// references to sibling types being defined in the same [`RecGroupBuilder`].
///
/// Already-known heap types (abstract heap types like `any`, or
/// already-registered concrete types) convert into this type via `From`/`Into`,
/// so the common case requires no ceremony.
#[derive(Clone, Debug)]
pub enum HeapTypeTemplate {
    /// An abstract heap type or an already-registered concrete heap type.
    Type(HeapType),
    /// A forward reference to a struct defined in the same rec group.
    LocalStruct(PendingStructId),
    /// A forward reference to an array defined in the same rec group.
    LocalArray(PendingArrayId),
    /// A forward reference to a function defined in the same rec group.
    LocalFunc(PendingFuncId),
}

impl From<HeapType> for HeapTypeTemplate {
    fn from(ty: HeapType) -> Self {
        HeapTypeTemplate::Type(ty)
    }
}
impl From<StructType> for HeapTypeTemplate {
    fn from(ty: StructType) -> Self {
        HeapTypeTemplate::Type(HeapType::ConcreteStruct(ty))
    }
}
impl From<ArrayType> for HeapTypeTemplate {
    fn from(ty: ArrayType) -> Self {
        HeapTypeTemplate::Type(HeapType::ConcreteArray(ty))
    }
}
impl From<FuncType> for HeapTypeTemplate {
    fn from(ty: FuncType) -> Self {
        HeapTypeTemplate::Type(HeapType::ConcreteFunc(ty))
    }
}
impl From<PendingStructId> for HeapTypeTemplate {
    fn from(id: PendingStructId) -> Self {
        HeapTypeTemplate::LocalStruct(id)
    }
}
impl From<PendingArrayId> for HeapTypeTemplate {
    fn from(id: PendingArrayId) -> Self {
        HeapTypeTemplate::LocalArray(id)
    }
}
impl From<PendingFuncId> for HeapTypeTemplate {
    fn from(id: PendingFuncId) -> Self {
        HeapTypeTemplate::LocalFunc(id)
    }
}

/// A value type usable while building a recursion group.
///
/// Mirrors [`ValType`]; the only thing it can express that a [`ValType`] cannot
/// is a `ref` whose target is a sibling type being defined in the same group.
#[derive(Clone, Debug)]
pub enum ValTypeTemplate {
    /// A scalar value type or an already-known concrete reference type.
    Type(ValType),
    /// A `ref` (nullable or not) whose target may be a forward reference.
    Ref {
        /// Whether the reference is nullable.
        nullable: bool,
        /// The referenced heap type.
        heap: HeapTypeTemplate,
    },
}

impl ValTypeTemplate {
    /// Construct a `ref` value type whose target may be a sibling type in the
    /// same rec group.
    pub fn ref_(nullable: bool, heap: impl Into<HeapTypeTemplate>) -> Self {
        ValTypeTemplate::Ref {
            nullable,
            heap: heap.into(),
        }
    }
}

impl From<ValType> for ValTypeTemplate {
    fn from(ty: ValType) -> Self {
        ValTypeTemplate::Type(ty)
    }
}

/// The storage type of a struct field or array element while building a
/// recursion group.
///
/// Mirrors [`StorageType`]; the only thing it can express that a [`StorageType`]
/// cannot is a `ref` whose target is a sibling type being defined in the same
/// group.
#[derive(Clone, Debug)]
pub enum StorageTypeTemplate {
    /// A packed integer storage type or an already-known value type.
    Type(StorageType),
    /// A `ref` (nullable or not) whose target may be a forward reference.
    Ref {
        /// Whether the reference is nullable.
        nullable: bool,
        /// The referenced heap type.
        heap: HeapTypeTemplate,
    },
}

impl From<StorageType> for StorageTypeTemplate {
    fn from(ty: StorageType) -> Self {
        StorageTypeTemplate::Type(ty)
    }
}
impl From<ValType> for StorageTypeTemplate {
    fn from(ty: ValType) -> Self {
        StorageTypeTemplate::Type(StorageType::ValType(ty))
    }
}

/// A struct field or array element type while building a recursion group.
///
/// Mirrors [`FieldType`]. A plain [`FieldType`] (with no forward references)
/// converts into this type via `From`/`Into`.
#[derive(Clone, Debug)]
pub struct FieldTemplate {
    mutability: Mutability,
    element: StorageTypeTemplate,
}

impl FieldTemplate {
    /// Construct a field template from a mutability and element storage type.
    pub fn new(mutability: Mutability, element: impl Into<StorageTypeTemplate>) -> Self {
        FieldTemplate {
            mutability,
            element: element.into(),
        }
    }

    /// Construct a field template whose element is a `ref` that may forward-
    /// reference a sibling type in the same rec group.
    pub fn ref_(mutability: Mutability, nullable: bool, heap: impl Into<HeapTypeTemplate>) -> Self {
        FieldTemplate {
            mutability,
            element: StorageTypeTemplate::Ref {
                nullable,
                heap: heap.into(),
            },
        }
    }
}

impl From<FieldType> for FieldTemplate {
    fn from(ty: FieldType) -> Self {
        FieldTemplate {
            mutability: ty.mutability(),
            element: StorageTypeTemplate::Type(ty.element_type().clone()),
        }
    }
}

/// The supertype of a struct type being defined in a [`RecGroupBuilder`].
///
/// May be either a sibling label ([`PendingStructId`]) or an already-registered
/// [`StructType`]; both convert into this type via `From`/`Into`.
#[derive(Clone, Debug)]
pub enum StructSuperType {
    /// A supertype defined as a sibling in the same rec group.
    Local(PendingStructId),
    /// An already-registered supertype.
    Type(StructType),
}

impl From<PendingStructId> for StructSuperType {
    fn from(id: PendingStructId) -> Self {
        StructSuperType::Local(id)
    }
}
impl From<StructType> for StructSuperType {
    fn from(ty: StructType) -> Self {
        StructSuperType::Type(ty)
    }
}

/// The supertype of an array type being defined in a [`RecGroupBuilder`].
///
/// May be either a sibling label ([`PendingArrayId`]) or an already-registered
/// [`ArrayType`]; both convert into this type via `From`/`Into`.
#[derive(Clone, Debug)]
pub enum ArraySuperType {
    /// A supertype defined as a sibling in the same rec group.
    Local(PendingArrayId),
    /// An already-registered supertype.
    Type(ArrayType),
}

impl From<PendingArrayId> for ArraySuperType {
    fn from(id: PendingArrayId) -> Self {
        ArraySuperType::Local(id)
    }
}
impl From<ArrayType> for ArraySuperType {
    fn from(ty: ArrayType) -> Self {
        ArraySuperType::Type(ty)
    }
}

/// The supertype of a function type being defined in a [`RecGroupBuilder`].
///
/// May be either a sibling label ([`PendingFuncId`]) or an already-registered
/// [`FuncType`]; both convert into this type via `From`/`Into`.
#[derive(Clone, Debug)]
pub enum FuncSuperType {
    /// A supertype defined as a sibling in the same rec group.
    Local(PendingFuncId),
    /// An already-registered supertype.
    Type(FuncType),
}

impl From<PendingFuncId> for FuncSuperType {
    fn from(id: PendingFuncId) -> Self {
        FuncSuperType::Local(id)
    }
}
impl From<FuncType> for FuncSuperType {
    fn from(ty: FuncType) -> Self {
        FuncSuperType::Type(ty)
    }
}

/// One of the concrete composite types in a registered [`RecGroup`].
#[derive(Clone, Debug)]
pub enum CompositeType {
    /// A struct type.
    Struct(StructType),
    /// An array type.
    Array(ArrayType),
    /// A function type.
    Func(FuncType),
}

/// The in-progress definition of one member of a rec group.
enum MemberDef {
    Struct {
        finality: Finality,
        supertype: Option<StructSuperType>,
        fields: Vec<FieldTemplate>,
    },
    Array {
        finality: Finality,
        supertype: Option<ArraySuperType>,
        field: FieldTemplate,
    },
    Func {
        finality: Finality,
        supertype: Option<FuncSuperType>,
        params: Vec<ValTypeTemplate>,
        results: Vec<ValTypeTemplate>,
    },
}

/// A builder for defining a recursion group of Wasm types, including types that
/// reference themselves or each other.
///
/// See the [module-level documentation](crate::RecGroupBuilder) for an overview
/// and examples.
pub struct RecGroupBuilder {
    engine: Engine,
    builder_id: usize,
    members: Vec<Option<MemberDef>>,
}

impl RecGroupBuilder {
    /// Create a new, empty rec group builder associated with the given engine.
    pub fn new(engine: &Engine) -> Self {
        RecGroupBuilder {
            engine: engine.clone(),
            builder_id: next_builder_id(),
            members: Vec::new(),
        }
    }

    fn declare(&mut self) -> u32 {
        let index = u32::try_from(self.members.len()).expect("too many types in a rec group");
        self.members.push(None);
        index
    }

    /// Declare a struct type, returning a label that can be used as a forward
    /// reference before the type is defined via [`define_struct`][Self::define_struct].
    pub fn declare_struct(&mut self) -> PendingStructId {
        PendingStructId {
            builder_id: self.builder_id,
            index: self.declare(),
        }
    }

    /// Declare an array type, returning a label that can be used as a forward
    /// reference before the type is defined via [`define_array`][Self::define_array].
    pub fn declare_array(&mut self) -> PendingArrayId {
        PendingArrayId {
            builder_id: self.builder_id,
            index: self.declare(),
        }
    }

    /// Declare a function type, returning a label that can be used as a forward
    /// reference before the type is defined via [`define_func`][Self::define_func].
    pub fn declare_func(&mut self) -> PendingFuncId {
        PendingFuncId {
            builder_id: self.builder_id,
            index: self.declare(),
        }
    }

    #[track_caller]
    fn check_owns_struct(&self, id: PendingStructId) {
        assert_eq!(
            id.builder_id, self.builder_id,
            "`PendingStructId` used with a different `RecGroupBuilder` than it came from"
        );
    }
    #[track_caller]
    fn check_owns_array(&self, id: PendingArrayId) {
        assert_eq!(
            id.builder_id, self.builder_id,
            "`PendingArrayId` used with a different `RecGroupBuilder` than it came from"
        );
    }
    #[track_caller]
    fn check_owns_func(&self, id: PendingFuncId) {
        assert_eq!(
            id.builder_id, self.builder_id,
            "`PendingFuncId` used with a different `RecGroupBuilder` than it came from"
        );
    }

    /// Define a previously-declared struct type, with the given fields, as a
    /// final type without a supertype.
    pub fn define_struct(
        &mut self,
        id: PendingStructId,
        fields: impl IntoIterator<Item = impl Into<FieldTemplate>>,
    ) -> &mut Self {
        self.define_struct_with_finality_and_supertype(
            id,
            Finality::Final,
            None::<StructSuperType>,
            fields,
        )
    }

    /// Define a previously-declared struct type with the given finality,
    /// supertype, and fields.
    ///
    /// The supertype may be either a sibling label ([`PendingStructId`]) or an
    /// already-registered [`StructType`].
    pub fn define_struct_with_finality_and_supertype(
        &mut self,
        id: PendingStructId,
        finality: Finality,
        supertype: Option<impl Into<StructSuperType>>,
        fields: impl IntoIterator<Item = impl Into<FieldTemplate>>,
    ) -> &mut Self {
        self.check_owns_struct(id);
        self.members[id.index as usize] = Some(MemberDef::Struct {
            finality,
            supertype: supertype.map(Into::into),
            fields: fields.into_iter().map(Into::into).collect(),
        });
        self
    }

    /// Declare and define a final struct type with no supertype, returning its
    /// label.
    pub fn add_struct(
        &mut self,
        fields: impl IntoIterator<Item = impl Into<FieldTemplate>>,
    ) -> PendingStructId {
        let id = self.declare_struct();
        self.define_struct(id, fields);
        id
    }

    /// Define a previously-declared array type, with the given element type, as
    /// a final type without a supertype.
    pub fn define_array(
        &mut self,
        id: PendingArrayId,
        field: impl Into<FieldTemplate>,
    ) -> &mut Self {
        self.define_array_with_finality_and_supertype(
            id,
            Finality::Final,
            None::<ArraySuperType>,
            field,
        )
    }

    /// Define a previously-declared array type with the given finality,
    /// supertype, and element type.
    pub fn define_array_with_finality_and_supertype(
        &mut self,
        id: PendingArrayId,
        finality: Finality,
        supertype: Option<impl Into<ArraySuperType>>,
        field: impl Into<FieldTemplate>,
    ) -> &mut Self {
        self.check_owns_array(id);
        self.members[id.index as usize] = Some(MemberDef::Array {
            finality,
            supertype: supertype.map(Into::into),
            field: field.into(),
        });
        self
    }

    /// Declare and define a final array type with no supertype, returning its
    /// label.
    pub fn add_array(&mut self, field: impl Into<FieldTemplate>) -> PendingArrayId {
        let id = self.declare_array();
        self.define_array(id, field);
        id
    }

    /// Define a previously-declared function type, with the given parameters and
    /// results, as a final type without a supertype.
    pub fn define_func(
        &mut self,
        id: PendingFuncId,
        params: impl IntoIterator<Item = impl Into<ValTypeTemplate>>,
        results: impl IntoIterator<Item = impl Into<ValTypeTemplate>>,
    ) -> &mut Self {
        self.define_func_with_finality_and_supertype(
            id,
            Finality::Final,
            None::<FuncSuperType>,
            params,
            results,
        )
    }

    /// Define a previously-declared function type with the given finality,
    /// supertype, parameters, and results.
    pub fn define_func_with_finality_and_supertype(
        &mut self,
        id: PendingFuncId,
        finality: Finality,
        supertype: Option<impl Into<FuncSuperType>>,
        params: impl IntoIterator<Item = impl Into<ValTypeTemplate>>,
        results: impl IntoIterator<Item = impl Into<ValTypeTemplate>>,
    ) -> &mut Self {
        self.check_owns_func(id);
        self.members[id.index as usize] = Some(MemberDef::Func {
            finality,
            supertype: supertype.map(Into::into),
            params: params.into_iter().map(Into::into).collect(),
            results: results.into_iter().map(Into::into).collect(),
        });
        self
    }

    /// Declare and define a final function type with no supertype, returning its
    /// label.
    pub fn add_func(
        &mut self,
        params: impl IntoIterator<Item = impl Into<ValTypeTemplate>>,
        results: impl IntoIterator<Item = impl Into<ValTypeTemplate>>,
    ) -> PendingFuncId {
        let id = self.declare_func();
        self.define_func(id, params, results);
        id
    }

    /// Finish building the rec group: validate all of its types, register them
    /// with the engine, and return the resulting [`RecGroup`].
    ///
    /// Returns an error if any declared type was never defined, if any type
    /// references a type from a different engine, if a supertype is final, if a
    /// type does not structurally match its declared supertype, or if a struct
    /// exceeds the implementation's field-count limit.
    pub fn build(self) -> Result<RecGroup> {
        let engine = &self.engine;

        ensure!(
            !self.members.is_empty(),
            "a rec group must contain at least one type"
        );

        // 1. Every declared label must have been defined.
        let mut defs = Vec::with_capacity(self.members.len());
        for (i, member) in self.members.iter().enumerate() {
            match member {
                Some(def) => defs.push(def),
                None => bail!("type {i} was declared but never defined"),
            }
        }

        // 2. Lower each member to a `WasmSubType`, checking engine ownership and
        //    supertype finality as we go.
        let mut sub_types = Vec::with_capacity(defs.len());
        for def in &defs {
            sub_types.push(self.lower_member(def)?);
        }

        // 3. Register the whole group with the engine.
        let registered = engine.register_rec_group_types(sub_types.into_iter())?;

        let group = RecGroup {
            builder_id: self.builder_id,
            types: registered,
        };

        // 4. Now that everything is registered (so that sibling references are
        //    real, resolvable types), validate that each type structurally
        //    matches its declared supertype. On failure, `group` is dropped,
        //    which unregisters the types.
        for (i, def) in defs.iter().enumerate() {
            group.validate_supertype(i, def)?;
        }

        Ok(group)
    }

    /// Look up the declared finality of a sibling member by index.
    fn member_finality(&self, index: u32) -> Finality {
        match self.members[index as usize]
            .as_ref()
            .expect("all members defined by this point")
        {
            MemberDef::Struct { finality, .. }
            | MemberDef::Array { finality, .. }
            | MemberDef::Func { finality, .. } => *finality,
        }
    }

    fn lower_member(&self, def: &MemberDef) -> Result<WasmSubType> {
        let (is_final, supertype, inner) = match def {
            MemberDef::Struct {
                finality,
                supertype,
                fields,
            } => {
                ensure!(
                    fields.len() <= MAX_FIELDS,
                    "attempted to define a struct type with {} fields, but that is more than the \
                     maximum supported number of fields ({MAX_FIELDS})",
                    fields.len(),
                );
                let supertype = self.lower_struct_supertype(supertype.as_ref())?;
                let fields = fields
                    .iter()
                    .map(|f| self.lower_field(f))
                    .collect::<Result<Vec<_>>>()?;
                (
                    finality.is_final(),
                    supertype,
                    WasmCompositeInnerType::Struct(WasmStructType {
                        fields: fields.into(),
                    }),
                )
            }
            MemberDef::Array {
                finality,
                supertype,
                field,
            } => {
                let supertype = self.lower_array_supertype(supertype.as_ref())?;
                let field = self.lower_field(field)?;
                (
                    finality.is_final(),
                    supertype,
                    WasmCompositeInnerType::Array(WasmArrayType(field)),
                )
            }
            MemberDef::Func {
                finality,
                supertype,
                params,
                results,
            } => {
                let supertype = self.lower_func_supertype(supertype.as_ref())?;
                let params = params
                    .iter()
                    .map(|p| self.lower_val(p))
                    .collect::<Result<Vec<_>>>()?;
                let results = results
                    .iter()
                    .map(|r| self.lower_val(r))
                    .collect::<Result<Vec<_>>>()?;
                let func = WasmFuncType::new(params, results)?;
                (
                    finality.is_final(),
                    supertype,
                    WasmCompositeInnerType::Func(func),
                )
            }
        };

        Ok(WasmSubType {
            is_final,
            supertype,
            composite_type: WasmCompositeType {
                shared: false,
                inner,
            },
        })
    }

    fn lower_struct_supertype(
        &self,
        supertype: Option<&StructSuperType>,
    ) -> Result<Option<EngineOrModuleTypeIndex>> {
        Ok(match supertype {
            None => None,
            Some(StructSuperType::Local(id)) => {
                self.check_owns_struct(*id);
                ensure!(
                    self.member_finality(id.index).is_non_final(),
                    "cannot create a subtype of a final supertype"
                );
                Some(module_index(id.index))
            }
            Some(StructSuperType::Type(ty)) => Some(self.lower_concrete_supertype(
                ty.comes_from_same_engine(&self.engine),
                ty.finality(),
                ty.type_index(),
            )?),
        })
    }

    fn lower_array_supertype(
        &self,
        supertype: Option<&ArraySuperType>,
    ) -> Result<Option<EngineOrModuleTypeIndex>> {
        Ok(match supertype {
            None => None,
            Some(ArraySuperType::Local(id)) => {
                self.check_owns_array(*id);
                ensure!(
                    self.member_finality(id.index).is_non_final(),
                    "cannot create a subtype of a final supertype"
                );
                Some(module_index(id.index))
            }
            Some(ArraySuperType::Type(ty)) => Some(self.lower_concrete_supertype(
                ty.comes_from_same_engine(&self.engine),
                ty.finality(),
                ty.type_index(),
            )?),
        })
    }

    fn lower_func_supertype(
        &self,
        supertype: Option<&FuncSuperType>,
    ) -> Result<Option<EngineOrModuleTypeIndex>> {
        Ok(match supertype {
            None => None,
            Some(FuncSuperType::Local(id)) => {
                self.check_owns_func(*id);
                ensure!(
                    self.member_finality(id.index).is_non_final(),
                    "cannot create a subtype of a final supertype"
                );
                Some(module_index(id.index))
            }
            Some(FuncSuperType::Type(ty)) => Some(self.lower_concrete_supertype(
                ty.comes_from_same_engine(&self.engine),
                ty.finality(),
                ty.type_index(),
            )?),
        })
    }

    fn lower_concrete_supertype(
        &self,
        same_engine: bool,
        finality: Finality,
        index: wasmtime_environ::VMSharedTypeIndex,
    ) -> Result<EngineOrModuleTypeIndex> {
        ensure!(
            same_engine,
            "supertype is associated with a different engine"
        );
        ensure!(
            finality.is_non_final(),
            "cannot create a subtype of a final supertype"
        );
        Ok(EngineOrModuleTypeIndex::Engine(index))
    }

    fn lower_field(&self, field: &FieldTemplate) -> Result<WasmFieldType> {
        Ok(WasmFieldType {
            element_type: self.lower_storage(&field.element)?,
            mutable: field.mutability.is_var(),
        })
    }

    fn lower_storage(&self, storage: &StorageTypeTemplate) -> Result<WasmStorageType> {
        Ok(match storage {
            StorageTypeTemplate::Type(ty) => {
                ensure!(
                    ty.comes_from_same_engine(&self.engine),
                    "type is associated with a different engine"
                );
                ty.to_wasm_storage_type()
            }
            StorageTypeTemplate::Ref { nullable, heap } => {
                WasmStorageType::Val(WasmValType::Ref(WasmRefType {
                    nullable: *nullable,
                    heap_type: self.lower_heap(heap)?,
                }))
            }
        })
    }

    fn lower_val(&self, val: &ValTypeTemplate) -> Result<WasmValType> {
        Ok(match val {
            ValTypeTemplate::Type(ty) => {
                ensure!(
                    ty.comes_from_same_engine(&self.engine),
                    "type is associated with a different engine"
                );
                ty.to_wasm_type()
            }
            ValTypeTemplate::Ref { nullable, heap } => WasmValType::Ref(WasmRefType {
                nullable: *nullable,
                heap_type: self.lower_heap(heap)?,
            }),
        })
    }

    fn lower_heap(&self, heap: &HeapTypeTemplate) -> Result<WasmHeapType> {
        Ok(match heap {
            HeapTypeTemplate::Type(ty) => {
                ensure!(
                    ty.comes_from_same_engine(&self.engine),
                    "type is associated with a different engine"
                );
                ty.to_wasm_type()
            }
            HeapTypeTemplate::LocalStruct(id) => {
                self.check_owns_struct(*id);
                WasmHeapType::ConcreteStruct(module_index(id.index))
            }
            HeapTypeTemplate::LocalArray(id) => {
                self.check_owns_array(*id);
                WasmHeapType::ConcreteArray(module_index(id.index))
            }
            HeapTypeTemplate::LocalFunc(id) => {
                self.check_owns_func(*id);
                WasmHeapType::ConcreteFunc(module_index(id.index))
            }
        })
    }
}

/// A registered recursion group of Wasm types.
///
/// Produced by [`RecGroupBuilder::build`]. Use the kind-typed getters
/// ([`struct_`][Self::struct_], [`array`][Self::array], [`func`][Self::func]) to
/// retrieve a member by its label.
///
/// The group's types stay registered with the engine for as long as either this
/// `RecGroup` or any type retrieved from it is alive.
#[derive(Debug)]
pub struct RecGroup {
    builder_id: usize,
    types: Vec<RegisteredType>,
}

impl RecGroup {
    /// The number of types in this rec group.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Whether this rec group is empty.
    ///
    /// This is always `false`, as a rec group must contain at least one type;
    /// it exists for API completeness alongside [`len`][Self::len].
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Get the struct type for the given label.
    ///
    /// # Panics
    ///
    /// Panics if the label did not come from the [`RecGroupBuilder`] that
    /// produced this rec group.
    pub fn struct_(&self, id: PendingStructId) -> StructType {
        assert_eq!(id.builder_id, self.builder_id, "label from another builder");
        StructType::from_registered_type(self.types[id.index as usize].clone())
    }

    /// Get the array type for the given label.
    ///
    /// # Panics
    ///
    /// Panics if the label did not come from the [`RecGroupBuilder`] that
    /// produced this rec group.
    pub fn array(&self, id: PendingArrayId) -> ArrayType {
        assert_eq!(id.builder_id, self.builder_id, "label from another builder");
        ArrayType::from_registered_type(self.types[id.index as usize].clone())
    }

    /// Get the function type for the given label.
    ///
    /// # Panics
    ///
    /// Panics if the label did not come from the [`RecGroupBuilder`] that
    /// produced this rec group.
    pub fn func(&self, id: PendingFuncId) -> FuncType {
        assert_eq!(id.builder_id, self.builder_id, "label from another builder");
        FuncType::from_registered_type(self.types[id.index as usize].clone())
    }

    /// Iterate over all of the types in this rec group, in definition order.
    pub fn types(&self) -> impl ExactSizeIterator<Item = CompositeType> + '_ {
        self.types.iter().map(|rt| {
            let rt = rt.clone();
            if rt.is_struct() {
                CompositeType::Struct(StructType::from_registered_type(rt))
            } else if rt.is_array() {
                CompositeType::Array(ArrayType::from_registered_type(rt))
            } else {
                debug_assert!(rt.is_func());
                CompositeType::Func(FuncType::from_registered_type(rt))
            }
        })
    }

    /// Get the registered struct type at the given member index, for validation.
    fn struct_at(&self, index: usize) -> StructType {
        StructType::from_registered_type(self.types[index].clone())
    }
    fn array_at(&self, index: usize) -> ArrayType {
        ArrayType::from_registered_type(self.types[index].clone())
    }
    fn func_at(&self, index: usize) -> FuncType {
        FuncType::from_registered_type(self.types[index].clone())
    }

    /// Validate that the member at `index` structurally matches its declared
    /// supertype, now that all sibling references are registered and resolvable.
    fn validate_supertype(&self, index: usize, def: &MemberDef) -> Result<()> {
        match def {
            MemberDef::Struct {
                supertype: Some(supertype),
                ..
            } => {
                let sub = self.struct_at(index);
                let sup = match supertype {
                    StructSuperType::Local(id) => self.struct_at(id.index as usize),
                    StructSuperType::Type(ty) => ty.clone(),
                };
                ensure!(
                    struct_fields_match(&sub, &sup),
                    "struct type {index} does not match its supertype: \
                     found {sub}, expected supertype {sup}",
                );
            }
            MemberDef::Array {
                supertype: Some(supertype),
                ..
            } => {
                let sub = self.array_at(index);
                let sup = match supertype {
                    ArraySuperType::Local(id) => self.array_at(id.index as usize),
                    ArraySuperType::Type(ty) => ty.clone(),
                };
                ensure!(
                    sub.field_type().matches(&sup.field_type()),
                    "array type {index} does not match its supertype: \
                     found {sub}, expected supertype {sup}",
                );
            }
            MemberDef::Func {
                supertype: Some(supertype),
                ..
            } => {
                let sub = self.func_at(index);
                let sup = match supertype {
                    FuncSuperType::Local(id) => self.func_at(id.index as usize),
                    FuncSuperType::Type(ty) => ty.clone(),
                };
                // `FuncType::matches` performs structural (not nominal) matching
                // for distinct types, which is exactly the subtype check.
                ensure!(
                    sub.matches(&sup),
                    "function type {index} does not match its supertype: \
                     found {sub}, expected supertype {sup}",
                );
            }
            // No supertype: nothing to validate.
            MemberDef::Struct { .. } | MemberDef::Array { .. } | MemberDef::Func { .. } => {}
        }
        Ok(())
    }
}

/// Does struct `sub` structurally match (i.e. subtype) struct `sup`?
///
/// Mirrors `StructType::fields_match`: `sub` must have at least as many fields
/// as `sup`, and each of `sup`'s fields must be matched by `sub`'s.
fn struct_fields_match(sub: &StructType, sup: &StructType) -> bool {
    sub.fields().len() >= sup.fields().len()
        && sub.fields().zip(sup.fields()).all(|(a, b)| a.matches(&b))
}
