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
//! Each type is defined via a builder (e.g. [`RecGroupBuilder::define_struct`])
//! and committed to the group by calling `finish` on that builder; a definition
//! that is never finished is treated as though the type was never defined.
//!
//! Already-registered types (and abstract heap types) are used directly via the
//! normal [`FieldType`]/[`ValType`] APIs; the `forward_ref_*` builder methods
//! are only for references to other types being defined in the same group.
//!
//! The order in which types are declared is significant; see
//! [`RecGroupBuilder`'s docs][RecGroupBuilder#declaration-order-is-significant].
//!
//! ```
//! # use wasmtime::*;
//! # fn main() -> Result<()> {
//! let engine = Engine::default();
//!
//! // Two mutually-recursive struct types.
//! let mut builder = RecGroupBuilder::new(&engine);
//! let a = builder.declare_struct();
//! let b = builder.declare_struct();
//! builder.define_struct(a).forward_ref_field(b).nullable(true).finish().finish();
//! builder.define_struct(b).forward_ref_field(a).nullable(false).finish().finish();
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
use crate::{
    ArrayType, Engine, FieldType, Finality, FuncType, HeapType, Mutability, StructType, ValType,
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

/// The composite kind a member was declared as, carried by [`PendingType`] so
/// that forward references can be lowered into the correct `WasmHeapType`
/// variant before the referenced member's body is defined.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum MemberKind {
    Struct,
    Array,
    Func,
}

/// A handle to a type being defined in a [`RecGroupBuilder`].
///
/// Obtained from [`RecGroupBuilder::declare_struct`] and friends. It is used
/// both to define the type (via [`RecGroupBuilder::define_struct`] and friends)
/// and to forward-reference it from other types in the same group (via the
/// `forward_ref_*` builder methods). It records the kind it was declared as, so
/// that a forward reference can be lowered into the correct `WasmHeapType`
/// variant.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PendingType {
    builder_id: usize,
    index: u32,
    kind: MemberKind,
}

/// A builder for defining a recursion group of Wasm types, including types that
/// reference themselves or each other.
///
/// See the [module-level documentation](crate::RecGroupBuilder) for an overview
/// and examples.
///
/// # Declaration order is significant
///
/// The order in which types are *declared* (via
/// [`declare_struct`][Self::declare_struct] and friends) fixes their order
/// within the rec group, and that order is semantically visible: two rec groups
/// containing the same types in a different order are *distinct* types, as are
/// their corresponding members. For example, `$f != $f'` and `$s != $s'` in the
/// following, even though each group defines one `func` and one `struct`:
///
/// ```wat
/// (rec (type $f (func))
///      (type $s (struct)))
///
/// (rec (type $s' (struct))
///      (type $f' (func)))
/// ```
///
/// It is the order of the `declare_*` calls that determines this, not the order
/// in which the types are subsequently defined and finished.
pub struct RecGroupBuilder {
    engine: Engine,
    builder_id: usize,
    /// The first error encountered while adding types to the builder (for
    /// example, referencing a type from a different engine). Surfaced by
    /// [`build`][Self::build]; the chaining builder methods stay infallible.
    error: Option<Error>,
    /// The finished definition of each member, or `None` if it was declared but
    /// its builder's `finish` was never called.
    members: Vec<Option<WasmSubType>>,
}

impl RecGroupBuilder {
    /// Create a new, empty rec group builder associated with the given engine.
    pub fn new(engine: &Engine) -> Self {
        RecGroupBuilder {
            engine: engine.clone(),
            builder_id: next_builder_id(),
            error: None,
            members: Vec::new(),
        }
    }

    fn declare(&mut self, kind: MemberKind) -> PendingType {
        let index = u32::try_from(self.members.len()).expect("too many types in a rec group");
        self.members.push(None);
        PendingType {
            builder_id: self.builder_id,
            index,
            kind,
        }
    }

    /// Declare a new struct type in this rec group, returning a handle that can
    /// be used as a forward reference before the type is defined.
    ///
    /// The order of `declare_*` calls fixes the types' order within the rec
    /// group, which is semantically significant; see the [module
    /// documentation](crate::RecGroupBuilder#declaration-order-is-significant)
    /// for details.
    pub fn declare_struct(&mut self) -> PendingType {
        self.declare(MemberKind::Struct)
    }

