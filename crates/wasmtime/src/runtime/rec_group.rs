//! Embedder API for defining recursion groups of Wasm types.
//!
//! The one-off constructors `StructType::new`, `ArrayType::new`, and
//! `FuncType::new` cannot describe types that reference themselves or each
//! other, because the constructors require all referenced types to already
//! exist. [`RecGroupBuilder`] lifts that restriction: you *declare* a type to
//! get a [`PendingType`] handle, use that handle as a forward reference while
//! defining other types, and the whole group is validated and registered
//! together when [`RecGroupBuilder::build`] is called.
//!
//! Already-registered types (and abstract heap types) are used directly via the
//! normal [`FieldType`]/[`ValType`] APIs; the `forward_ref_*` builder methods
//! are only for references to other types being defined in the same group.
//!
//! ```
//! # use wasmtime::*;
//! # fn main() -> Result<()> {
//! let engine = Engine::default();
//!
//! // Two mutually-recursive struct types.
//! let mut builder = RecGroupBuilder::new(&engine);
//! let a = builder.declare();
//! let b = builder.declare();
//! // forward_ref_field(mutability, is_nullable, target)
//! builder.define_struct(a).forward_ref_field(Mutability::Const, true, b);
//! builder.define_struct(b).forward_ref_field(Mutability::Const, false, a);
//! let group = builder.build()?;
//!
//! let a: StructType = group.get_struct(a).unwrap();
//! let b: StructType = group.get_struct(b).unwrap();
//! assert!(a.field(0).unwrap().element_type().is_val_type());
//! # Ok(())
//! # }
//! ```

use crate::prelude::*;
use crate::type_registry::RegisteredType;
use crate::{ArrayType, Engine, FieldType, Finality, FuncType, Mutability, StructType, ValType};
use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use wasmtime_environ::{
    EngineOrModuleTypeIndex, EntityRef, ModuleInternedTypeIndex, WasmArrayType,
    WasmCompositeInnerType, WasmCompositeType, WasmFieldType, WasmFuncType, WasmHeapType,
    WasmRefType, WasmStorageType, WasmStructType, WasmSubType, WasmValType,
};

/// Maximum number of fields in a struct, mirroring `StructType::from_wasm_struct_type`.
const MAX_FIELDS: usize = 10_000;

/// A process-global counter used to give each [`RecGroupBuilder`] a distinct id
/// so that handles from one builder cannot be accidentally used with another.
static NEXT_BUILDER_ID: AtomicUsize = AtomicUsize::new(0);

fn next_builder_id() -> usize {
    NEXT_BUILDER_ID.fetch_add(1, Relaxed)
}

/// The 0-based index of a member within the rec group being built, expressed as
/// the module-level type reference that `register_rec_group` expects for
/// intra-group references.
fn module_index(index: u32) -> EngineOrModuleTypeIndex {
    EngineOrModuleTypeIndex::Module(ModuleInternedTypeIndex::new(index as usize))
}

/// A handle to a type being defined in a [`RecGroupBuilder`].
///
/// Obtained from [`RecGroupBuilder::declare`]. It is used both to define the
/// type (via [`RecGroupBuilder::define_struct`] and friends) and to
/// forward-reference it from other types in the same group (via the
/// `forward_ref_*` builder methods).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PendingType {
    builder_id: usize,
    index: u32,
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

/// A struct field or array element being defined: either an already-known
/// concrete/abstract type, or a forward reference to a sibling in this group.
enum FieldDef {
    Concrete(FieldType),
    Forward {
        target: u32,
        nullable: bool,
        mutable: bool,
    },
}

/// A function parameter or result being defined: either an already-known
/// concrete/abstract type, or a forward reference to a sibling in this group.
enum ValDef {
    Concrete(ValType),
    Forward { target: u32, nullable: bool },
}

/// A supertype being defined: either a sibling in this group or an
/// already-registered concrete type. Generic over the concrete kind.
enum SuperDef<T> {
    Forward(u32),
    Known(T),
}

/// The in-progress definition of one member of a rec group.
enum MemberDef {
    Struct {
        finality: Finality,
        supertype: Option<SuperDef<StructType>>,
        fields: Vec<FieldDef>,
    },
    Array {
        finality: Finality,
        supertype: Option<SuperDef<ArrayType>>,
        element: Option<FieldDef>,
    },
    Func {
        finality: Finality,
        supertype: Option<SuperDef<FuncType>>,
        params: Vec<ValDef>,
        results: Vec<ValDef>,
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

    /// Declare a new type in this rec group, returning a handle that can be used
    /// as a forward reference before the type is defined.
    pub fn declare(&mut self) -> PendingType {
        let index = u32::try_from(self.members.len()).expect("too many types in a rec group");
        self.members.push(None);
        PendingType {
            builder_id: self.builder_id,
            index,
        }
    }

    #[track_caller]
    fn check_owns(&self, ty: PendingType) {
        assert_eq!(
            ty.builder_id, self.builder_id,
            "`PendingType` used with a different `RecGroupBuilder` than it came from"
        );
    }

    /// Begin defining the given handle as a struct type.
    ///
    /// Any previous definition of this handle is discarded.
    #[track_caller]
    pub fn define_struct(&mut self, ty: PendingType) -> StructTypeBuilder<'_> {
        self.check_owns(ty);
        self.members[ty.index as usize] = Some(MemberDef::Struct {
            finality: Finality::Final,
            supertype: None,
            fields: Vec::new(),
        });
        StructTypeBuilder {
            rec: self,
            index: ty.index,
        }
    }

    /// Begin defining the given handle as an array type.
    ///
    /// Any previous definition of this handle is discarded.
    #[track_caller]
    pub fn define_array(&mut self, ty: PendingType) -> ArrayTypeBuilder<'_> {
        self.check_owns(ty);
        self.members[ty.index as usize] = Some(MemberDef::Array {
            finality: Finality::Final,
            supertype: None,
            element: None,
        });
        ArrayTypeBuilder {
            rec: self,
            index: ty.index,
        }
    }

    /// Begin defining the given handle as a function type.
    ///
    /// Any previous definition of this handle is discarded.
    #[track_caller]
    pub fn define_func(&mut self, ty: PendingType) -> FuncTypeBuilder<'_> {
        self.check_owns(ty);
        self.members[ty.index as usize] = Some(MemberDef::Func {
            finality: Finality::Final,
            supertype: None,
            params: Vec::new(),
            results: Vec::new(),
        });
        FuncTypeBuilder {
            rec: self,
            index: ty.index,
        }
    }

    /// Finish building the rec group: validate all of its types, register them
    /// with the engine, and return the resulting [`RecGroup`].
    ///
    /// Returns an error if the group is empty, if any declared type was never
    /// defined, if an array's element type was never set, if any type references
    /// a type from a different engine, or if a struct exceeds the
    /// implementation's field-count limit.
    pub fn build(self) -> Result<RecGroup> {
        let RecGroupBuilder {
            engine,
            builder_id,
            members,
        } = self;

        ensure!(
            !members.is_empty(),
            "a rec group must contain at least one type"
        );

        for (i, member) in members.iter().enumerate() {
            match member {
                None => bail!("type {i} was declared but never defined"),
                Some(MemberDef::Array { element: None, .. }) => {
                    bail!("array type {i} was declared but its element type was never set")
                }
                Some(_) => {}
            }
        }

        let mut sub_types = Vec::with_capacity(members.len());
        for member in &members {
            sub_types.push(lower_member(&engine, &members, member.as_ref().unwrap())?);
        }

        let registered = engine.register_rec_group_types(sub_types.into_iter())?;
        let group = RecGroup {
            builder_id,
            types: registered,
        };

        // Validate that each type structurally matches its declared supertype,
        // now that forward references resolve to registered types. On failure,
        // `group` is dropped, which unregisters the types.
        for (i, member) in members.iter().enumerate() {
            validate_supertype(&group, i, member.as_ref().unwrap())?;
        }

        Ok(group)
    }
}

/// Builder for a struct type within a [`RecGroupBuilder`].
///
/// Returned by [`RecGroupBuilder::define_struct`].
pub struct StructTypeBuilder<'a> {
    rec: &'a mut RecGroupBuilder,
    index: u32,
}