    /// Declare a new array type in this rec group, returning a handle that can
    /// be used as a forward reference before the type is defined.
    ///
    /// The order of `declare_*` calls fixes the types' order within the rec
    /// group, which is semantically significant; see the [module
    /// documentation](crate::RecGroupBuilder#declaration-order-is-significant)
    /// for details.
    pub fn declare_array(&mut self) -> PendingType {
        self.declare(MemberKind::Array)
    }

    /// Declare a new function type in this rec group, returning a handle that
    /// can be used as a forward reference before the type is defined.
    ///
    /// The order of `declare_*` calls fixes the types' order within the rec
    /// group, which is semantically significant; see the [module
    /// documentation](crate::RecGroupBuilder#declaration-order-is-significant)
    /// for details.
    pub fn declare_func(&mut self) -> PendingType {
        self.declare(MemberKind::Func)
    }

    #[track_caller]
    fn check_owns(&self, ty: PendingType) {
        assert_eq!(
            ty.builder_id, self.builder_id,
            "`PendingType` used with a different `RecGroupBuilder` than it came from"
        );
    }

    /// Record an error to be surfaced by [`build`][Self::build]. Only the first
    /// error is kept, so the chaining builder methods can stay infallible.
    fn record_error(&mut self, error: Error) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    fn check_engine(&mut self, same_engine: bool, what: &str) {
        if !same_engine {
            self.record_error(format_err!("{what} is associated with a different engine"));
        }
    }

    /// Begin defining the given handle as a struct type.
    ///
    /// The definition is committed to the group by calling
    /// [`finish`][StructTypeBuilder::finish]; committing replaces any previous
    /// definition of this handle.
    ///
    /// # Panics
    ///
    /// Panics if the handle was not declared via
    /// [`declare_struct`][Self::declare_struct].
    #[track_caller]
    pub fn define_struct(&mut self, ty: PendingType) -> StructTypeBuilder<'_> {
        self.check_owns(ty);
        assert!(
            matches!(ty.kind, MemberKind::Struct),
            "handle was not declared as a struct type"
        );
        StructTypeBuilder {
            rec: self,
            index: ty.index,
            finality: Finality::Final,
            supertype: None,
            fields: Vec::new(),
        }
    }

    /// Begin defining the given handle as an array type.
    ///
    /// The definition is committed to the group by calling
    /// [`finish`][ArrayTypeBuilder::finish]; committing replaces any previous
    /// definition of this handle.
    ///
    /// # Panics
    ///
    /// Panics if the handle was not declared via
    /// [`declare_array`][Self::declare_array].
    #[track_caller]
    pub fn define_array(&mut self, ty: PendingType) -> ArrayTypeBuilder<'_> {
        self.check_owns(ty);
        assert!(
            matches!(ty.kind, MemberKind::Array),
            "handle was not declared as an array type"
        );
        ArrayTypeBuilder {
            rec: self,
            index: ty.index,
            finality: Finality::Final,
            supertype: None,
            element: None,
        }
    }

    /// Begin defining the given handle as a function type.
    ///
    /// The definition is committed to the group by calling
    /// [`finish`][FuncTypeBuilder::finish]; committing replaces any previous
    /// definition of this handle.
    ///
    /// # Panics
    ///
    /// Panics if the handle was not declared via
    /// [`declare_func`][Self::declare_func].
    #[track_caller]
    pub fn define_func(&mut self, ty: PendingType) -> FuncTypeBuilder<'_> {
        self.check_owns(ty);
        assert!(
            matches!(ty.kind, MemberKind::Func),
            "handle was not declared as a function type"
        );
        FuncTypeBuilder {
            rec: self,
            index: ty.index,
            finality: Finality::Final,
            supertype: None,
            params: Vec::new(),
            results: Vec::new(),
        }
    }

    /// Finish building the rec group: validate all of its types, register them
    /// with the engine, and return the resulting [`RecGroup`].
    ///
    /// An empty group is allowed and produces an empty [`RecGroup`].
    ///
    /// Returns an error if any declared type was never defined (i.e. its
    /// builder's `finish` was never called), if any type references a type from
    /// a different engine, or if a struct exceeds the implementation's
    /// field-count limit.
    pub fn build(self) -> Result<RecGroup> {
        let RecGroupBuilder {
            engine,
            builder_id,
            error,
            members,
        } = self;

        if let Some(error) = error {
            return Err(error);
        }

        let mut sub_types = Vec::with_capacity(members.len());
        for (i, member) in members.into_iter().enumerate() {
            match member {
                Some(sub_type) => sub_types.push(sub_type),
                None => bail!("type {i} was declared but never defined"),
            }
        }

        // Keep each member's declared supertype so we can structurally validate
        // it after registration, once forward references resolve to registered
        // types. (`register_rec_group_types` does not itself check subtyping.)
        let supertypes: Vec<_> = sub_types.iter().map(|s| s.supertype).collect();

        let registered = engine.register_rec_group_types(sub_types.into_iter())?;
        let group = RecGroup {
            builder_id,
            types: registered,
        };

        // On validation failure, `group` is dropped, which unregisters the types.
        for (i, supertype) in supertypes.into_iter().enumerate() {
            validate_supertype(&engine, &group, i, supertype)?;
        }

        Ok(group)
    }
}