impl<'a> StructTypeBuilder<'a> {
    fn fields_mut(&mut self) -> &mut Vec<FieldDef> {
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Struct { fields, .. }) => fields,
            _ => unreachable!("struct builder on a non-struct member"),
        }
    }

    /// Set this struct type's finality. Defaults to [`Finality::Final`].
    pub fn finality(&mut self, finality: Finality) -> &mut Self {
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Struct { finality: f, .. }) => *f = finality,
            _ => unreachable!("struct builder on a non-struct member"),
        }
        self
    }

    /// Set this struct type's supertype to an already-registered struct type.
    pub fn supertype(&mut self, supertype: StructType) -> &mut Self {
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Struct { supertype: s, .. }) => *s = Some(SuperDef::Known(supertype)),
            _ => unreachable!("struct builder on a non-struct member"),
        }
        self
    }

    /// Set this struct type's supertype to another struct being defined in the
    /// same rec group.
    #[track_caller]
    pub fn forward_supertype(&mut self, supertype: PendingType) -> &mut Self {
        assert_eq!(
            supertype.builder_id, self.rec.builder_id,
            "`PendingType` used with a different `RecGroupBuilder` than it came from"
        );
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Struct { supertype: s, .. }) => {
                *s = Some(SuperDef::Forward(supertype.index))
            }
            _ => unreachable!("struct builder on a non-struct member"),
        }
        self
    }

    /// Append a field whose type is already known (a scalar, an abstract ref, or
    /// a reference to an already-registered type).
    pub fn field(&mut self, ty: FieldType) -> &mut Self {
        self.fields_mut().push(FieldDef::Concrete(ty));
        self
    }

    /// Append a field that is a reference to another type being defined in the
    /// same rec group, with the given mutability and nullability.
    #[track_caller]
    pub fn forward_ref_field(
        &mut self,
        mutability: Mutability,
        is_nullable: bool,
        ty: PendingType,
    ) -> &mut Self {
        assert_eq!(
            ty.builder_id, self.rec.builder_id,
            "`PendingType` used with a different `RecGroupBuilder` than it came from"
        );
        self.fields_mut().push(FieldDef::Forward {
            target: ty.index,
            nullable: is_nullable,
            mutable: mutability.is_var(),
        });
        self
    }
}

/// Builder for an array type within a [`RecGroupBuilder`].
///
/// Returned by [`RecGroupBuilder::define_array`]. An array has exactly one
/// element type, which must be set via [`element`][Self::element] or
/// [`forward_ref_element`][Self::forward_ref_element].
pub struct ArrayTypeBuilder<'a> {
    rec: &'a mut RecGroupBuilder,
    index: u32,
}

impl<'a> ArrayTypeBuilder<'a> {
    fn set_element(&mut self, def: FieldDef) {
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Array { element, .. }) => *element = Some(def),
            _ => unreachable!("array builder on a non-array member"),
        }
    }

    /// Set this array type's finality. Defaults to [`Finality::Final`].
    pub fn finality(&mut self, finality: Finality) -> &mut Self {
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Array { finality: f, .. }) => *f = finality,
            _ => unreachable!("array builder on a non-array member"),
        }
        self
    }

    /// Set this array type's supertype to an already-registered array type.
    pub fn supertype(&mut self, supertype: ArrayType) -> &mut Self {
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Array { supertype: s, .. }) => *s = Some(SuperDef::Known(supertype)),
            _ => unreachable!("array builder on a non-array member"),
        }
        self
    }

    /// Set this array type's supertype to another array being defined in the
    /// same rec group.
    #[track_caller]
    pub fn forward_supertype(&mut self, supertype: PendingType) -> &mut Self {
        assert_eq!(
            supertype.builder_id, self.rec.builder_id,
            "`PendingType` used with a different `RecGroupBuilder` than it came from"
        );
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Array { supertype: s, .. }) => {
                *s = Some(SuperDef::Forward(supertype.index))
            }
            _ => unreachable!("array builder on a non-array member"),
        }
        self
    }

    /// Set the array's element type to an already-known type.
    pub fn element(&mut self, ty: FieldType) -> &mut Self {
        self.set_element(FieldDef::Concrete(ty));
        self
    }

    /// Set the array's element type to a reference to another type being defined
    /// in the same rec group, with the given mutability and nullability.
    #[track_caller]
    pub fn forward_ref_element(
        &mut self,
        mutability: Mutability,
        is_nullable: bool,
        ty: PendingType,
    ) -> &mut Self {
        assert_eq!(
            ty.builder_id, self.rec.builder_id,
            "`PendingType` used with a different `RecGroupBuilder` than it came from"
        );
        self.set_element(FieldDef::Forward {
            target: ty.index,
            nullable: is_nullable,
            mutable: mutability.is_var(),
        });
        self
    }
}

/// Builder for a function type within a [`RecGroupBuilder`].
///
/// Returned by [`RecGroupBuilder::define_func`].
pub struct FuncTypeBuilder<'a> {
    rec: &'a mut RecGroupBuilder,
    index: u32,
}

impl<'a> FuncTypeBuilder<'a> {
    fn params_mut(&mut self) -> &mut Vec<ValDef> {
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Func { params, .. }) => params,
            _ => unreachable!("func builder on a non-func member"),
        }
    }

    fn results_mut(&mut self) -> &mut Vec<ValDef> {
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Func { results, .. }) => results,
            _ => unreachable!("func builder on a non-func member"),
        }
    }

    /// Set this function type's finality. Defaults to [`Finality::Final`].
    pub fn finality(&mut self, finality: Finality) -> &mut Self {
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Func { finality: f, .. }) => *f = finality,
            _ => unreachable!("func builder on a non-func member"),
        }
        self
    }

    /// Set this function type's supertype to an already-registered function type.
    pub fn supertype(&mut self, supertype: FuncType) -> &mut Self {
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Func { supertype: s, .. }) => *s = Some(SuperDef::Known(supertype)),
            _ => unreachable!("func builder on a non-func member"),
        }
        self
    }

    /// Set this function type's supertype to another function being defined in
    /// the same rec group.
    #[track_caller]
    pub fn forward_supertype(&mut self, supertype: PendingType) -> &mut Self {
        self.check_owns(supertype);
        match self.rec.members[self.index as usize].as_mut() {
            Some(MemberDef::Func { supertype: s, .. }) => {
                *s = Some(SuperDef::Forward(supertype.index))
            }
            _ => unreachable!("func builder on a non-func member"),
        }
        self
    }

    /// Append a parameter whose type is already known.
    pub fn param(&mut self, ty: ValType) -> &mut Self {
        self.params_mut().push(ValDef::Concrete(ty));
        self
    }

    /// Append a result whose type is already known.
    pub fn result(&mut self, ty: ValType) -> &mut Self {
        self.results_mut().push(ValDef::Concrete(ty));
        self
    }

    /// Append a parameter that is a reference to another type being defined in
    /// the same rec group, with the given nullability.
    #[track_caller]
    pub fn forward_ref_param(&mut self, is_nullable: bool, ty: PendingType) -> &mut Self {
        self.check_owns(ty);
        self.params_mut().push(ValDef::Forward {
            target: ty.index,
            nullable: is_nullable,
        });
        self
    }

    /// Append a result that is a reference to another type being defined in the
    /// same rec group, with the given nullability.
    #[track_caller]
    pub fn forward_ref_result(&mut self, is_nullable: bool, ty: PendingType) -> &mut Self {
        self.check_owns(ty);
        self.results_mut().push(ValDef::Forward {
            target: ty.index,
            nullable: is_nullable,
        });
        self
    }

    #[track_caller]
    fn check_owns(&self, ty: PendingType) {
        assert_eq!(
            ty.builder_id, self.rec.builder_id,
            "`PendingType` used with a different `RecGroupBuilder` than it came from"
        );
    }
}