/// Builder for a struct type within a [`RecGroupBuilder`].
///
/// Returned by [`RecGroupBuilder::define_struct`]. Call [`finish`][Self::finish]
/// to commit the type to the group.
pub struct StructTypeBuilder<'a> {
    rec: &'a mut RecGroupBuilder,
    index: u32,
    finality: Finality,
    supertype: Option<EngineOrModuleTypeIndex>,
    fields: Vec<WasmFieldType>,
}

impl<'a> StructTypeBuilder<'a> {
    /// Set this struct type's finality. Defaults to [`Finality::Final`].
    pub fn finality(&mut self, finality: Finality) -> &mut Self {
        self.finality = finality;
        self
    }

    /// Set this struct type's supertype to an already-registered struct type.
    pub fn supertype(&mut self, supertype: StructType) -> &mut Self {
        let same = supertype.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "supertype");
        self.supertype = Some(EngineOrModuleTypeIndex::Engine(supertype.type_index()));
        self
    }

    /// Set this struct type's supertype to another struct being defined in the
    /// same rec group.
    #[track_caller]
    pub fn forward_supertype(&mut self, supertype: PendingType) -> &mut Self {
        self.rec.check_owns(supertype);
        self.supertype = Some(module_index(supertype.index));
        self
    }

    /// Append a field whose type is already known (a scalar, an abstract ref, or
    /// a reference to an already-registered type).
    pub fn field(&mut self, ty: FieldType) -> &mut Self {
        let same = ty.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "field type");
        self.fields.push(ty.to_wasm_field_type());
        self
    }

    /// Append a field that is a reference to another type being defined in the
    /// same rec group.
    ///
    /// Returns a builder for configuring the reference; call
    /// [`finish`][ForwardRefFieldBuilder::finish] to commit the field. The field
    /// defaults to immutable and nullable.
    #[track_caller]
    pub fn forward_ref_field(&mut self, ty: PendingType) -> ForwardRefFieldBuilder<'_, 'a> {
        self.rec.check_owns(ty);
        ForwardRefFieldBuilder {
            parent: self,
            target: ty,
            mutability: Mutability::Const,
            nullable: true,
        }
    }

    /// Commit this struct definition to the rec group.
    pub fn finish(&mut self) {
        let index = self.index as usize;
        let fields = core::mem::take(&mut self.fields);
        if fields.len() > MAX_FIELDS {
            self.rec.record_error(format_err!(
                "attempted to define a struct type with {} fields, but that is more than the \
                 maximum supported number of fields ({MAX_FIELDS})",
                fields.len(),
            ));
            return;
        }
        let sub_type = WasmSubType {
            is_final: self.finality.is_final(),
            supertype: self.supertype,
            composite_type: WasmCompositeType {
                shared: false,
                inner: WasmCompositeInnerType::Struct(WasmStructType {
                    fields: fields.into(),
                }),
            },
        };
        self.rec.members[index] = Some(sub_type);
    }
}

/// Builder for a struct field that forward-references another type in the same
/// rec group. Created by [`StructTypeBuilder::forward_ref_field`].
///
/// Configure the reference, then call [`finish`][Self::finish] to commit the
/// field and return to the struct builder.
pub struct ForwardRefFieldBuilder<'p, 'a> {
    parent: &'p mut StructTypeBuilder<'a>,
    target: PendingType,
    mutability: Mutability,
    nullable: bool,
}

impl<'p, 'a> ForwardRefFieldBuilder<'p, 'a> {
    /// Set the field's mutability. Defaults to [`Mutability::Const`].
    pub fn mutability(mut self, mutability: Mutability) -> Self {
        self.mutability = mutability;
        self
    }

    /// Set whether the reference is nullable. Defaults to `true`.
    pub fn nullable(mut self, is_nullable: bool) -> Self {
        self.nullable = is_nullable;
        self
    }