/// A registered recursion group of Wasm types.
///
/// Produced by [`RecGroupBuilder::build`]. Use the getters
/// ([`get_struct`][Self::get_struct], [`get_array`][Self::get_array],
/// [`get_func`][Self::get_func]) to retrieve a member by its handle.
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

    /// Whether this rec group is empty. Always `false`, since a rec group must
    /// contain at least one type; provided for API completeness.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    #[track_caller]
    fn registered(&self, ty: PendingType) -> &RegisteredType {
        assert_eq!(
            ty.builder_id, self.builder_id,
            "`PendingType` used with a different `RecGroup` than it came from"
        );
        &self.types[ty.index as usize]
    }

    fn struct_at(&self, index: usize) -> StructType {
        StructType::from_registered_type(self.types[index].clone())
    }
    fn array_at(&self, index: usize) -> ArrayType {
        ArrayType::from_registered_type(self.types[index].clone())
    }
    fn func_at(&self, index: usize) -> FuncType {
        FuncType::from_registered_type(self.types[index].clone())
    }

    /// Get the struct type for the given handle, or `None` if it was defined as
    /// a different kind of type.
    pub fn get_struct(&self, ty: PendingType) -> Option<StructType> {
        let rt = self.registered(ty);
        rt.is_struct()
            .then(|| StructType::from_registered_type(rt.clone()))
    }

    /// Get the array type for the given handle, or `None` if it was defined as a
    /// different kind of type.
    pub fn get_array(&self, ty: PendingType) -> Option<ArrayType> {
        let rt = self.registered(ty);
        rt.is_array()
            .then(|| ArrayType::from_registered_type(rt.clone()))
    }

    /// Get the function type for the given handle, or `None` if it was defined
    /// as a different kind of type.
    pub fn get_func(&self, ty: PendingType) -> Option<FuncType> {
        let rt = self.registered(ty);
        rt.is_func()
            .then(|| FuncType::from_registered_type(rt.clone()))
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
}

/// The `WasmHeapType` for a forward reference to the member at `target`,
/// choosing the concrete variant based on the target's kind.
fn forward_heap(members: &[Option<MemberDef>], target: u32) -> WasmHeapType {
    match members[target as usize]
        .as_ref()
        .expect("all members are defined before lowering")
    {
        MemberDef::Struct { .. } => WasmHeapType::ConcreteStruct(module_index(target)),
        MemberDef::Array { .. } => WasmHeapType::ConcreteArray(module_index(target)),
        MemberDef::Func { .. } => WasmHeapType::ConcreteFunc(module_index(target)),
    }
}

fn lower_member(
    engine: &Engine,
    members: &[Option<MemberDef>],
    def: &MemberDef,
) -> Result<WasmSubType> {
    let (finality, supertype, inner) = match def {
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
            let supertype = match supertype {
                None => None,
                Some(SuperDef::Forward(t)) => Some(module_index(*t)),
                Some(SuperDef::Known(ty)) => {
                    ensure!(
                        ty.comes_from_same_engine(engine),
                        "supertype is associated with a different engine"
                    );
                    Some(EngineOrModuleTypeIndex::Engine(ty.type_index()))
                }
            };
            let fields = fields
                .iter()
                .map(|f| lower_field(engine, members, f))
                .collect::<Result<Vec<_>>>()?;
            (
                *finality,
                supertype,
                WasmCompositeInnerType::Struct(WasmStructType {
                    fields: fields.into(),
                }),
            )
        }
        MemberDef::Array {
            finality,
            supertype,
            element,
        } => {
            let supertype = match supertype {
                None => None,
                Some(SuperDef::Forward(t)) => Some(module_index(*t)),
                Some(SuperDef::Known(ty)) => {
                    ensure!(
                        ty.comes_from_same_engine(engine),
                        "supertype is associated with a different engine"
                    );
                    Some(EngineOrModuleTypeIndex::Engine(ty.type_index()))
                }
            };
            let field = lower_field(engine, members, element.as_ref().unwrap())?;
            (
                *finality,
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
            let supertype = match supertype {
                None => None,
                Some(SuperDef::Forward(t)) => Some(module_index(*t)),
                Some(SuperDef::Known(ty)) => {
                    ensure!(
                        ty.comes_from_same_engine(engine),
                        "supertype is associated with a different engine"
                    );
                    Some(EngineOrModuleTypeIndex::Engine(ty.type_index()))
                }
            };
            let params = params
                .iter()
                .map(|p| lower_val(engine, members, p))
                .collect::<Result<Vec<_>>>()?;
            let results = results
                .iter()
                .map(|r| lower_val(engine, members, r))
                .collect::<Result<Vec<_>>>()?;
            (
                *finality,
                supertype,
                WasmCompositeInnerType::Func(WasmFuncType::new(params, results)?),
            )
        }
    };

    Ok(WasmSubType {
        is_final: finality.is_final(),
        supertype,
        composite_type: WasmCompositeType {
            shared: false,
            inner,
        },
    })
}