    /// Commit this field and return to the struct builder.
    pub fn finish(self) -> &'p mut StructTypeBuilder<'a> {
        let ForwardRefFieldBuilder {
            parent,
            target,
            mutability,
            nullable,
        } = self;
        parent
            .fields
            .push(forward_field(target, nullable, mutability.is_var()));
        parent
    }
}

/// Builder for an array type within a [`RecGroupBuilder`].
///
/// Returned by [`RecGroupBuilder::define_array`]. An array has exactly one
/// element type, which must be set via [`element`][Self::element] or
/// [`forward_ref_element`][Self::forward_ref_element]. Call
/// [`finish`][Self::finish] to commit the type to the group.
pub struct ArrayTypeBuilder<'a> {
    rec: &'a mut RecGroupBuilder,
    index: u32,
    finality: Finality,
    supertype: Option<EngineOrModuleTypeIndex>,
    element: Option<WasmFieldType>,
}

impl<'a> ArrayTypeBuilder<'a> {
    /// Set this array type's finality. Defaults to [`Finality::Final`].
    pub fn finality(&mut self, finality: Finality) -> &mut Self {
        self.finality = finality;
        self
    }

    /// Set this array type's supertype to an already-registered array type.
    pub fn supertype(&mut self, supertype: ArrayType) -> &mut Self {
        let same = supertype.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "supertype");
        self.supertype = Some(EngineOrModuleTypeIndex::Engine(supertype.type_index()));
        self
    }

    /// Set this array type's supertype to another array being defined in the
    /// same rec group.
    #[track_caller]
    pub fn forward_supertype(&mut self, supertype: PendingType) -> &mut Self {
        self.rec.check_owns(supertype);
        self.supertype = Some(module_index(supertype.index));
        self
    }

    /// Set the array's element type to an already-known type.
    pub fn element(&mut self, ty: FieldType) -> &mut Self {
        let same = ty.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "element type");
        self.element = Some(ty.to_wasm_field_type());
        self
    }

    /// Set the array's element type to a reference to another type being defined
    /// in the same rec group.
    ///
    /// Returns a builder for configuring the reference; call
    /// [`finish`][ForwardRefElementBuilder::finish] to commit the element. The
    /// element defaults to immutable and nullable.
    #[track_caller]
    pub fn forward_ref_element(&mut self, ty: PendingType) -> ForwardRefElementBuilder<'_, 'a> {
        self.rec.check_owns(ty);
        ForwardRefElementBuilder {
            parent: self,
            target: ty,
            mutability: Mutability::Const,
            nullable: true,
        }
    }

    /// Commit this array definition to the rec group.
    pub fn finish(&mut self) {
        let index = self.index as usize;
        let Some(element) = self.element else {
            self.rec.record_error(format_err!(
                "array type {index} was declared but its element type was never set"
            ));
            return;
        };
        let sub_type = WasmSubType {
            is_final: self.finality.is_final(),
            supertype: self.supertype,
            composite_type: WasmCompositeType {
                shared: false,
                inner: WasmCompositeInnerType::Array(WasmArrayType(element)),
            },
        };
        self.rec.members[index] = Some(sub_type);
    }
}

/// Builder for an array element that forward-references another type in the same
/// rec group. Created by [`ArrayTypeBuilder::forward_ref_element`].
///
/// Configure the reference, then call [`finish`][Self::finish] to commit the
/// element and return to the array builder.
pub struct ForwardRefElementBuilder<'p, 'a> {
    parent: &'p mut ArrayTypeBuilder<'a>,
    target: PendingType,
    mutability: Mutability,
    nullable: bool,
}

impl<'p, 'a> ForwardRefElementBuilder<'p, 'a> {
    /// Set the element's mutability. Defaults to [`Mutability::Const`].
    pub fn mutability(mut self, mutability: Mutability) -> Self {
        self.mutability = mutability;
        self
    }

    /// Set whether the reference is nullable. Defaults to `true`.
    pub fn nullable(mut self, is_nullable: bool) -> Self {
        self.nullable = is_nullable;
        self
    }

    /// Commit this element and return to the array builder.
    pub fn finish(self) -> &'p mut ArrayTypeBuilder<'a> {
        let ForwardRefElementBuilder {
            parent,
            target,
            mutability,
            nullable,
        } = self;
        parent.element = Some(forward_field(target, nullable, mutability.is_var()));
        parent
    }
}

/// Builder for a function type within a [`RecGroupBuilder`].
///
/// Returned by [`RecGroupBuilder::define_func`]. Call [`finish`][Self::finish]
/// to commit the type to the group.
pub struct FuncTypeBuilder<'a> {
    rec: &'a mut RecGroupBuilder,
    index: u32,
    finality: Finality,
    supertype: Option<EngineOrModuleTypeIndex>,
    params: Vec<WasmValType>,
    results: Vec<WasmValType>,
}

impl<'a> FuncTypeBuilder<'a> {
    /// Set this function type's finality. Defaults to [`Finality::Final`].
    pub fn finality(&mut self, finality: Finality) -> &mut Self {
        self.finality = finality;
        self
    }

    /// Set this function type's supertype to an already-registered function type.
    pub fn supertype(&mut self, supertype: FuncType) -> &mut Self {
        let same = supertype.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "supertype");
        self.supertype = Some(EngineOrModuleTypeIndex::Engine(supertype.type_index()));
        self
    }

    /// Set this function type's supertype to another function being defined in
    /// the same rec group.
    #[track_caller]
    pub fn forward_supertype(&mut self, supertype: PendingType) -> &mut Self {
        self.rec.check_owns(supertype);
        self.supertype = Some(module_index(supertype.index));
        self
    }

    /// Append a parameter whose type is already known.
    pub fn param(&mut self, ty: ValType) -> &mut Self {
        let same = ty.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "type");
        self.params.push(ty.to_wasm_type());
        self
    }

    /// Append a result whose type is already known.
    pub fn result(&mut self, ty: ValType) -> &mut Self {
        let same = ty.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "type");
        self.results.push(ty.to_wasm_type());
        self
    }

    /// Append a parameter that is a reference to another type being defined in
    /// the same rec group.
    ///
    /// Returns a builder for configuring the reference; call
    /// [`finish`][ForwardRefFuncValBuilder::finish] to commit it. Defaults to
    /// nullable.
    #[track_caller]
    pub fn forward_ref_param(&mut self, ty: PendingType) -> ForwardRefFuncValBuilder<'_, 'a> {
        self.rec.check_owns(ty);
        ForwardRefFuncValBuilder {
            parent: self,
            target: ty,
            nullable: true,
            is_result: false,
        }
    }

    /// Append a result that is a reference to another type being defined in the
    /// same rec group.
    ///
    /// Returns a builder for configuring the reference; call
    /// [`finish`][ForwardRefFuncValBuilder::finish] to commit it. Defaults to
    /// nullable.
    #[track_caller]
    pub fn forward_ref_result(&mut self, ty: PendingType) -> ForwardRefFuncValBuilder<'_, 'a> {
        self.rec.check_owns(ty);
        ForwardRefFuncValBuilder {
            parent: self,
            target: ty,
            nullable: true,
            is_result: true,
        }
    }

    /// Commit this function definition to the rec group.
    pub fn finish(&mut self) {
        let index = self.index as usize;
        let params = core::mem::take(&mut self.params);
        let results = core::mem::take(&mut self.results);
        let func = match WasmFuncType::new(params, results) {
            Ok(func) => func,
            Err(e) => {
                self.rec.record_error(e.into());
                return;
            }
        };
        let sub_type = WasmSubType {
            is_final: self.finality.is_final(),
            supertype: self.supertype,
            composite_type: WasmCompositeType {
                shared: false,
                inner: WasmCompositeInnerType::Func(func),
            },
        };
        self.rec.members[index] = Some(sub_type);
    }
}

/// Builder for a function parameter or result that forward-references another
/// type in the same rec group. Created by
/// [`FuncTypeBuilder::forward_ref_param`] and
/// [`FuncTypeBuilder::forward_ref_result`].
///
/// Configure the reference, then call [`finish`][Self::finish] to commit it and
/// return to the function builder.
pub struct ForwardRefFuncValBuilder<'p, 'a> {
    parent: &'p mut FuncTypeBuilder<'a>,
    target: PendingType,
    nullable: bool,
    is_result: bool,
}

impl<'p, 'a> ForwardRefFuncValBuilder<'p, 'a> {
    /// Set whether the reference is nullable. Defaults to `true`.
    pub fn nullable(mut self, is_nullable: bool) -> Self {
        self.nullable = is_nullable;
        self
    }