fn lower_field(
    engine: &Engine,
    members: &[Option<MemberDef>],
    field: &FieldDef,
) -> Result<WasmFieldType> {
    Ok(match field {
        FieldDef::Concrete(ty) => {
            ensure!(
                ty.comes_from_same_engine(engine),
                "field type is associated with a different engine"
            );
            ty.to_wasm_field_type()
        }
        FieldDef::Forward {
            target,
            nullable,
            mutable,
        } => WasmFieldType {
            element_type: WasmStorageType::Val(WasmValType::Ref(WasmRefType {
                nullable: *nullable,
                heap_type: forward_heap(members, *target),
            })),
            mutable: *mutable,
        },
    })
}

fn lower_val(engine: &Engine, members: &[Option<MemberDef>], val: &ValDef) -> Result<WasmValType> {
    Ok(match val {
        ValDef::Concrete(ty) => {
            ensure!(
                ty.comes_from_same_engine(engine),
                "type is associated with a different engine"
            );
            ty.to_wasm_type()
        }
        ValDef::Forward { target, nullable } => WasmValType::Ref(WasmRefType {
            nullable: *nullable,
            heap_type: forward_heap(members, *target),
        }),
    })
}

/// Validate that the member at `index` structurally matches its declared
/// supertype (if any), now that all forward references are registered.
fn validate_supertype(group: &RecGroup, index: usize, def: &MemberDef) -> Result<()> {
    match def {
        MemberDef::Struct {
            supertype: Some(supertype),
            ..
        } => {
            let sub = group.struct_at(index);
            let sup = match supertype {
                SuperDef::Forward(t) => {
                    let t = *t as usize;
                    ensure!(
                        group.types[t].is_struct(),
                        "a struct type's supertype must be a struct type"
                    );
                    group.struct_at(t)
                }
                SuperDef::Known(ty) => ty.clone(),
            };
            ensure!(
                sup.finality().is_non_final(),
                "cannot create a subtype of a final supertype"
            );
            ensure!(
                struct_fields_match(&sub, &sup),
                "struct fields must match their supertype's fields"
            );
        }
        MemberDef::Array {
            supertype: Some(supertype),
            ..
        } => {
            let sub = group.array_at(index);
            let sup = match supertype {
                SuperDef::Forward(t) => {
                    let t = *t as usize;
                    ensure!(
                        group.types[t].is_array(),
                        "an array type's supertype must be an array type"
                    );
                    group.array_at(t)
                }
                SuperDef::Known(ty) => ty.clone(),
            };
            ensure!(
                sup.finality().is_non_final(),
                "cannot create a subtype of a final supertype"
            );
            ensure!(
                sub.field_type().matches(&sup.field_type()),
                "array field type must match its supertype's field type"
            );
        }
        MemberDef::Func {
            supertype: Some(supertype),
            ..
        } => {
            let sub = group.func_at(index);
            let sup = match supertype {
                SuperDef::Forward(t) => {
                    let t = *t as usize;
                    ensure!(
                        group.types[t].is_func(),
                        "a function type's supertype must be a function type"
                    );
                    group.func_at(t)
                }
                SuperDef::Known(ty) => ty.clone(),
            };
            ensure!(
                sup.finality().is_non_final(),
                "cannot create a subtype of a final supertype"
            );
            // `FuncType::matches` performs structural (not nominal) matching for
            // distinct types, which is exactly the subtype check we want.
            ensure!(sub.matches(&sup), "function type must match its supertype");
        }
        // No supertype: nothing to validate.
        MemberDef::Struct { .. } | MemberDef::Array { .. } | MemberDef::Func { .. } => {}
    }
    Ok(())
}

/// Does struct `sub` structurally match (i.e. subtype) struct `sup`? `sub` must
/// have at least as many fields as `sup`, and each of `sup`'s fields must be
/// matched by `sub`'s.
fn struct_fields_match(sub: &StructType, sup: &StructType) -> bool {
    sub.fields().len() >= sup.fields().len()
        && sub.fields().zip(sup.fields()).all(|(a, b)| a.matches(&b))
}