    /// Commit this parameter/result and return to the function builder.
    pub fn finish(self) -> &'p mut FuncTypeBuilder<'a> {
        let ForwardRefFuncValBuilder {
            parent,
            target,
            nullable,
            is_result,
        } = self;
        let val = WasmValType::Ref(WasmRefType {
            nullable,
            heap_type: forward_heap(target),
        });
        if is_result {
            parent.results.push(val);
        } else {
            parent.params.push(val);
        }
        parent
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

    /// Whether this rec group is empty, i.e. was built without declaring any
    /// types.
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

    /// Iterate over all of the types in this rec group, in definition order,
    /// each as a concrete [`HeapType`].
    pub fn types(&self) -> impl ExactSizeIterator<Item = HeapType> + '_ {
        self.types.iter().map(|rt| {
            let rt = rt.clone();
            if rt.is_struct() {
                HeapType::ConcreteStruct(StructType::from_registered_type(rt))
            } else if rt.is_array() {
                HeapType::ConcreteArray(ArrayType::from_registered_type(rt))
            } else {
                debug_assert!(rt.is_func());
                HeapType::ConcreteFunc(FuncType::from_registered_type(rt))
            }
        })
    }
}

/// The `WasmHeapType` for a forward reference to the `target` member, choosing
/// the concrete variant based on the target's declared kind.
fn forward_heap(target: PendingType) -> WasmHeapType {
    match target.kind {
        MemberKind::Struct => WasmHeapType::ConcreteStruct(module_index(target.index)),
        MemberKind::Array => WasmHeapType::ConcreteArray(module_index(target.index)),
        MemberKind::Func => WasmHeapType::ConcreteFunc(module_index(target.index)),
    }
}

/// The `WasmFieldType` for a struct field or array element that forward-references
/// the `target` member.
fn forward_field(target: PendingType, nullable: bool, mutable: bool) -> WasmFieldType {
    WasmFieldType {
        element_type: WasmStorageType::Val(WasmValType::Ref(WasmRefType {
            nullable,
            heap_type: forward_heap(target),
        })),
        mutable,
    }
}

/// Validate that the member at `index` structurally matches its declared
/// `supertype` (if any), now that all forward references are registered.
fn validate_supertype(
    engine: &Engine,
    group: &RecGroup,
    index: usize,
    supertype: Option<EngineOrModuleTypeIndex>,
) -> Result<()> {
    let Some(supertype) = supertype else {
        return Ok(());
    };
    let rt = &group.types[index];
    if rt.is_struct() {
        let sub = group.struct_at(index);
        let sup = match supertype {
            EngineOrModuleTypeIndex::Module(t) => {
                let t = t.index();
                ensure!(
                    group.types[t].is_struct(),
                    "a struct type's supertype must be a struct type"
                );
                group.struct_at(t)
            }
            EngineOrModuleTypeIndex::Engine(idx) => StructType::from_shared_type_index(engine, idx),
            EngineOrModuleTypeIndex::RecGroup(_) => unreachable!(),
        };
        ensure!(
            sup.finality().is_non_final(),
            "cannot create a subtype of a final supertype"
        );
        ensure!(
            struct_fields_match(&sub, &sup),
            "struct fields must match their supertype's fields"
        );
    } else if rt.is_array() {
        let sub = group.array_at(index);
        let sup = match supertype {
            EngineOrModuleTypeIndex::Module(t) => {
                let t = t.index();
                ensure!(
                    group.types[t].is_array(),
                    "an array type's supertype must be an array type"
                );
                group.array_at(t)
            }
            EngineOrModuleTypeIndex::Engine(idx) => ArrayType::from_shared_type_index(engine, idx),
            EngineOrModuleTypeIndex::RecGroup(_) => unreachable!(),
        };
        ensure!(
            sup.finality().is_non_final(),
            "cannot create a subtype of a final supertype"
        );
        ensure!(
            sub.field_type().matches(&sup.field_type()),
            "array field type must match its supertype's field type"
        );
    } else {
        debug_assert!(rt.is_func());
        let sub = group.func_at(index);
        let sup = match supertype {
            EngineOrModuleTypeIndex::Module(t) => {
                let t = t.index();
                ensure!(
                    group.types[t].is_func(),
                    "a function type's supertype must be a function type"
                );
                group.func_at(t)
            }
            EngineOrModuleTypeIndex::Engine(idx) => FuncType::from_shared_type_index(engine, idx),
            EngineOrModuleTypeIndex::RecGroup(_) => unreachable!(),
        };
        ensure!(
            sup.finality().is_non_final(),
            "cannot create a subtype of a final supertype"
        );
        // `FuncType::matches` performs structural (not nominal) matching for
        // distinct types, which is exactly the subtype check we want.
        ensure!(sub.matches(&sup), "function type must match its supertype");
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
